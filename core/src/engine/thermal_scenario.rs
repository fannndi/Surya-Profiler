//! MIUI thermal scenario IDs — decoded from `com.miui.powerkeeper`.
//!
//! `ThermalSwitchHandler` writes `/sys/class/thermal/thermal_message/sconfig`
//! with a base scenario ID (0–24) plus optional offsets:
//!
//! | Offset | Meaning (REA-confirmed 2026-11-23, smali `writeThermalId`) |
//! |---:|---|
//! | `+30` | performance mode (`mPowerModeEnable && mIsPerformance`) |
//! | `+100` | display unfolded (`!mIsDisplayFolded`) |
//! | `+200` | IEC-All48 allowed, monitor not allowed |
//! | `+500` | IEC monitor mode (test handler) |
//!
//! The handler rejects any base `>= 25` (writes 0–24 only) and skips the
//! write when the resolved ID is unchanged — matching `mi_thermald`'s own
//! change-only select. Game intents fire for base IDs 9/13/16/20/23.
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
/// `25` is kept for table completeness; REA 2026-11-23 confirmed the
/// handler rejects it, so it never appears on the node.
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
///
/// REA 2026-11-23: `writeThermalId` rejects `id >= 25` outright, so base
/// 25 never reaches the node from PowerKeeper on this build — `sconfig=25`
/// decodes as unknown here, and 60–63 stay SuryaProfiler-owned.
pub fn decode(raw: i32) -> MiuiScenario {
    if (0..=24).contains(&raw) {
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
        if (0..=24).contains(&base) {
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

/// MIUI "special game" packages whose **virtual-sensor compensation** is
/// applied by PowerKeeper while they are foreground (`mEnableSpOnVirtual
/// SensorComp && mSpeedModeStatus == 1` → message 12).
///
/// REA 2026-11-23 (`ThermalSwitchHandler.DEFAULT_SP_ON_SPECIAL_GAME_VIRTUAL
/// _SENSOR_COMPENSATION = -2000`, i.e. −2 °C): MIUI subtracts this from the
/// virtual sensor reading during these games, so `quiet-therm-adc` /
/// `board_sensor_temp` telemetry reads *lower* than the real skin temp.
/// Diagnostics surface this flag so the numbers are not over-trusted.
pub const SP_GAME_PACKAGES: &[&str] = &[
    "com.miHoYo.ys.mi",
    "com.tencent.tmgp.sgame",
    "com.miHoYo.ys.bilibili",
    "com.miHoYo.Yuanshen",
    "com.miHoYo.GenshinImpact",
];

/// True when `pkg` gets MIUI's virtual-sensor compensation.
pub fn is_miui_sp_game(pkg: &str) -> bool {
    SP_GAME_PACKAGES.contains(&pkg)
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

/// True when a raw `sconfig` value belongs to MIUI, including decoded base +
/// offset combinations that have no literal vendor-map entry (for example
/// camera+foreground 112, phone+foreground 108, and 4K/8K camera 117/118).
///
/// Unknown, truly-unclassified values deliberately remain unowned so callers
/// can keep their conservative stock fallback. Private indices are unknown to
/// the decoder and therefore never MIUI-owned.
pub fn is_miui_owned(raw: i32) -> bool {
    has_vendor_map_entry(raw) || decode(raw).known
}

#[cfg(test)]
mod tests;
