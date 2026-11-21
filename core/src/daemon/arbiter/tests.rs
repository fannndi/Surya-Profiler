//! Arbiter decision table tests.

use super::*;

use crate::daemon::test_util::map;

#[allow(clippy::too_many_arguments)] // wide test matrix; a builder would hide the cases
fn input(
    enabled: bool,
    screen_on: bool,
    locked: bool,
    fg: Option<&str>,
    app_map: &[(&str, &str)],
    base: &str,
    saver: bool,
    mw: bool,
    dynamic: bool,
) -> ArbiterInput {
    ArbiterInput {
        service_enabled: enabled,
        screen_on,
        keyguard_locked: locked,
        foreground_pkg: fg.map(str::to_string),
        app_map: map(app_map),
        base_profile: base.into(),
        sleep_profile: SLEEP_PROFILE.into(),
        saver_on: saver,
        multi_window: mw,
        dynamic_profile: dynamic,
    }
}

fn d(i: &ArbiterInput) -> Decision {
    decide(i)
}

fn applied(profile: &str, reason: &str) -> Decision {
    Decision::Apply {
        profile: profile.into(),
        reason: reason.into(),
    }
}

#[test]
fn service_off_is_none() {
    assert_eq!(
        d(&input(
            false,
            true,
            false,
            Some("com.example.app"),
            &[],
            "balance",
            false,
            false,
            true
        )),
        Decision::None
    );
}

#[test]
fn screen_off_applies_sleep() {
    assert_eq!(
        d(&input(
            true,
            false,
            false,
            Some("com.example.app"),
            &[],
            "balance",
            false,
            false,
            true
        )),
        applied("sleep", "screen off")
    );
}

#[test]
fn keyguard_locked_is_none() {
    assert_eq!(
        d(&input(
            true,
            true,
            true,
            Some("com.example.app"),
            &[],
            "balance",
            false,
            false,
            true
        )),
        Decision::None
    );
}

#[test]
fn mapped_app_applies_mapped_profile() {
    let i = input(
        true,
        true,
        false,
        Some("com.YoStarEN.AzurLane"),
        &[("com.YoStarEN.AzurLane", "game")],
        "balance",
        false,
        false,
        true,
    );
    assert_eq!(d(&i), applied("game", "app"));
}

#[test]
fn unmapped_app_applies_base() {
    let i = input(
        true,
        true,
        false,
        Some("com.whatsapp"),
        &[],
        "powersave",
        false,
        false,
        true,
    );
    assert_eq!(d(&i), applied("powersave", "base"));
}

#[test]
fn launcher_reverts_to_base() {
    let i = input(
        true,
        true,
        false,
        Some("com.miui.home"),
        &[],
        "powersave",
        false,
        false,
        true,
    );
    assert_eq!(d(&i), applied("powersave", "base"));
}

#[test]
fn own_app_reverts_to_base() {
    // Surya Profiler is a normal foreground app without a mapping, so
    // returning to it from YouTube must apply the universal base.
    let i = input(
        true,
        true,
        false,
        Some("com.suryaprofiler"),
        &[("com.google.android.youtube", "powersave")],
        "balance",
        false,
        false,
        true,
    );
    assert_eq!(d(&i), applied("balance", "base"));
}

#[test]
fn system_chrome_is_transient() {
    for pkg in [
        "com.android.systemui",
        "com.lbe.security.miui",
        "com.google.android.inputmethod.latin",
    ] {
        let i = input(
            true,
            true,
            false,
            Some(pkg),
            &[],
            "balance",
            false,
            false,
            true,
        );
        assert_eq!(d(&i), Decision::None, "{pkg}");
    }
}

#[test]
fn decision_foreground_prefers_real_app_over_real_last() {
    assert_eq!(
        decision_foreground(
            Some("com.suryaprofiler"),
            Some("com.google.android.youtube")
        )
        .as_deref(),
        Some("com.suryaprofiler")
    );
}

#[test]
fn decision_foreground_falls_back_only_for_overlays() {
    assert_eq!(
        decision_foreground(
            Some("com.android.systemui"),
            Some("com.google.android.youtube")
        )
        .as_deref(),
        Some("com.google.android.youtube")
    );
    assert_eq!(
        decision_foreground(Some("com.android.systemui"), None),
        None
    );
    assert_eq!(
        decision_foreground(None, Some("com.google.android.youtube")).as_deref(),
        Some("com.google.android.youtube")
    );
}

#[test]
fn settings_is_a_normal_app_now() {
    // The old MIUI-bridge hidden sheet is gone; Settings is a real app.
    assert!(!is_transient(Some("com.android.settings")));
}

#[test]
fn null_foreground_is_none() {
    assert_eq!(
        d(&input(
            true,
            true,
            false,
            None,
            &[],
            "balance",
            false,
            false,
            true
        )),
        Decision::None
    );
}

#[test]
fn transient_predicate_matches_ime_substring() {
    assert!(is_transient(Some("com.sohu.inputmethod.sogou.xiaomi")));
    assert!(is_transient(Some("com.baidu.input_mi")));
    assert!(is_transient(Some("com.iflytek.inputmethod.miui")));
    assert!(is_transient(None));
    assert!(!is_transient(Some("com.miui.home")));
    assert!(!is_transient(Some("com.suryaprofiler")));
}

#[test]
fn saver_on_forces_powersave_base_but_mapping_still_wins() {
    let i = input(
        true,
        true,
        false,
        Some("com.example.app"),
        &[],
        "game",
        true,
        false,
        true,
    );
    assert_eq!(d(&i), applied("powersave", "base"));

    let i = input(
        true,
        true,
        false,
        Some("com.YoStarEN.AzurLane"),
        &[("com.YoStarEN.AzurLane", "game")],
        "balance",
        true,
        false,
        true,
    );
    assert_eq!(d(&i), applied("game", "app"));
}

#[test]
fn multi_window_forces_balance_over_mapping() {
    let i = input(
        true,
        true,
        false,
        Some("com.YoStarEN.AzurLane"),
        &[("com.YoStarEN.AzurLane", "game")],
        "balance",
        false,
        true,
        true,
    );
    assert_eq!(d(&i), applied("balance", "multi-window"));
}

#[test]
fn multi_window_forces_balance_over_saver_base() {
    let i = input(
        true,
        true,
        false,
        Some("com.example.app"),
        &[],
        "balance",
        true,
        true,
        true,
    );
    assert_eq!(d(&i), applied("balance", "multi-window"));
}

#[test]
fn multi_window_does_not_beat_screen_off() {
    let i = input(
        true,
        false,
        false,
        Some("com.example.app"),
        &[],
        "balance",
        false,
        true,
        true,
    );
    assert_eq!(d(&i), applied("sleep", "screen off"));
}

#[test]
fn dynamic_off_mapped_app_falls_back_to_base() {
    let i = input(
        true,
        true,
        false,
        Some("com.YoStarEN.AzurLane"),
        &[("com.YoStarEN.AzurLane", "game")],
        "balance",
        false,
        false,
        false,
    );
    assert_eq!(d(&i), applied("balance", "base"));
}

#[test]
fn dynamic_off_still_forces_powersave_under_saver() {
    let i = input(
        true,
        true,
        false,
        Some("com.example.app"),
        &[],
        "balance",
        true,
        false,
        false,
    );
    assert_eq!(d(&i), applied("powersave", "base"));
}

#[test]
fn dynamic_off_still_applies_sleep() {
    let i = input(
        true,
        false,
        false,
        Some("com.example.app"),
        &[],
        "balance",
        false,
        false,
        false,
    );
    assert_eq!(d(&i), applied("sleep", "screen off"));
}

#[test]
fn dynamic_off_still_forces_balance_on_multi_window() {
    let i = input(
        true,
        true,
        false,
        Some("com.example.app"),
        &[],
        "balance",
        false,
        true,
        false,
    );
    assert_eq!(d(&i), applied("balance", "multi-window"));
}

#[test]
fn dynamic_off_keeps_transient_noop() {
    let i = input(
        true,
        true,
        false,
        Some("com.android.systemui"),
        &[],
        "balance",
        false,
        false,
        false,
    );
    assert_eq!(d(&i), Decision::None);
}

#[test]
fn dynamic_on_mapped_app_applies_mapped_profile() {
    let i = input(
        true,
        true,
        false,
        Some("com.YoStarEN.AzurLane"),
        &[("com.YoStarEN.AzurLane", "game")],
        "balance",
        false,
        false,
        true,
    );
    assert_eq!(d(&i), applied("game", "app"));
}
