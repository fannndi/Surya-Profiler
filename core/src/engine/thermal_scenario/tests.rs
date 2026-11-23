//! Thermal scenario unit tests.

use crate::engine::thermal_scenario::*;

#[test]
fn decodes_base_ids() {
    assert_eq!(decode(0).base, 0);
    assert_eq!(decode(12).base, 12);
    assert_eq!(decode(21).base, 21);
    assert_eq!(base_name(21), Some("video"));
    assert_eq!(base_name(17), Some("camera-4k"));
    assert_eq!(base_name(99), None);
    assert!(!decode(99).known);
}

#[test]
fn decodes_offsets() {
    let s = decode(121);
    assert!(s.known);
    assert_eq!(s.base, 21);
    assert!(s.foreground);
    assert!(!s.performance);

    let s = decode(151); // 21 + 100 + 30
    assert_eq!(s.base, 21);
    assert!(s.foreground && s.performance);

    let s = decode(130); // default + foreground + performance
    assert_eq!(s.base, 0);
    assert!(s.foreground && s.performance);

    let s = decode(521); // game-normal + IEC monitor
    assert_eq!(s.base, 21);
    assert!(s.iec_monitor);

    let s = decode(620); // 500 + 100 + game-normal
    assert_eq!(s.base, 20);
    assert!(s.iec_monitor && s.foreground);
}

#[test]
fn rejects_impossible_combinations() {
    assert!(!decode(26).known);
    // REA 2026-11-23: PowerKeeper's writeThermalId rejects base >= 25.
    assert!(!decode(25).known, "base 25 is unwritable on this build");
    for v in 60..=63 {
        assert!(!decode(v).known, "{v} belongs to SuryaProfiler");
    }
    assert!(!decode(999).known);
    assert_eq!(label(999), "unknown(999)");
}

#[test]
fn sp_game_list_matches_powerkeeper_constants() {
    for pkg in SP_GAME_PACKAGES {
        assert!(is_miui_sp_game(pkg), "{pkg}");
        assert!(pkg.starts_with("com."), "{pkg}");
    }
    assert!(is_miui_sp_game("com.miHoYo.GenshinImpact"));
    assert!(!is_miui_sp_game("com.miHoYo.GenshinImpact.mod"));
    assert!(!is_miui_sp_game("com.tencent.tmgp.pubgmhd"));
    assert!(!is_miui_sp_game("com.suryaprofiler"));
}

#[test]
fn labels_are_readable() {
    assert_eq!(label(0), "default");
    assert_eq!(label(12), "camera");
    assert_eq!(label(121), "video + foreground");
    assert_eq!(label(130), "default + foreground + performance");
}

#[test]
fn yield_policy_matches_the_user_choice() {
    for v in [8, 12, 15, 17, 18, 112, 130, 131, 151] {
        assert!(is_yield(v), "{v} must yield");
    }
    for v in [
        0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 13, 14, 16, 19, 20, 21, 22, 23, 24, 25, 70, 101, 999,
    ] {
        assert!(!is_yield(v), "{v} must keep custom");
    }
    assert!(
        !is_yield(1),
        "base 1 alone is stale when performance mode is off"
    );
}

#[test]
fn default_detection_handles_foreground_offset() {
    assert!(is_default(0));
    assert!(is_default(100));
    assert!(!is_default(130), "performance mode is an override");
    assert!(!is_default(300), "IEC is not default");
    assert!(!is_default(21));
}

#[test]
fn vendor_map_ids_are_the_stock_subset() {
    for v in [0, 1, 2, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18] {
        assert!(has_vendor_map_entry(v), "{v}");
    }
    for v in [20, 21, 70] {
        assert!(!has_vendor_map_entry(v), "{v}");
    }
}
