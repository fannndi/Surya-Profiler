//! The decision point and the supervisor timer.
//!
//! `evaluate` is the single place that turns world state into an arbiter
//! decision; `supervise` runs every 3 s (periodic re-evaluate + watcher
//! health + split-screen probe). Inherent impl for
//! [`Runtime`](super::Runtime).

use super::arbiter::{self, ArbiterInput, Decision, SLEEP_PROFILE};
use super::bridge::SyncCtx;
use super::watcher;
use super::worker::{self, Work};
use super::{Runtime, PERIODIC_TICKS};
use std::time::{Duration, Instant};

/// Neutral package used once at startup when the event buffer gives us no
/// real foreground: applying the base beats applying nothing.
const STARTUP_FALLBACK_PKG: &str = "com.miui.home";

/// Auto-revive cadence: reconcile-drift check; 0 disables the watchdog.
pub(super) fn watchdog_secs() -> u64 {
    std::env::var("SURYAPROFILER_WATCHDOG_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1_800)
}

impl Runtime {
    /// The one decision point. `fg_override` is the foreground chosen by a
    /// config/unlock path: a real app (including Surya Profiler) or the app
    /// underneath a true overlay.
    pub(super) fn evaluate(&mut self, trigger: &str, fg_override: Option<String>) {
        // a real decision supersedes any pending debounced foreground switch
        self.fg_debounce = None;
        if self.restoring {
            return;
        }
        let cfg = self.config.get().clone();
        if !cfg.enabled {
            // the app owns lifecycle: it stops the service (and thus us)
            return;
        }
        // Screen-on decisions need a real foreground. The startup peek or a
        // live event supplies it; until then applying the launcher fallback
        // could drop a mapped game/video profile after daemon restart.
        if self.screen_on && !self.locked && !self.foreground_ready {
            return;
        }
        let mut fg = fg_override.or_else(|| self.last_fg.clone());
        // On a (re)start MIUI may have cleared the events buffer and the
        // foreground may be an overlay. Apply the base once instead of
        // idling until the next real app switch; later overlays keep the
        // old no-op behaviour.
        let fg_known = fg
            .as_deref()
            .is_some_and(|p| !arbiter::is_transient(Some(p)));
        // A service started screen-off applies sleep first; when the screen
        // comes on without a fresh foreground event, that sleep must not be
        // mistaken for a completed startup reconciliation.
        let stale_sleep = self.active.as_deref() == Some(SLEEP_PROFILE);
        if !fg_known && (!self.reconciled || stale_sleep) && self.screen_on && !self.locked {
            fg = self
                .last_real
                .clone()
                .or_else(|| Some(STARTUP_FALLBACK_PKG.to_string()));
        }
        let input = ArbiterInput {
            service_enabled: true,
            screen_on: self.screen_on,
            keyguard_locked: self.locked,
            foreground_pkg: fg.clone(),
            app_map: cfg.effective_app_map(),
            base_profile: cfg.effective_base_profile(),
            sleep_profile: SLEEP_PROFILE.to_string(),
            // attribution: a saver flag WE hold for a mapped app is invisible
            // to the decision — only the user's own saver forces the base
            saver_on: self.bridge.user_saver(self.flags.saver()),
            multi_window: self.multi_window,
            dynamic_profile: cfg.dynamic,
        };
        let mut desired_profile = self.active.clone();
        let mut sync_immediately = true;
        match arbiter::decide(&input) {
            Decision::None => {
                self.log(&format!("evaluate({trigger}): no-op"));
                self.emit_decision(trigger, "none", None, None);
            }
            Decision::Apply { profile, reason } => {
                // Thermal must follow the profile that actually applied, and
                // never run before `ensure_stock` captures the device. Every
                // Apply (including an in-place fast path) is synced from
                // `on_applied`; only no-op decisions sync here.
                sync_immediately = false;
                desired_profile = Some(profile.clone());
                self.log(&format!("evaluate({trigger}): -> {profile} ({reason})"));
                self.emit_decision(
                    trigger,
                    "apply",
                    Some(profile.clone()),
                    Some(reason.clone()),
                );
                // config/pack changes, explicit taps, the first apply of
                // this daemon run and the freq-floor audit must re-plan even
                // when the profile id is unchanged (new keys, drift while we
                // were down)
                let force = matches!(trigger, "config" | "profiles" | "watchdog" | "freq_audit")
                    || !self.reconciled;
                if force {
                    self.log("evaluate: forced re-plan");
                }
                let job = worker::Job {
                    profile,
                    reason,
                    src_pkg: fg,
                    used_saver: input.saver_on,
                    queued: Instant::now(),
                    force,
                    watchdog: trigger == "watchdog" || trigger == "freq_audit",
                    sticky: false,
                };
                let _ = self.work_tx.send(Work::Apply(job));
            }
        }
        // MIUI bridge extras: settings IO on the bridge thread (never blocks
        // the decision loop); serialized there by the bridge Mutex. A new
        // profile waits for `on_applied` so thermal follows reality.
        if sync_immediately {
            let _ = self.sync_tx.send(self.bridge_ctx(desired_profile));
        }
    }

    /// One bridge-sync context from the live world.
    pub(super) fn bridge_ctx(&self, profile: Option<String>) -> SyncCtx {
        let cfg = self.config.get();
        SyncCtx {
            enabled: cfg.enabled,
            last_real: self.last_real.clone(),
            screen_on: self.screen_on,
            locked: self.locked,
            dynamic: cfg.dynamic,
            app_map: cfg.effective_app_map(),
            multi_window: self.multi_window,
            live_saver: self.flags.saver(),
            profile,
            custom_thermal: cfg.custom_thermal,
            rnd_input_suspend: cfg.rnd_input_suspend,
            thread_affinity: cfg.thread_affinity,
            app_downscale: cfg.app_downscale.clone(),
            display_scale: cfg.display_scale.clone(),
        }
    }

    /// 3 s supervisor: periodic re-evaluate + watcher health + fallback peek,
    /// plus the auto-revive watchdog (drift reconciliation at WATCHDOG_SECS).
    pub(super) fn supervise(&mut self, ticks: u64) {
        if self.restoring {
            return;
        }
        // Safety net for split changes without a foreground/task event. A
        // screen-off device cannot be in split screen; skip the dumpsys.
        if self.screen_on && !self.locked && ticks.is_multiple_of(5) {
            self.peek_mw_now();
        }
        // A hung-but-alive logcat stream cannot be detected by EOF alone.
        // A cheap periodic peek keeps foreground changes flowing and also
        // repairs a cleared MIUI event buffer.
        if self.screen_on
            && !self.locked
            && self.last_fg_probe.elapsed() >= Duration::from_millis(super::FG_PROBE_MS)
        {
            self.peek_now("periodic");
        }
        if ticks.is_multiple_of(PERIODIC_TICKS) && self.screen_on && !self.locked {
            self.evaluate("periodic", None);
        }
        // MIUI may rewrite `sconfig` to default shortly after screen-off.
        // Re-assert the sleep thermal profile on the same 15 s cadence; the
        // decision loop itself stays idle while the screen is off.
        if !self.screen_on && self.config.get().enabled && ticks.is_multiple_of(PERIODIC_TICKS) {
            let _ = self.sync_tx.send(self.bridge_ctx(self.active.clone()));
        }
        // Freq floor audit: rate-limits itself to ~15 s; cheap (2 file reads).
        self.freq_audit_tick();
        let wd_ms = watchdog_secs() * 1000;
        if wd_ms > 0 && self.config.get().enabled {
            self.watchdog_tick += 1;
            if self.watchdog_tick * super::SUPERVISE_MS >= wd_ms {
                self.watchdog_tick = 0;
                self.log("watchdog: auto-revive check (reconcile drift)");
                self.evaluate("watchdog", None);
            }
        }
        // foreground stream health
        let fg_alive = self
            .fg_watcher
            .as_mut()
            .map(|w| w.is_alive())
            .unwrap_or(false);
        if !fg_alive && self.last_watcher_restart.elapsed() >= watcher::RESTART_BACKOFF {
            self.last_watcher_restart = Instant::now();
            self.log("foreground stream down — restarting");
            if let Some(w) = self.fg_watcher.as_mut() {
                w.stop();
            }
            self.fg_watcher = watcher::spawn_fg(self.tx.clone());
            // fallback peek while the event stream is down (screen visible)
            if self.screen_on && !self.locked {
                self.peek_now("peek");
            }
        }
    }
}
