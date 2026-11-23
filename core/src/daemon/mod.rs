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
//! - `stats.rs`         transition history (bounded, persisted)
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
mod proto;
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
pub use proto::{Command, Event, Publisher, Snapshot, VERSION};

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
    Env(EnvSnapshot),
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
    /// False until the first apply of this daemon run: the startup decision
    /// is forced so a pack update or drift that happened while we were down
    /// is reconciled.
    reconciled: bool,
    /// Auto-revive watchdog counter (3 s supervise ticks).
    watchdog_tick: u64,
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

/// PID file guard: one daemon/CLI writer per state dir.
pub struct InstanceGuard {
    path: PathBuf,
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        // Only remove the file if it still names this process; a losing
        // starter must never delete the winner's lock.
        let mine = std::process::id().to_string();
        let current = std::fs::read_to_string(&self.path)
            .ok()
            .map(|s| s.trim().to_string());
        if current.as_deref() == Some(mine.as_str()) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

fn pid_file(state_dir: &Path) -> PathBuf {
    state_dir.join("daemon.pid")
}

fn pid_is_live(pid: u32) -> bool {
    if pid == std::process::id() {
        return true;
    }
    let proc_path = format!("/proc/{pid}");
    let proc_dir = Path::new(&proc_path);
    if !proc_dir.exists() {
        return false;
    }
    // Guard against PID reuse by an unrelated process after a hard kill.
    std::fs::read_to_string(proc_dir.join("cmdline"))
        .map(|s| s.contains("surya-prof"))
        .unwrap_or(false)
}

pub fn running_pid(state_dir: &Path) -> Option<u32> {
    let raw = std::fs::read_to_string(pid_file(state_dir)).ok()?;
    let pid = raw.trim().parse::<u32>().ok()?;
    pid_is_live(pid).then_some(pid)
}

pub fn claim_instance(state_dir: &Path) -> Result<InstanceGuard, String> {
    use std::io::Write;
    let _ = std::fs::create_dir_all(state_dir);
    let path = pid_file(state_dir);
    for _ in 0..3 {
        if let Some(pid) = running_pid(state_dir) {
            return Err(format!("another daemon is running (pid {pid})"));
        }
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut f) => {
                writeln!(f, "{}", std::process::id())
                    .map_err(|e| format!("{}: {e}", path.display()))?;
                f.sync_all()
                    .map_err(|e| format!("{}: fsync: {e}", path.display()))?;
                return Ok(InstanceGuard { path });
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // A winner may have created the file but not written the pid
                // yet; give it a moment before treating the lock as stale.
                for _ in 0..5 {
                    if let Some(pid) = running_pid(state_dir) {
                        return Err(format!("another daemon is running (pid {pid})"));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                let _ = std::fs::remove_file(&path);
            }
            Err(e) => return Err(format!("{}: {e}", path.display())),
        }
    }
    Err(format!(
        "could not claim {} (another writer keeps racing)",
        path.display()
    ))
}

pub fn ensure_no_daemon(state_dir: &Path) -> Result<(), String> {
    if let Some(pid) = running_pid(state_dir) {
        Err(format!(
            "daemon is running (pid {pid}); use the app or stop the service first"
        ))
    } else {
        Ok(())
    }
}

mod run;
pub use run::run;

#[cfg(test)]
mod instance_tests {
    use super::*;

    #[test]
    fn instance_lock_rejects_a_second_writer_and_ignores_stale_pid() {
        let dir = super::test_util::tmpdir("daemon-instance");
        std::fs::write(dir.join("daemon.pid"), "999999\n").unwrap();
        let guard = claim_instance(&dir).expect("stale pid must not block");
        assert!(running_pid(&dir).is_some());
        assert!(
            claim_instance(&dir).is_err(),
            "second daemon must be rejected"
        );
        drop(guard);
        assert!(running_pid(&dir).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
