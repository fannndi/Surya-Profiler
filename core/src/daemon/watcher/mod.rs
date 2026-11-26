//! Foreground watcher + split-screen probe: the daemon spawns `logcat` and
//! `dumpsys` directly.
//!
//! Precision choices:
//! - `-v epoch` timestamps: freshness is epoch arithmetic — no timezone,
//!   no two-digit-year inference, no 1970-style parse bugs.
//! - capture time travels with the event, so a slow pipe cannot reorder
//!   decisions after the fact.
//!
//! The daemon runs as root already, so plain `/system/bin/logcat` works —
//! no per-stream `su` wrapper.
//!
//! Pure parsers live at the bottom and are fixture-tested; the spawn/read
//! plumbing is thin on purpose.

use std::io::BufRead;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::Msg;

use self::parse::is_after_start;

const LOGCAT: &str = "/system/bin/logcat";
/// Dumpsys is the fallback when MIUI's event buffer was cleared or a
/// resume line is older than [MAX_PEEK_AGE_S].
const DUMPSYS: &str = "/system/bin/dumpsys";
/// Events older than this are history, not context (peek only).
pub const MAX_PEEK_AGE_S: f64 = 60.0;

/// logcat path; overridable for host tests (fake-logcat fixtures).
fn logcat_bin() -> String {
    if crate::engine::probe::allow_env_overrides() {
        if let Ok(v) = std::env::var("SURYAPROFILER_LOGCAT_BIN") {
            return v;
        }
    }
    LOGCAT.to_string()
}

fn dumpsys_bin() -> String {
    if crate::engine::probe::allow_env_overrides() {
        if let Ok(v) = std::env::var("SURYAPROFILER_DUMPSYS_BIN") {
            return v;
        }
    }
    DUMPSYS.to_string()
}

/// A live logcat stream with its reader thread.
pub struct Watcher {
    child: Child,
    alive: Arc<AtomicBool>,
}

impl Watcher {
    pub fn is_alive(&mut self) -> bool {
        if !self.alive.load(Ordering::Relaxed) {
            return false;
        }
        if matches!(self.child.try_wait(), Ok(Some(_))) {
            self.alive.store(false, Ordering::Relaxed);
            return false;
        }
        true
    }

    pub fn stop(&mut self) {
        let _ = self.child.kill();
        for _ in 0..50 {
            if self.child.try_wait().ok().flatten().is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        self.alive.store(false, Ordering::Relaxed);
    }
}

/// Spawns the foreground event stream (resume events) and feeds `Msg::Fg`
/// into the daemon loop.
pub fn spawn_fg(tx: Sender<Msg>) -> Option<Watcher> {
    // logcat without `-T` replays its ring buffer before following. Those
    // events are history: replaying them here would resurrect the app that
    // was in front before the service restarted (e.g. YouTube's mapping) and
    // race the startup peek. Only lines logged after this spawn are signals.
    let started = now_epoch();
    let mut child = Command::new(logcat_bin())
        .args([
            "-b",
            "events",
            "-v",
            "epoch",
            "-s",
            "wm_resume_activity:V",
            "wm_set_resumed_activity:V",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let stdout = child.stdout.take()?;
    let alive = Arc::new(AtomicBool::new(true));
    let alive2 = alive.clone();
    std::thread::Builder::new()
        .name("watcher-foreground".into())
        .spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if let Some((epoch, pkg)) = parse_fg_line(&line) {
                    if is_after_start(epoch, started) && is_fresh(epoch, now_epoch()) {
                        let _ = tx.send(Msg::Fg(pkg));
                    }
                }
            }
            alive2.store(false, Ordering::Relaxed);
            let _ = tx.send(Msg::WatcherDown);
        })
        .ok()?;
    Some(Watcher { child, alive })
}

/// One-shot peek of the current foreground package.
/// Blocking — call from a short-lived thread.
///
/// Prefer a fresh `wm_*` event from the logcat ring buffer. MIUI clears or
/// rotates that buffer (and the app may clear it while debugging), so fall
/// back to `dumpsys activity activities`; without this a service start with
/// an empty event buffer applies nothing until the next app switch.
mod parse;
mod peek;

#[cfg(test)]
mod tests;

pub use parse::{is_fresh, now_epoch, parse_fg_line, RESTART_BACKOFF};
pub use peek::{peek_fg, peek_mw};
