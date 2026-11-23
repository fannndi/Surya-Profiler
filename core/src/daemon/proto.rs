//! Daemon IPC protocol: JSON-lines over stdio.
//!
//! Commands (app -> daemon) and events (daemon -> app) are tagged enums
//! serialized one-per-line. Unknown commands are logged and ignored — a
//! protocol mismatch must never crash the daemon (precision policy).
//!
//! Logging: human-readable lines go to stderr (the app relays them to
//! logcat under the `SuryaProfiler` tag); stdout carries ONLY protocol JSON.

use crate::daemon::bridge::{HoldsInfo, ReleaseReport};
use crate::daemon::config::DaemonConfig;
use crate::daemon::stats::StatEntry;
use crate::engine::env::EnvSnapshot;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::sync::Arc;

/// Protocol version — bumped on breaking changes; the app checks `hello`.
pub const VERSION: u32 = 3;

// --- commands (app -> daemon) ----------------------------------------------

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Command {
    /// Handshake after spawn; daemon answers with a `state` event.
    Hello,
    /// config.json changed on disk — reload + re-evaluate.
    ConfigChanged,
    /// Screen state (broadcast or reconcile tick).
    Screen { on: bool, locked: bool },
    /// Keyguard dismissed (also implies screen on).
    UserPresent,
    /// Raw foreground event forwarded by the app watcher.
    Fg { pkg: String },
    /// Manual card tap: apply now and treat as the universal base.
    SetBase { profile: String },
    /// Service going off: release holds + write the stock snapshot back.
    Restore,
    /// Diagnostics snapshot: daemon health + env + holds.
    Diag,
    /// Transition history (dashboard).
    Stats,
    /// Graceful shutdown (service destroyed on purpose).
    Shutdown,
    /// Liveness check -> `pong`.
    Ping,
}

// --- events (daemon -> app) -------------------------------------------------

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    /// First event after startup.
    Hello { version: u32, pid: u32 },
    /// What the arbiter decided and why (diagnostics + host tests).
    Decision {
        trigger: String,
        action: String, // "none" | "apply"
        #[serde(skip_serializing_if = "Option::is_none")]
        profile: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    /// Profile applied (or confirmed already active). `reason` is raw:
    /// "base" | "screen off" | "app" | "multi-window" | "MIUI saver";
    /// the app resolves labels for "app" via `src_pkg`.
    Applied {
        profile: String,
        reason: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        src_pkg: Option<String>,
        ok: bool,
        /// At least one key was written/verified (or already in sync).
        #[serde(default)]
        applied: bool,
        wrote: usize,
        verified: usize,
        failed: usize,
        ms: u64,
        settle_ms: u64,
    },
    /// Full runtime snapshot (emitted when anything visible changes).
    State { state: Snapshot },
    /// Environment sample (battery / thermal / GPU busy) — emitted on change.
    Env { env: EnvSnapshot },
    /// Diagnostics reply for `cmd: diag` (boxed: it carries the whole config).
    Diag { diag: Box<DiagInfo> },
    /// Transition history reply for `cmd: stats` (plus auto-revive totals).
    Stats {
        entries: Vec<StatEntry>,
        #[serde(skip_serializing_if = "Option::is_none")]
        heals_total: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        heals_last_t: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        heals_last_keys: Option<u32>,
    },
    /// Service-off restore finished.
    Restored {
        ok: bool,
        wrote: usize,
        verified: usize,
        failed: usize,
        #[serde(flatten)]
        release: ReleaseReport,
    },
    /// Ping reply.
    Pong,
    /// Non-fatal error for the log (daemon keeps running).
    Error { msg: String },
    /// Graceful exit.
    Bye,
}

/// Runtime snapshot mirrored into the app's `DynamicProfileState`.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Snapshot {
    pub screen_on: bool,
    pub locked: bool,
    pub multi_window: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub second_window: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreground: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src_pkg: Option<String>,
}

/// Watcher liveness for diagnostics (restart happens on the next tick).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Watchers {
    pub fg: bool,
}

/// Complete diagnostics reply: what the daemon is, sees and holds.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DiagInfo {
    pub version: u32,
    pub pid: u32,
    pub uptime_s: u64,
    pub state_dir: String,
    pub config_path: String,
    pub config: DaemonConfig,
    pub env: EnvSnapshot,
    pub screen_on: bool,
    pub locked: bool,
    pub multi_window: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreground: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub watchers: Watchers,
    pub holds: HoldsInfo,
    pub stats_len: usize,
    /// Newest MIUI `mi_thermald` events (read-only; diagnostic only).
    #[serde(default)]
    pub thermal_events: Vec<crate::engine::thermal_log::ThermalEvent>,
    /// REA 2026-11-23: PowerKeeper applies a −2 °C virtual-sensor
    /// compensation while one of the SP-game packages is foreground —
    /// `Some(true)`/`Some(false)` only when a foreground package is known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub miui_sp_game: Option<bool>,
}

// --- emission ---------------------------------------------------------------

enum OutMsg {
    Line(String),
    Shutdown(std::sync::mpsc::SyncSender<()>),
}

/// Serialized JSON-line writer shared by all threads.
///
/// A dedicated writer thread owns stdout. Producers use `try_send`: if the
/// app stops reading and the bounded queue fills, events are dropped and
/// logged instead of freezing the daemon's decision loop. `shutdown` flushes
/// the queue before process exit.
pub struct Publisher {
    tx: std::sync::mpsc::SyncSender<OutMsg>,
}

impl Publisher {
    pub fn new() -> Arc<Self> {
        let (tx, rx) = std::sync::mpsc::sync_channel::<OutMsg>(256);
        let spawned = std::thread::Builder::new()
            .name("stdout-writer".into())
            .spawn(move || {
                let mut out = std::io::stdout();
                while let Ok(msg) = rx.recv() {
                    match msg {
                        OutMsg::Line(line) => {
                            if writeln!(out, "{line}").is_err() {
                                break;
                            }
                            let _ = out.flush();
                        }
                        OutMsg::Shutdown(ack) => {
                            let _ = out.flush();
                            let _ = ack.send(());
                            break;
                        }
                    }
                }
            });
        if let Err(e) = spawned {
            let _ = writeln!(std::io::stderr(), "publisher: writer thread failed: {e}");
        }
        Arc::new(Publisher { tx })
    }

    pub fn emit(&self, ev: &Event) {
        self.emit_with_wait(ev, 0);
    }

    /// Command replies / restore results must not be silently dropped by the
    /// bounded queue. Wait briefly, then give up (never block forever).
    pub fn emit_reliable(&self, ev: &Event) {
        self.emit_with_wait(ev, 2_000);
    }

    fn emit_with_wait(&self, ev: &Event, wait_ms: u64) {
        let line = match serde_json::to_string(ev) {
            Ok(l) => l,
            Err(e) => {
                self.log(&format!("event serialize failed: {e}"));
                return;
            }
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(wait_ms);
        loop {
            match self.tx.try_send(OutMsg::Line(line.clone())) {
                Ok(()) => return,
                Err(std::sync::mpsc::TrySendError::Full(_)) => {
                    if std::time::Instant::now() >= deadline {
                        self.log("publisher: stdout backpressure; dropped event");
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(std::sync::mpsc::TrySendError::Disconnected(_)) => return,
            }
        }
    }

    /// Flushes queued events and stops the writer thread. Call once at exit.
    pub fn shutdown(&self) {
        let (ack_tx, ack_rx) = std::sync::mpsc::sync_channel(0);
        let mut msg = Some(OutMsg::Shutdown(ack_tx));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            match self
                .tx
                .try_send(msg.take().expect("shutdown message is present"))
            {
                Ok(()) => {
                    let _ = ack_rx.recv_timeout(std::time::Duration::from_secs(2));
                    return;
                }
                Err(std::sync::mpsc::TrySendError::Full(m)) => {
                    if std::time::Instant::now() >= deadline {
                        return;
                    }
                    msg = Some(m);
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(std::sync::mpsc::TrySendError::Disconnected(_)) => return,
            }
        }
    }

    /// Human log line on stderr (relayed to logcat by the app).
    ///
    /// Never use `eprintln!`: the crate is built with `panic = "abort"`, and
    /// an app death closes stderr at the same time as stdin — an EPIPE panic
    /// would abort before the exit restore. A failed log write is ignored.
    pub fn log(&self, msg: &str) {
        let _ = writeln!(std::io::stderr(), "{msg}");
    }
}

#[cfg(test)]
mod tests;
