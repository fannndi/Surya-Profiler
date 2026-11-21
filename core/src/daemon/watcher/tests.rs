//! Watcher parser tests.

use super::parse::is_after_start;
use super::peek::{parse_dumpsys_fg, parse_split_panes};
use super::{is_fresh, parse_fg_line};

// Fixtures below are real `logcat -b events -v epoch` shapes from surya.

#[test]
fn parse_resume_activity_line() {
    let line = "1795100618.869  1732  2942 I wm_resume_activity: [0,117144395,177,com.whatsapp/.home.ui.HomeActivity]";
    let (epoch, pkg) = parse_fg_line(line).unwrap();
    assert!((epoch - 1795100618.869).abs() < 0.001);
    assert_eq!(pkg, "com.whatsapp");
}

#[test]
fn parse_set_resumed_activity_line() {
    // Launcher appears only in wm_set_resumed_activity on Android 12.
    let line = "1795100621.031  1732  1767 I wm_set_resumed_activity: [0,com.miui.home/.launcher.Launcher,bringingFoundTaskToFront]";
    let (_, pkg) = parse_fg_line(line).unwrap();
    assert_eq!(pkg, "com.miui.home");
}

#[test]
fn parse_rejects_unrelated_lines() {
    assert!(
        parse_fg_line("1795100618.869  1732  2942 I wm_pause_activity: [0,123,com.x/.Y]").is_none()
    );
    assert!(parse_fg_line("").is_none());
    assert!(parse_fg_line("no-epoch wm_resume_activity: [0,1,2,com.x/.Y]").is_none());
    // Old MIUI 12 tags must not match anymore.
    assert!(
        parse_fg_line("1760022735.123  1234  1234 I am_resume_activity: [0,1,2,com.x/.Y,9]")
            .is_none()
    );
}

#[test]
fn freshness_window_is_epoch_arithmetic() {
    let now = 1_000_000.0;
    assert!(is_fresh(now - 5.0, now));
    assert!(is_fresh(now, now));
    assert!(!is_fresh(now - 61.0, now));
    assert!(!is_fresh(now + 1.0, now), "future stamps are stale");
}

#[test]
fn stream_start_drops_replayed_history() {
    let started = 1_000_000.0;
    assert!(!is_after_start(started - 0.001, started));
    assert!(!is_after_start(started - 30.0, started));
    assert!(is_after_start(started, started));
    assert!(is_after_start(started + 0.5, started));
}

#[test]
fn parse_dumpsys_resumed_activity() {
    // Real surya/MIUI 14 shape.
    let normal =
        "  mResumedActivity: ActivityRecord{9662983 u0 com.suryaprofiler/.MainActivity t208}\n";
    assert_eq!(
        parse_dumpsys_fg(normal).as_deref(),
        Some("com.suryaprofiler")
    );
    // Split screen: topResumedActivity comes first; full class name.
    let split = "    topResumedActivity=ActivityRecord{1234 u0 com.android.chrome/com.google.android.apps.chrome.Main t5}\n    mResumedActivity: ActivityRecord{1234 u0 com.android.chrome/com.google.android.apps.chrome.Main t5}\n";
    assert_eq!(
        parse_dumpsys_fg(split).as_deref(),
        Some("com.android.chrome")
    );
    assert!(parse_dumpsys_fg("no activity here").is_none());
    assert!(parse_dumpsys_fg("mResumedActivity: null").is_none());
}

#[test]
fn parse_split_panes_from_window_dump() {
    let text = "  * Task{d8c45f8 #215 type=standard A=1000:com.android.settings.root U=0 rootTaskId=7 visible=true mode=split-screen-secondary translucent=false sz=1}\n  * Task{3b15b6f #211 type=standard A=10155:com.android.chrome U=0 rootTaskId=6 visible=true mode=split-screen-primary translucent=false sz=1}\n";
    assert_eq!(
        parse_split_panes(text),
        Some(("com.android.chrome".into(), "com.android.settings".into()))
    );
    // A single split root (the other pane empty/transitioning) is not
    // a usable dual-app state.
    let single = "  * Task{3b15b6f #211 type=standard A=10155:com.android.chrome U=0 rootTaskId=6 visible=true mode=split-screen-primary translucent=false sz=1}\n";
    assert_eq!(parse_split_panes(single), None);
    // Same-package split (two Chrome tasks) still counts.
    let same = "  * Task{a #1 A=10155:com.android.chrome U=0 visible=true mode=split-screen-primary}\n  * Task{b #2 A=10155:com.android.chrome.root U=0 visible=true mode=split-screen-secondary}\n";
    assert_eq!(
        parse_split_panes(same),
        Some(("com.android.chrome".into(), "com.android.chrome".into()))
    );
    // Fullscreen tasks never match.
    let full = "  * Task{... A=10155:com.android.chrome ... visible=true mode=fullscreen ...}\n";
    assert_eq!(parse_split_panes(full), None);
}
