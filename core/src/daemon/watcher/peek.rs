//! One-shot foreground / split-screen peeks.

use super::*;

pub fn peek_fg() -> Option<String> {
    peek_fg_logcat().or_else(peek_fg_dumpsys)
}

fn peek_fg_logcat() -> Option<String> {
    let out = crate::platform::run(
        &logcat_bin(),
        &[
            "-b",
            "events",
            "-v",
            "epoch",
            "-d",
            "-s",
            "wm_resume_activity:V",
            "wm_set_resumed_activity:V",
        ],
        std::time::Duration::from_secs(3),
    )?;
    if !out.success() {
        return None;
    }
    let text = out.stdout_lossy();
    let now = now_epoch();
    text.lines()
        .filter_map(parse_fg_line)
        .rfind(|(epoch, _)| is_fresh(*epoch, now))
        .map(|(_, pkg)| pkg)
}

/// One-shot split-screen probe: `dumpsys window` lists each visible task
/// with `mode=split-screen-primary|secondary`. MIUI's `ProcessMonitor` stream
/// does not fire for every split transition (verified 2026-11-20), so this is
/// the authoritative source.
///
/// `None` = probe failed; `Some(None)` = no split; `Some(Some((top, bottom)))`
/// = both panes. Blocking, call from a short-lived thread.
pub fn peek_mw() -> Option<Option<(String, String)>> {
    let out = crate::platform::run(
        &dumpsys_bin(),
        &["window"],
        std::time::Duration::from_secs(5),
    )?;
    if !out.success() {
        return None;
    }
    Some(parse_split_panes(&out.stdout_lossy()))
}

/// Parse the compact `dumpsys window` task lines:
/// `* Task{... #211 ... A=10155:com.android.chrome ... visible=true mode=split-screen-primary ...}`
pub fn parse_split_panes(text: &str) -> Option<(String, String)> {
    let mut primary = None;
    let mut secondary = None;
    for line in text.lines() {
        if !line.contains("visible=true") {
            continue;
        }
        let slot = if line.contains("mode=split-screen-primary") {
            &mut primary
        } else if line.contains("mode=split-screen-secondary") {
            &mut secondary
        } else {
            continue;
        };
        if slot.is_none() {
            if let Some(pkg) = task_package(line) {
                *slot = Some(pkg);
            }
        }
    }
    match (primary, secondary) {
        // Same-package split (e.g. two Chrome windows) is still dual-pane.
        (Some(p), Some(s)) => Some((p, s)),
        _ => None,
    }
}

fn task_package(line: &str) -> Option<String> {
    let idx = line.find(" A=")?;
    let rest = &line[idx + " A=".len()..];
    let end = rest.find(' ')?;
    let affinity = &rest[..end];
    let pkg = affinity.split_once(':')?.1;
    let pkg = pkg.strip_suffix(".root").unwrap_or(pkg);
    if pkg.is_empty() {
        None
    } else {
        Some(pkg.to_string())
    }
}

fn peek_fg_dumpsys() -> Option<String> {
    let out = crate::platform::run(
        &dumpsys_bin(),
        &["activity", "activities"],
        std::time::Duration::from_secs(5),
    )?;
    if !out.success() {
        return None;
    }
    parse_dumpsys_fg(&out.stdout_lossy())
}

/// Parse the resumed activity record from `dumpsys activity activities`.
/// Handles both the split-screen `topResumedActivity=` and the normal
/// `mResumedActivity:` shapes on MIUI 14 / Android 12.
pub fn parse_dumpsys_fg(text: &str) -> Option<String> {
    for key in ["topResumedActivity=", "mResumedActivity:"] {
        for line in text.lines().filter(|l| l.contains(key)) {
            if let Some(pkg) = parse_activity_record(line) {
                return Some(pkg);
            }
        }
    }
    None
}

fn parse_activity_record(line: &str) -> Option<String> {
    const PREFIX: &str = "ActivityRecord{";
    let start = line.find(PREFIX)?;
    let rest = &line[start + PREFIX.len()..];
    let end = rest.find('}')?;
    let token = rest[..end].split_whitespace().find(|t| t.contains('/'))?;
    let pkg = token.split('/').next()?.trim();
    if pkg.is_empty() {
        None
    } else {
        Some(pkg.to_string())
    }
}
