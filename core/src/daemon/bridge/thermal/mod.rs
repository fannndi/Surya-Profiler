//! Per-profile thermal takeover for MIUI's `mi_thermald`.
//!
//! Four custom configs (balance/game/powersave/sleep) live in
//! `core/assets/thermal/`. They are encrypted at write time with the same
//! AES-128-CBC key as the vendor blobs and installed into
//! `/data/vendor/thermal/config/` together with a custom `thermal-map.conf`.
//!
//! The custom map is generated from `/vendor/etc/thermal-map.conf` so a ROM
//! update that adds stock entries is preserved. SuryaProfiler appends four
//! private `sconfig` indices:
//!
//! | Profile | `sconfig` |
//! |---|---:|
//! | balance | 60 |
//! | game | 61 |
//! | powersave | 62 |
//! | sleep | 63 |
//!
//! **Why not 20–23?** PowerKeeper's `ThermalSwitchHandler` uses base IDs
//! 20–25 for game-normal/video/demo/yuanshen and adds offsets (+30, +100,
//! +200, +500). Its package map maps YouTube to 21, for example. 60–63 is
//! outside every MIUI value and below mi_thermald's map-ID limit (0–64).
//!
//! The SS/GPU steps use the `nvm-therm-adc` zone directly. The vendor config
//! wraps the same zone in a weight-1 `[VIRTUAL-SENSOR]`; referencing
//! `VIRTUAL-SENSOR` without defining it is what `check_sensor_valid` rejects,
//! so our configs address the real zone instead (equivalent on surya).
//! Screen-on policy floors (big 1.8432 GHz, little 1.6128 GHz) mean thermal
//! targets below those values are clamped by the kernel; the custom steps
//! therefore use real frequencies at/above the floor.
//!
//! Ownership: a `.suryaprofiler` marker makes the takeover idempotent and
//! prevents us from clobbering another thermal mod's `/data` files. On
//! Service OFF the marker-owned files are removed and the original
//! `sconfig` is restored. MIUI camera/phone/ARVR/performance scenarios are
//! yielded to; video/game/benchmark/unknown scenarios keep the active
//! Device Profile (the user selected "keep custom").

use super::{Bridge, State, SyncCtx};
use crate::engine::crypt;
use crate::engine::env::default_root;
use crate::engine::thermal_scenario;
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

pub(crate) const MARKER_NAME: &str = ".suryaprofiler";
pub(crate) const MARKER_BODY: &str = "surya-profiler thermal takeover v2\n";

mod files;
mod install;
mod io;
mod map;
mod paths;
mod perf;
mod profile;
mod release;
mod select;
mod sync;

#[cfg(test)]
mod tests;

pub(crate) use install::{install_all, remove_owned};
pub(crate) use io::{
    config_file_allowed, normalize, read_i32, verify_loaded, write_atomic, write_text,
};
pub(crate) use map::{build_map, map_contains_owned, parse_map};
pub(crate) use paths::marker_exists;
pub(crate) use paths::{
    config_dir, config_path, decrypt_path, global_mode_path, map_path, marker_path,
    perf_config_path, sconfig_path,
};
pub(crate) use perf::{
    force_perf_stock, getprop_i32, miui_performance_mode, yield_reason, yield_value,
};
pub(crate) use profile::{
    is_custom_index, is_legacy_index, ThermalProfile, CUSTOM_INDICES, LEGACY_INDICES,
    PREVIEW_INDICES,
};
pub(crate) use select::{select_profile, SelectOutcome};

/// Read-only foreign-file scan for `doctor`. Safe at any time.
pub fn files_scan() -> Value {
    files::scan()
}

/// Removes foreign thermal ships + repairs dir metadata (CLI-only, daemon off).
pub fn files_fix() -> Value {
    files::fix()
}

pub fn cleanup_orphan() -> Result<(), String> {
    if !marker_path().exists() && !map_contains_owned() {
        return Ok(());
    }
    let live = read_i32(&sconfig_path()).unwrap_or(0);
    let owned_live = is_custom_index(live) || is_legacy_index(live);
    let default_live = live == 0 || thermal_scenario::is_default(live);
    if owned_live || default_live || !thermal_scenario::has_vendor_map_entry(live) {
        write_text(&sconfig_path(), "0\n")?;
    }
    if remove_owned() {
        Ok(())
    } else {
        Err("thermal cleanup incomplete (map or marker removal failed)".into())
    }
}

/// Read-only inspection for `doctor` and diagnostics. Never mutates.
pub fn inspect() -> Value {
    let marker = marker_path().exists();
    let map_exists = map_path().exists();
    let mut map_ids = Vec::new();
    let mut map_source = "missing";
    if map_exists {
        if let Ok(bytes) = fs::read(map_path()) {
            if let Ok(plain) = crypt::decrypt(&bytes) {
                if let Ok(text) = String::from_utf8(plain) {
                    if let Ok(entries) = parse_map(&text) {
                        map_ids = entries.iter().map(|(id, _)| *id).collect();
                        map_source = "data";
                    }
                }
            }
        }
    }
    let sconfig = read_i32(&sconfig_path());
    let decrypt_profile = fs::read_to_string(decrypt_path()).ok().and_then(|raw| {
        ThermalProfile::all()
            .iter()
            .find(|p| normalize(&raw) == normalize(p.plaintext()))
            .map(|p| p.id().to_string())
    });
    json!({
        "marker": marker,
        "map_exists": map_exists,
        "map_source": map_source,
        "map_entries": map_ids.len(),
        "custom_indices": CUSTOM_INDICES,
        "has_custom_indices": CUSTOM_INDICES.iter().all(|id| map_ids.contains(id)),
        "has_legacy_indices": LEGACY_INDICES.iter().chain(PREVIEW_INDICES.iter()).any(|id| map_ids.contains(id)),
        "sconfig": sconfig,
        "scenario": sconfig.map(crate::engine::env::thermal_scenario_label),
        "decrypt_profile": decrypt_profile,
        "global_mode": read_i32(&global_mode_path()),
        "perf_config": read_i32(&perf_config_path())
            .or_else(|| getprop_i32("persist.sys.thermal.config")),
        "performance_mode": miui_performance_mode(),
    })
}
