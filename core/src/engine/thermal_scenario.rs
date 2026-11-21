//! MIUI thermal scenario IDs — decoded from `com.miui.powerkeeper`.
//!
//! `ThermalSwitchHandler` writes `/sys/class/thermal/thermal_message/sconfig`
//! with a base scenario ID (0–25) plus optional offsets:
//!
//! | Offset | Meaning |
//! |---:|---|
//! | `+30` | power-mode performance |
//! | `+100` | foreground/display not folded |
//! | `+200` | IEC-All48 mode |
//! | `+500` | IEC monitor mode |
//!
//! The IDs below are the constants in the ROM's PowerKeeper APK. Keeping this
//! table in the engine lets the daemon and the UI speak MIUI's language while
//! the private SuryaProfiler indices stay far away (60–63).

/// One decoded MIUI thermal scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MiuiScenario {
    /// Raw `sconfig` value.
    pub raw: i32,
    /// Base scenario after subtracting the known offsets (or `raw` if unknown).
    pub base: i32,
    /// True when the raw value matches a base ID or a known offset combination.
    pub known: bool,
    pub performance: bool,
    pub foreground: bool,
    pub iec_all48: bool,
    pub iec_monitor: bool,
}

const OFF_PERFORMANCE: i32 = 30;
const OFF_FOREGROUND: i32 = 100;
const OFF_IEC_ALL48: i32 = 200;
const OFF_IEC_MONITOR: i32 = 500;

/// Base scenario names for `sconfig` 0–25.
///
/// These names come from `ThermalSwitchHandler` in PowerKeeper 4.2.00
/// (`THERMAL_ID_*` constants), not from the stock config map: several IDs
/// intentionally have no vendor config and fall back to normal.
pub fn base_name(base: i32) -> Option<&'static str> {
    Some(match base {
        0 => "default",
        1 => "performance",
        2 => "extreme",
        3 => "huanji",
        4 => "videochat",
        5 => "abnormal",
        6 => "nightvideo",
        7 => "dolbyvision",
        8 => "incall",
        9 => "game",
        10 => "benchmark",
        11 => "class0",
        12 => "camera",
        13 => "pubg",
        14 => "youtube",
        15 => "arvr",
        16 => "game2",
        17 => "camera-4k",
        18 => "camera-8k",
        19 => "navigation",
        20 => "game-normal",
        21 => "video",
        22 => "demo",
        23 => "yuanshen",
        24 => "sptm",
        25 => "max",
        _ => return None,
    })
}

/// Decodes a raw `sconfig` value. Unknown values keep `base = raw` and
/// `known = false`; callers must treat them as MIUI-owned but unclassified.
pub fn decode(raw: i32) -> MiuiScenario {
    if (0..=25).contains(&raw) {
        return MiuiScenario {
            raw,
            base: raw,
            known: true,
            performance: false,
            foreground: false,
            iec_all48: false,
            iec_monitor: false,
        };
    }

    // Try every offset combination. The offsets are small powers-of-ten-like
    // constants, so 16 combinations are cheap and unambiguous here.
    for mask in 1..16u8 {
        let mut sum = 0;
        if mask & 1 != 0 {
            sum += OFF_PERFORMANCE;
        }
        if mask & 2 != 0 {
            sum += OFF_FOREGROUND;
        }
        if mask & 4 != 0 {
            sum += OFF_IEC_ALL48;
        }
        if mask & 8 != 0 {
            sum += OFF_IEC_MONITOR;
        }
        let base = raw - sum;
        if (0..=25).contains(&base) {
            return MiuiScenario {
                raw,
                base,
                known: true,
                performance: mask & 1 != 0,
                foreground: mask & 2 != 0,
                iec_all48: mask & 4 != 0,
                iec_monitor: mask & 8 != 0,
            };
        }
    }

    MiuiScenario {
        raw,
        base: raw,
        known: false,
        performance: false,
        foreground: false,
        iec_all48: false,
        iec_monitor: false,
    }
}

/// Human label for diagnostics: `camera`, `video + foreground`,
/// `performance + foreground`, or `unknown(999)`.
pub fn label(raw: i32) -> String {
    let s = decode(raw);
    if !s.known {
        return format!("unknown({raw})");
    }
    let mut out = base_name(s.base).unwrap_or("?").to_string();
    let mut flags = Vec::new();
    if s.foreground {
        flags.push("foreground");
    }
    if s.performance {
        flags.push("performance");
    }
    if s.iec_all48 {
        flags.push("IEC-All48");
    }
    if s.iec_monitor {
        flags.push("IEC");
    }
    if !flags.is_empty() {
        out.push_str(" + ");
        out.push_str(&flags.join(" + "));
    }
    out
}

/// MIUI scenarios SuryaProfiler must yield to. The user selected
/// "keep custom" for video/game/benchmark, so those stay under the active
/// Device Profile.
///
/// Base `1` ("performance") is **not** a yield by itself: MIUI's performance
/// *mode* is detected separately from `thermal-global-mode` /
/// `persist.sys.thermal.config`. Treating a stale `sconfig=1` as a permanent
/// yield would never reassert the custom profile after the mode is turned
/// off (observed on device 2026-11-20).
pub fn is_yield(raw: i32) -> bool {
    let s = decode(raw);
    if !s.known {
        return false;
    }
    s.performance || matches!(s.base, 8 | 12 | 15 | 17 | 18)
}

/// True for MIUI's default scenario (base 0 without user/IEC overrides).
/// `+100` (foreground) alone is still default; the daemon can re-assert its
/// custom config after MIUI writes it.
pub fn is_default(raw: i32) -> bool {
    let s = decode(raw);
    s.known && s.base == 0 && !s.performance && !s.iec_all48 && !s.iec_monitor
}

/// IDs that must not be overwritten when releasing/yielding: either a stock
/// vendor-map entry or a MIUI camera scenario (17/18) that our generated map
/// resolves to a stock camera config. `mi_thermald` keeps the last config for
/// an ID with no entry, so forcing these to another value fights MIUI.
pub fn has_vendor_map_entry(raw: i32) -> bool {
    matches!(
        raw,
        0 | 1 | 2 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 17 | 18
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
        for v in 60..=63 {
            assert!(!decode(v).known, "{v} belongs to SuryaProfiler");
        }
        assert!(!decode(999).known);
        assert_eq!(label(999), "unknown(999)");
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
}
