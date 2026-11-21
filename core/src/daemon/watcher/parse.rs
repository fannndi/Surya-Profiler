//! Pure logcat/dumpsys parsers and freshness helpers.

use super::*;

pub fn now_epoch() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// Age check in epoch seconds: 0 <= age <= MAX (future stamps are stale).
pub fn is_fresh(epoch: f64, now: f64) -> bool {
    let age = now - epoch;
    (0.0..=MAX_PEEK_AGE_S).contains(&age)
}

/// Stream-start cutoff: a line older than the spawn time is ring-buffer
/// history, not a live foreground change. The startup peek covers the
/// current package, so dropping history is safe.
pub(crate) fn is_after_start(epoch: f64, started: f64) -> bool {
    epoch >= started
}

/// First whitespace token of a `-v epoch` line -> epoch seconds.
fn line_epoch(line: &str) -> Option<f64> {
    line.split_whitespace().next()?.parse().ok()
}

/// Nth comma-separated field of `rest`, up to the first `/` (pkg/class).
fn tuple_field(rest: &str, n: usize) -> Option<String> {
    let field = rest.split(',').nth(n)?.trim();
    let pkg = field.split('/').next()?.trim();
    if pkg.is_empty() {
        None
    } else {
        Some(pkg.to_string())
    }
}

/// MIUI 14 / Android 12 activity events (the old `am_*` tags are gone):
/// `... wm_resume_activity: [user, token, task, pkg/class]`
/// `... wm_set_resumed_activity: [user, pkg/class, reason]`
pub fn parse_fg_line(line: &str) -> Option<(f64, String)> {
    let epoch = line_epoch(line)?;
    if let Some(idx) = line.find("wm_resume_activity: [") {
        let rest = &line[idx + "wm_resume_activity: [".len()..];
        return tuple_field(rest, 3).map(|pkg| (epoch, pkg));
    }
    if let Some(idx) = line.find("wm_set_resumed_activity: [") {
        let rest = &line[idx + "wm_set_resumed_activity: [".len()..];
        return tuple_field(rest, 1).map(|pkg| (epoch, pkg));
    }
    None
}

/// Restart backoff for a dead stream (matches the old Java-side policy).
pub const RESTART_BACKOFF: Duration = Duration::from_secs(10);
