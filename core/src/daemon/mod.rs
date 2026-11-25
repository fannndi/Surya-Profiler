//! Daemon: the stdio JSON-lines service behind `surya-prof serve`.
//!
//! Layer map (read in this order):
//! - `proto.rs`         wire format (Command/Event)
//! - `config.rs`        config.json cache (app-owned file)
//! - `arbiter.rs`       pure decision table
//! - `runtime.rs`       main-loop state emission + applied/restored events
//! - `commands.rs`      command dispatch (IPC + watchers)
//! - `evaluate.rs`      decision point + supervisor timer
//! - `worker.rs`        coalescing apply/restore worker (thread)
//! - `engine_driver.rs` engine adapter (in-process, no `su`)
//! - `settings.rs`      live saver flag via the `settings` CLI
//! - `watcher.rs`       logcat foreground stream + dumpsys peek/probe
//! - `env.rs`           environment sampler thread (read-only telemetry)
//! - `power/`           fast battery/charger sampler + power history
//! - `stats.rs`         transition history (bounded, persisted)
//! - `publisher.rs`     stdout JSON-lines writer thread
//! - `instance.rs`      PID-file single-writer guard
//! - `bridge/`          saver follow + R&D input suspend (holds/restore)
//! - `mod.rs`           this file: state type, constants, threads, main loop
//!
//! Lifecycle: spawned by the app's foreground service with the app's stdio
//! bridged. stdin EOF (app death) or `shutdown` ends the process; a clean
//! exit restores the stock snapshot when a profile is still active.

mod arbiter;
mod bridge;
mod commands;
mod config;
mod engine_driver;
mod env;
mod evaluate;
mod freq_audit;
mod instance;
mod power;
mod proto;
mod publisher;
mod runtime;
mod settings;
mod stats;
#[cfg(test)]
mod test_util;
mod watcher;
mod worker;

pub use bridge::clean_holds;
pub use bridge::cleanup_thermal;
pub use bridge::thermal_files_fix;
pub use bridge::thermal_files_scan;
pub use bridge::thermal_inspect;
pub use bridge::ReleaseReport;
pub use freq_audit::{desired_floors, live_floors};
pub use instance::{claim_instance, ensure_no_daemon, running_pid};
pub use proto::{Command, Event, Snapshot, VERSION};
pub use publisher::Publisher;

/// Config shape shared with `doctor` (and future tools).
pub use config::{DaemonConfig, SCHEMA as CONFIG_SCHEMA};

use crate::daemon::bridge::{Bridge, SyncCtx};
use crate::daemon::config::ConfigFile;
use crate::daemon::settings::LiveFlags;
use crate::daemon::stats::Stats;
use crate::daemon::watcher::Watcher;
use crate::daemon::worker::{RestoredEvent, Work};
use crate::engine::apply::Store;
use crate::engine::env::EnvSnapshot;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(crate) const SUPERVISE_MS: u64 = 3_000; // watcher health / periodic cadence
const PERIODIC_TICKS: u64 = 5; // * SUPERVISE_MS = 15 s re-evaluate cadence
const CONFIG_POLL_MS: u64 = 1_000; // mtime fallback (app also pings)
const SLEEP_DELAY_MS: u64 = 10_000; // screen off -> sleep grace period
/// Debounce foreground switches so a rapid burst of app transitions (e.g.
/// launcher flashes between two registered apps) coalesces into one smooth
/// profile switch instead of back-to-back full applies.
const FG_DEBOUNCE_MS: u64 = 300;

/// Messages the main loop consumes.
enum Msg {
    Command(Command),
    Applied(worker::AppliedEvent),
    Restored(RestoredEvent),
    StdinClosed,
    /// Raw foreground event from the daemon's own logcat stream.
    Fg(String),
    /// Multi-window state from the `dumpsys window` probe.
    Mw {
        active: bool,
        other: Option<String>,
        started: Instant,
    },
    /// Wake/unlock seed from the daemon's own peek.
    /// `unlock` always re-evaluates; `peek` (stream-down fallback) only
    /// evaluates when the package actually changed.
    Peek {
        pkg: Option<String>,
        trigger: &'static str,
        started: Instant,
    },
    /// Fresh environment sample (battery / thermal / GPU busy).
    Env(Box<EnvSnapshot>),
    /// Fresh power sample (fast battery/charger read).
    Power(power::PowerTick),
    /// The foreground watcher stream died; the supervisor restarts it.
    WatcherDown,
    /// Worker-owned exit cleanup finished.
    ShutdownDone(worker::ShutdownReport),
}

/// All mutable daemon state — owned by the main loop, no locks.
struct Runtime {
    publisher: Arc<Publisher>,
    work_tx: Sender<Work>,
    /// Bridge sync requests (settings IO happens on the bridge thread).
    sync_tx: Sender<SyncCtx>,
    /// Own sender back (peek/probe threads + watcher restarts).
    tx: Sender<Msg>,
    config: ConfigFile,
    flags: Arc<LiveFlags>,
    bridge: Arc<Bridge>,
    // paths + lifecycle (diag)
    state_dir: PathBuf,
    config_path: PathBuf,
    started: Instant,
    /// Last seen mtime of `state_dir/profiles.json` (app deploy detection).
    profiles_mtime: Option<std::time::SystemTime>,
    // environment + history
    env: EnvSnapshot,
    stats: Stats,
    /// Power usage ring + all-time peaks (read-only telemetry).
    power: power::PowerHistory,
    /// The Power screen is open: forward live samples.
    power_watch: bool,
    /// Screen state shared with the power sampler thread (cadence switch).
    power_screen: Arc<AtomicBool>,
    /// False until the first apply of this daemon run: the startup decision
    /// is forced so a pack update or drift that happened while we were down
    /// is reconciled.
    reconciled: bool,
    /// Auto-revive watchdog counter (3 s supervise ticks).
    watchdog_tick: u64,
    /// Freq floor audit state (external floor drift detection).
    floor_audit: freq_audit::FloorAudit,
    /// True from the service-off restore until exit/failed restore. Blocks
    /// decisions and bridge syncs so nothing re-tunes a restored device.
    restoring: bool,
    // device context
    screen_on: bool,
    locked: bool,
    multi_window: bool,
    second_window: Option<String>,
    /// In-flight `dumpsys window` split probe (never more than one).
    mw_probe: Arc<AtomicBool>,
    /// Deduped raw foreground.
    last_fg: Option<String>,
    /// Last non-transient package.
    last_real: Option<String>,
    /// When the last live foreground event was accepted (stale peek guard).
    last_fg_at: Option<Instant>,
    // decision mirror
    active: Option<String>,
    reason: Option<String>,
    src_pkg: Option<String>,
    // timers
    sleep_deadline: Option<Instant>,
    /// Debounce deadline for a pending foreground-switch evaluation.
    fg_debounce: Option<Instant>,
    // watcher
    fg_watcher: Option<Watcher>,
    last_watcher_restart: Instant,
    // event dedupe
    last_state: Snapshot,
}

/// File mtime (None when missing) — deploy/update detection.
fn mtime(path: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

mod run;
pub use run::run;
