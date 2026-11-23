//! Command dispatch: IPC commands and watcher/peek messages.
//!
//! Inherent impl for [`Runtime`](super::Runtime); state lives in `mod.rs`,
//! the decision point in `evaluate.rs`.

mod foreground;

use super::arbiter;
use super::proto::{DiagInfo, Watchers};
use super::watcher;
use super::worker::{self, Work};
use super::{Command, Event, Msg, Runtime, SLEEP_DELAY_MS, VERSION};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

impl Runtime {
    pub(super) fn on_command(&mut self, cmd: Command) -> bool {
        match cmd {
            Command::Hello => {
                self.log("ipc: hello");
                self.emit_state(true);
            }
            Command::Ping => self.publisher.emit_reliable(&Event::Pong),
            Command::Shutdown => {
                self.log("ipc: shutdown");
                // Close the gate before the exit restore so a queued bridge
                // sync cannot re-engage a hold while we shut down.
                self.bridge.begin_restore();
                return false;
            }
            Command::ConfigChanged => {
                // explicit hint: force the reload (mtime granularity must
                // never swallow a write)
                if self.config.reload() {
                    self.log("config reloaded");
                }
                if self.config.get().enabled && !self.restoring {
                    // A previously failed stale-hold release or restore leaves
                    // the bridge blocked; a fresh enable must be able to sync.
                    self.bridge.resume();
                }
                // A real foreground app (including Surya Profiler) wins;
                // only a genuine overlay falls back to last_real. This keeps
                // a stale mapped app from overriding the visible app on
                // every app resume (config_changed is sent on each resume).
                let fg = arbiter::decision_foreground(
                    self.last_fg.as_deref(),
                    self.last_real.as_deref(),
                );
                self.evaluate("config", fg);
            }
            Command::Screen { on, locked } => {
                let was_on = self.screen_on;
                self.screen_on = on;
                self.locked = locked;
                if !on {
                    self.fg_debounce = None;
                    if was_on {
                        self.log("screen off");
                    }
                    let sleep_ms = std::env::var("SURYAPROFILER_SLEEP_DELAY_MS")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(SLEEP_DELAY_MS);
                    self.sleep_deadline = Some(Instant::now() + Duration::from_millis(sleep_ms));
                } else {
                    self.sleep_deadline = None;
                    if !was_on {
                        self.log("screen on");
                    }
                    if !self.locked {
                        self.peek_now("unlock");
                    }
                }
                self.emit_state(false);
                // screen state changed: re-sync the MIUI saver gate (it is
                // visibility-gated) without waiting for a decision.
                let cfg = self.config.get().clone();
                if cfg.enabled && !self.restoring {
                    let _ = self.sync_tx.send(self.bridge_ctx(self.active.clone()));
                }
            }
            Command::UserPresent => {
                self.locked = false;
                self.peek_now("unlock");
                self.peek_mw_now();
                self.emit_state(false);
                let cfg = self.config.get().clone();
                if cfg.enabled && !self.restoring {
                    let _ = self.sync_tx.send(self.bridge_ctx(self.active.clone()));
                }
            }
            Command::Fg { pkg } => self.on_fg(pkg),
            Command::SetBase { profile } => {
                // A restore is final: ignore user/automation taps until the
                // app either stops the service or a failed restore resumes.
                if self.restoring {
                    self.log("set_base: rejected while restore is in progress");
                    self.publisher.emit_reliable(&Event::Error {
                        msg: "set_base rejected: restore in progress".into(),
                    });
                    return true;
                }
                if !self.config.get().enabled {
                    self.log("set_base: rejected while service is disabled");
                    self.publisher.emit_reliable(&Event::Error {
                        msg: "set_base rejected: service disabled".into(),
                    });
                    return true;
                }
                // validate against the shipped profile set so a bad/tampered
                // value can never reach the engine (or the device).
                if !super::config::USER_PROFILES.contains(&profile.as_str()) {
                    self.log(&format!("set_base: rejected unknown profile '{profile}'"));
                    self.publisher.emit_reliable(&Event::Error {
                        msg: format!("set_base rejected: unknown profile '{profile}'"),
                    });
                    return true;
                }
                // the app writes config.json first; force-sync, then apply
                self.config.reload();
                self.log(&format!("set_base: {profile}"));
                let job = worker::Job {
                    profile,
                    reason: "base".into(),
                    src_pkg: None,
                    used_saver: false,
                    queued: Instant::now(),
                    // explicit user tap: re-plan even when already active
                    force: true,
                    watchdog: false,
                    sticky: true,
                };
                let _ = self.work_tx.send(Work::Apply(job));
            }
            Command::Restore => {
                // Service off = true stock restore. Close the bridge gate and
                // block decisions immediately; a failed restore resumes.
                self.restoring = true;
                self.bridge.begin_restore();
                self.log("ipc: restore (service off)");
                let _ = self.work_tx.send(Work::Restore);
            }
            Command::Diag => {
                let diag = DiagInfo {
                    version: VERSION,
                    pid: std::process::id(),
                    uptime_s: self.started.elapsed().as_secs(),
                    state_dir: self.state_dir.display().to_string(),
                    config_path: self.config_path.display().to_string(),
                    config: self.config.get().clone(),
                    env: self.env.clone(),
                    screen_on: self.screen_on,
                    locked: self.locked,
                    multi_window: self.multi_window,
                    foreground: self.last_fg.clone(),
                    active: self.active.clone(),
                    reason: self.reason.clone(),
                    watchers: Watchers {
                        fg: self
                            .fg_watcher
                            .as_ref()
                            .map(|w| w.is_alive())
                            .unwrap_or(false),
                    },
                    holds: self.bridge.holds_info(),
                    stats_len: self.stats.entries.len(),
                    thermal_events: crate::engine::thermal_log::load_recent(
                        &crate::engine::env::default_root(),
                        12,
                    ),
                    miui_sp_game: self
                        .last_fg
                        .as_deref()
                        .map(crate::engine::thermal_scenario::is_miui_sp_game),
                };
                self.publisher.emit_reliable(&Event::Diag {
                    diag: Box::new(diag),
                });
            }
            Command::Stats => {
                self.publisher.emit_reliable(&Event::Stats {
                    entries: self.stats.entries.clone(),
                    heals_total: Some(self.stats.heals_total),
                    heals_last_t: Some(self.stats.heals_last_t),
                    heals_last_keys: Some(self.stats.heals_last_keys),
                });
            }
        }
        true
    }
}
