//! Levelled, component-tagged logging shared by the daemon and the CLI.
//!
//! Three sinks in one path:
//! - a bounded in-memory ring buffer (surfaced through the `diag` reply and
//!   `surya-prof log --last N`) — survives even when no file is configured;
//! - a rotating file sink under `<state_dir>/logs/daemon.log` (daemon only);
//! - stderr, mirrored for the app's logcat relay (E2E greps it).
//!
//! Safety: the crate builds with `panic = "abort"` and stderr dies together
//! with the app — every write failure is ignored, nothing here panics, and
//! logging is callable from any thread before/after `init`.

use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Severity order: a line passes when `line_level <= configured_level`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
}

impl Level {
    fn tag(self) -> char {
        match self {
            Level::Error => 'E',
            Level::Warn => 'W',
            Level::Info => 'I',
            Level::Debug => 'D',
        }
    }
}

/// `SURYAPROFILER_LOG` / `log_level` config value → level (default `info`).
pub fn parse_level(s: &str) -> Option<Level> {
    match s.trim().to_ascii_lowercase().as_str() {
        "error" => Some(Level::Error),
        "warn" | "warning" => Some(Level::Warn),
        "info" => Some(Level::Info),
        "debug" | "trace" => Some(Level::Debug),
        _ => None,
    }
}

/// Level from the `SURYAPROFILER_LOG` env var (unset/invalid → `info`).
pub fn level_from_env() -> Level {
    std::env::var("SURYAPROFILER_LOG")
        .ok()
        .and_then(|v| parse_level(&v))
        .unwrap_or(Level::Info)
}

/// Apply a config-provided level override (`log_level` in config.json).
/// Invalid/absent values leave the current level untouched.
pub fn set_level_str(s: Option<&str>) {
    if let Some(level) = s.and_then(parse_level) {
        set_level(level);
    }
}

/// Ring capacity: ~a minute of decision chatter, plenty for a diag reply.
const RING_LINES: usize = 512;
/// File size that triggers `daemon.log` → `daemon.log.1` rotation.
const ROTATE_BYTES: u64 = 512 * 1024;

struct Sink {
    path: PathBuf,
    file: Option<File>,
    written: u64,
    /// Overridable for tests (real builds use [`ROTATE_BYTES`]).
    rotate_at: u64,
}

struct Logger {
    level: AtomicU8,
    ring: Mutex<VecDeque<String>>,
    sink: Mutex<Option<Sink>>,
}

impl Logger {
    fn new() -> Self {
        Logger {
            level: AtomicU8::new(level_from_env() as u8),
            ring: Mutex::new(VecDeque::with_capacity(RING_LINES)),
            sink: Mutex::new(None),
        }
    }

    fn push(&self, line: String) {
        if let Ok(mut ring) = self.ring.lock() {
            ring.push_back(line.clone());
            while ring.len() > RING_LINES {
                ring.pop_front();
            }
        }
        // stderr mirror: the app relays these lines to logcat. A dead
        // reader (app death) turns this into EPIPE — ignore, never panic.
        let _ = writeln!(std::io::stderr(), "{line}");
        if let Ok(mut slot) = self.sink.lock() {
            if let Some(sink) = slot.as_mut() {
                if sink.file.is_none() {
                    sink.file = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&sink.path)
                        .ok();
                    sink.written = sink
                        .file
                        .as_ref()
                        .and_then(|f| f.metadata().ok())
                        .map(|m| m.len())
                        .unwrap_or(0);
                }
                if sink.written >= sink.rotate_at {
                    let gen = sink.path.with_extension("log.1");
                    let _ = std::fs::remove_file(&gen);
                    if std::fs::rename(&sink.path, &gen).is_ok() {
                        sink.file = OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&sink.path)
                            .ok();
                        sink.written = 0;
                    }
                }
                if let Some(f) = sink.file.as_mut() {
                    if writeln!(f, "{line}").is_ok() {
                        sink.written += line.len() as u64 + 1;
                    } else {
                        // transient EIO: reopen lazily on the next line
                        sink.file = None;
                    }
                }
            }
        }
    }
}

static LOGGER: OnceLock<Logger> = OnceLock::new();

fn logger() -> &'static Logger {
    LOGGER.get_or_init(Logger::new)
}

/// Configure the global logger: level now, file sink when `state_dir` is
/// given. Safe to call before/after the first `write` (ring-only start).
pub fn init(state_dir: Option<&Path>, level: Level) {
    let log = logger();
    log.level.store(level as u8, Ordering::Relaxed);
    if let Some(dir) = state_dir {
        let logs = dir.join("logs");
        let _ = std::fs::create_dir_all(&logs);
        if let Ok(mut slot) = log.sink.lock() {
            *slot = Some(Sink {
                path: logs.join("daemon.log"),
                file: None,
                written: 0,
                rotate_at: ROTATE_BYTES,
            });
        }
    }
}

/// Runtime level change (a config reload may lower/raise verbosity).
pub fn set_level(level: Level) {
    logger().level.store(level as u8, Ordering::Relaxed);
}

pub fn enabled(level: Level) -> bool {
    (level as u8) <= logger().level.load(Ordering::Relaxed)
}

/// One log line: `<utc> <tag> [component] message`.
pub fn write(level: Level, component: &str, msg: &str) {
    if !enabled(level) {
        return;
    }
    logger().push(format!(
        "{} {} [{}] {}",
        clock(),
        level.tag(),
        component,
        msg
    ));
}

/// Last `n` lines (oldest first) from the ring buffer.
pub fn recent(n: usize) -> Vec<String> {
    let ring = logger().ring.lock();
    match ring {
        Ok(r) => {
            let skip = r.len().saturating_sub(n);
            r.iter().skip(skip).cloned().collect()
        }
        Err(_) => Vec::new(),
    }
}

/// The daemon's current log file (`<state_dir>/logs/daemon.log`).
pub fn log_path(state_dir: &Path) -> PathBuf {
    state_dir.join("logs").join("daemon.log")
}

/// `YYYY-MM-DD HH:MM:SS.mmm` in UTC (no external date dependency).
fn clock() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let (y, m, d) = civil_from_days((secs / 86_400) as i64);
    let sod = secs % 86_400;
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}.{:03}",
        sod / 3600,
        (sod % 3600) / 60,
        sod % 60,
        now.subsec_millis()
    )
}

/// Days-since-epoch → (year, month, day); Howard Hinnant's civil calendar.
/// January/February belong to the next civil year (the algorithm counts
/// years from March) — hence the `m <= 2` adjustment.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (mp + if mp < 10 { 3 } else { -9 }) as u32;
    (y + i64::from(m <= 2), m, d)
}

#[cfg(test)]
mod tests;
