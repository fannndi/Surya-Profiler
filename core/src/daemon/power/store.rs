//! Durable power history: append-only JSONL + atomic peaks file.
//!
//! One line per closed minute. No per-line fsync: the history is display
//! data, and a crash may lose at most the in-progress bucket. A corrupt or
//! partial line is skipped on load (never fatal). Old lines are dropped by
//! rewriting the file atomically at most once per day.

use super::aggregate::merge_buckets;
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

/// Loads and sorts all valid bucket lines. Duplicate `t_ms` rows are merged
/// duration-first so an interrupted same-minute close never loses a segment.
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
    let raw_rows = rows.len();
    let mut out: VecDeque<PowerBucket> = VecDeque::with_capacity(rows.len());
    for row in rows {
        if let Some(last) = out.back_mut() {
            if last.t_ms == row.t_ms {
                match (&last.segment_id, &row.segment_id) {
                    // A retried/durable replay of the same segment is not new
                    // duration. Keep one copy; distinct partials are merged.
                    (Some(a), Some(b)) if a == b => *last = row,
                    _ => *last = merge_buckets(last.clone(), row),
                }
                continue;
            }
        }
        out.push_back(row);
    }
    // Heal legacy files that still contain multiple rows for one minute.
    // The in-memory result is already correct; rewrite is best effort.
    if out.len() != raw_rows {
        log(&format!(
            "history: merged {raw_rows} rows into {} unique minutes",
            out.len()
        ));
        let _ = rewrite(dir, &out);
    }
    out.into()
}

/// Appends one closed bucket. Best effort: a failed write is logged and the
/// daemon keeps running (the in-memory ring still has the data).
pub fn append(dir: &Path, bucket: &PowerBucket) -> std::io::Result<()> {
    let line = serde_json::to_string(bucket).map_err(|e| {
        log(&format!("history encode: {e}"));
        std::io::Error::new(std::io::ErrorKind::InvalidData, e)
    })?;
    let path = history_path(dir);
    let result = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| writeln!(f, "{line}"));
    result.map_err(|e| {
        log(&format!("history append: {e}"));
        e
    })
}

/// Atomic rewrite (tmp + rename) after retention trimming or a same-minute
/// merge. Serialization/write/rename failures are returned to the caller.
pub fn rewrite(dir: &Path, buckets: &VecDeque<PowerBucket>) -> std::io::Result<()> {
    let mut body = String::with_capacity(buckets.len() * 180);
    for b in buckets {
        let line = serde_json::to_string(b).map_err(|e| {
            log(&format!("history encode: {e}"));
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })?;
        body.push_str(&line);
        body.push('\n');
    }
    let path = history_path(dir);
    let tmp = path.with_extension("jsonl.tmp");
    fs::write(&tmp, body).map_err(|e| {
        log(&format!("history rewrite: {e}"));
        e
    })?;
    fs::rename(&tmp, &path).map_err(|e| {
        log(&format!("history rename: {e}"));
        e
    })
}

/// Removes history, peaks and both atomic-write temporaries (user Clear).
/// Success means every path is gone; all paths are attempted even on error.
pub fn clear(dir: &Path) -> std::io::Result<()> {
    let mut first_error = None;
    for path in [
        history_path(dir),
        peaks_path(dir),
        history_path(dir).with_extension("jsonl.tmp"),
        peaks_path(dir).with_extension("json.tmp"),
    ] {
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                log(&format!("clear {}: {e}", path.display()));
                first_error.get_or_insert(e);
            }
        }
    }
    first_error.map_or(Ok(()), Err)
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
pub fn save_peaks(dir: &Path, peaks: &PowerPeaks) -> std::io::Result<()> {
    let file = PeaksFile {
        schema: PEAKS_SCHEMA,
        peaks: peaks.clone(),
    };
    let body = serde_json::to_string(&file).map_err(|e| {
        log(&format!("peaks encode: {e}"));
        std::io::Error::new(std::io::ErrorKind::InvalidData, e)
    })?;
    let path = peaks_path(dir);
    let tmp = path.with_extension("json.tmp");
    fs::File::create(&tmp)
        .and_then(|mut f| {
            f.write_all(body.as_bytes())?;
            f.sync_all()
        })
        .map_err(|e| {
            log(&format!("peaks write: {e}"));
            e
        })?;
    fs::rename(&tmp, &path).map_err(|e| {
        log(&format!("peaks rename: {e}"));
        e
    })
}

fn log(msg: &str) {
    write(Level::Warn, "power", msg);
}
