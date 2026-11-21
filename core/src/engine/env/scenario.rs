//! MIUI thermal scenario labels.

/// MIUI `sconfig` -> loaded config name, mirroring `/vendor/etc/thermal-map.conf`.
///
/// 1/2/11/14 point at configs this ROM does not ship (`high`, `extreme`,
/// `class0`, `youtube`); mi_thermald falls back to `thermal-normal.conf`
/// (verified on device 2026-11-20).
pub fn thermal_mode_name(sconfig: i32) -> Option<&'static str> {
    match sconfig {
        0 | 1 | 2 | 11 | 14 => Some("normal"),
        8 => Some("phone"),
        9 | 13 | 16 => Some("tgame"),
        10 => Some("nolimits"),
        12 => Some("camera"),
        15 => Some("arvr"),
        // Surya private indices (installed by the thermal bridge).
        60 => Some("custom-balance"),
        61 => Some("custom-game"),
        62 => Some("custom-powersave"),
        63 => Some("custom-sleep"),
        _ => None,
    }
}

/// UI label: custom profiles keep their `custom-*` name; MIUI scenarios use
/// the PowerKeeper decoder (including offset flags).
pub fn thermal_scenario_label(sconfig: i32) -> String {
    thermal_mode_name(sconfig)
        .filter(|name| name.starts_with("custom-"))
        .map(str::to_string)
        .unwrap_or_else(|| crate::engine::thermal_scenario::label(sconfig))
}
