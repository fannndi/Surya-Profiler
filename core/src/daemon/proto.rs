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
use crate::daemon::power::{PowerBucket, PowerSample, PowerSnapshot};
use crate::daemon::stats::StatEntry;
use crate::engine::env::EnvSnapshot;
use serde::{Deserialize, Serialize};

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
    /// Power usage snapshot; `watch` also starts/stops live samples.
    Power {
        watch: bool,
        /// Requested history window (clamped by the daemon; default 24 h).
        #[serde(default)]
        window_s: Option<u64>,
    },
    /// Clear power history + all-time peaks (user action).
    PowerClear,
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
    /// Boxed: EnvSnapshot outgrew the other variants (clippy).
    Env { env: Box<EnvSnapshot> },
    /// Power history reply for `cmd: power` (boxed: window + peaks).
    Power { snapshot: Box<PowerSnapshot> },
    /// Live power sample while a Power screen is watching.
    PowerLive {
        sample: Box<PowerSample>,
        #[serde(skip_serializing_if = "Option::is_none")]
        bucket: Option<Box<PowerBucket>>,
    },
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
    /// UFS health (read-only): `life_time_estimation_a` / `eol_info` raw hex.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_life: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_eol: Option<String>,
    /// Newest daemon log lines (ring buffer, oldest first; display only).
    #[serde(default)]
    pub recent_logs: Vec<String>,
}

#[cfg(test)]
mod tests;
