//! Durable power history: append-only JSONL + atomic peaks file.
//!
//! One line per closed minute. No per-line fsync: the history is display
//! data, and a crash may lose at most the in-progress bucket. A corrupt or
//! partial line is skipped on load (never fatal). Old lines are dropped by
//! rewriting the file atomically at most once per day.

use super::model::{PeaksFile, PowerBucket, PowerPeaks, HISTORY_FILE, PEAKS_FILE, PEAKS_SCHEMA};
use crate::logging::{write, Level};
use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

fn history_path(dir: &Path) -> PathBuf {
    dir.join(HISTORY_FILE)
}

fn peaks_path(dir: &Path) -> PathBuf {
    dir.join(PEAKS_FILE)
}

/// Loads and sorts all valid bucket lines. Duplicate `t_ms` keeps the newest
/// line (an interrupted append can leave the same minute twice).
pub fn load_history(dir: &Path) -> Vec<PowerBucket> {
    let path = history_path(dir);
    let Ok(text) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    let mut bad = 0usize;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<PowerBucket>(line) {
            Ok(b) => rows.push(b),
            Err(_) => bad += 1,
        }
    }
    if bad > 0 {
        log(&format!("history: skipped {bad} corrupt line(s)"));
    }
    rows.sort_by_key(|b| b.t_ms);
    let mut out: Vec<PowerBucket> = Vec::with_capacity(rows.len());
    for row in rows {
        if let Some(last) = out.last_mut() {
            if last.t_ms == row.t_ms {
                *last = row;
                continue;
            }
        }
        out.push(row);
    }
    out
}

/// Appends one closed bucket. Best effort: a failed write is logged and the
/// daemon keeps running (the in-memory ring still has the data).
pub fn append(dir: &Path, bucket: &PowerBucket) {
    let Ok(line) = serde_json::to_string(bucket) else {
        return;
    };
    let path = history_path(dir);
    match OpenOptions::new().create(true).append(true).open(&path) {
        Ok(mut f) => {
            if let Err(e) = writeln!(f, "{line}") {
                log(&format!("history append: {e}"));
            }
        }
        Err(e) => log(&format!("history open: {e}")),
    }
}

/// Atomic rewrite (tmp + rename) after retention trimming.
pub fn rewrite(dir: &Path, buckets: &VecDeque<PowerBucket>) {
    let mut body = String::with_capacity(buckets.len() * 180);
    for b in buckets {
        let Ok(line) = serde_json::to_string(b) else {
            continue;
        };
        body.push_str(&line);
        body.push('\n');
    }
    let path = history_path(dir);
    let tmp = path.with_extension("jsonl.tmp");
    if let Err(e) = fs::write(&tmp, body) {
        log(&format!("history rewrite: {e}"));
        return;
    }
    if let Err(e) = fs::rename(&tmp, &path) {
        log(&format!("history rename: {e}"));
    }
}

/// Removes both history and peaks (user Clear).
pub fn clear(dir: &Path) {
    for path in [history_path(dir), peaks_path(dir)] {
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => log(&format!("clear {}: {e}", path.display())),
        }
    }
}

/// Loads all-time peaks; corrupt/missing starts empty (never fatal).
pub fn load_peaks(dir: &Path) -> PowerPeaks {
    let Ok(text) = fs::read_to_string(peaks_path(dir)) else {
        return PowerPeaks::default();
    };
    match serde_json::from_str::<PeaksFile>(&text) {
        Ok(f) if f.schema <= PEAKS_SCHEMA => f.peaks,
        _ => {
            log("peaks: corrupt file ignored");
            PowerPeaks::default()
        }
    }
}

/// Atomic peaks write; only called when a peak actually changed.
pub fn save_peaks(dir: &Path, peaks: &PowerPeaks) {
    let file = PeaksFile {
        schema: PEAKS_SCHEMA,
        peaks: peaks.clone(),
    };
    let Ok(body) = serde_json::to_string(&file) else {
        return;
    };
    let path = peaks_path(dir);
    let tmp = path.with_extension("json.tmp");
    match fs::File::create(&tmp).and_then(|mut f| {
        f.write_all(body.as_bytes())?;
        f.sync_all()
    }) {
        Ok(()) => {
            if let Err(e) = fs::rename(&tmp, &path) {
                log(&format!("peaks rename: {e}"));
            }
        }
        Err(e) => log(&format!("peaks write: {e}")),
    }
}

fn log(msg: &str) {
    write(Level::Warn, "power", msg);
}
