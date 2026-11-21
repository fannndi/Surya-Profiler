//! Thermal takeover + MIUI thermal-mode checks.

use super::*;
use crate::engine::thermal_log;

pub(crate) fn run(r: &mut Report, root: &std::path::Path) {
    let thermal = crate::daemon::thermal_inspect();
    let marker = thermal["marker"].as_bool().unwrap_or(false);
    let has_custom = thermal["has_custom_indices"].as_bool().unwrap_or(false);
    let has_legacy = thermal["has_legacy_indices"].as_bool().unwrap_or(false);
    let decrypt_profile = thermal["decrypt_profile"].as_str();
    let decrypt_ok = decrypt_profile.is_none()
        || matches!(
            decrypt_profile,
            Some("balance" | "game" | "powersave" | "sleep")
        );
    r.check(
        "thermal_takeover",
        !marker || (has_custom && !has_legacy && decrypt_ok),
        false,
        json!({
            "marker": marker,
            "map_source": thermal["map_source"],
            "map_entries": thermal["map_entries"],
            "has_custom_indices": has_custom,
            "has_legacy_indices": has_legacy,
            "sconfig": thermal["sconfig"],
            "scenario": thermal["scenario"],
            "decrypt_profile": decrypt_profile,
            "hint": if marker && !has_custom {
                "custom map missing 60–63; toggle Service OFF/ON"
            } else if has_legacy {
                "legacy 20–23 entry found; restart the service to migrate"
            } else if !decrypt_ok {
                "decrypt.txt does not match a shipped custom config"
            } else {
                "thermal takeover state is consistent"
            },
        }),
    );
    r.check(
        "miui_thermal_mode",
        true,
        false,
        json!({
            "sconfig": thermal["sconfig"],
            "scenario": thermal["scenario"],
            "global_mode": thermal["global_mode"],
            "perf_config": thermal["perf_config"],
            "performance_mode": thermal["performance_mode"],
            "hint": if thermal["performance_mode"].as_bool().unwrap_or(false) {
                "MIUI performance mode is active; custom thermal yields"
            } else {
                "MIUI performance mode is off"
            },
        }),
    );
    let events = thermal_log::load_recent(root, 8);
    r.check(
        "thermal_log",
        true,
        false,
        json!({
            "events": events.len(),
            "last": events.last(),
            "hint": if events.is_empty() {
                "no mi_thermald dump yet (normal on a fresh boot)"
            } else {
                "newest mi_thermald events parsed"
            },
        }),
    );
}
