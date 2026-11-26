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
use std::time::{Duration, Instant};

pub(crate) const MARKER_NAME: &str = ".suryaprofiler";
pub(crate) const MARKER_BODY: &str = "surya-profiler thermal takeover v2\n";

/// Backoff belongs to the profile that failed, not merely to the bridge. A
/// newly requested profile must be allowed to select immediately.
#[derive(Debug, Clone, Default)]
pub(crate) struct ThermalRetry {
    deadline: Option<Instant>,
    profile: Option<String>,
    failures: u8,
}

impl ThermalRetry {
    /// True while the same requested profile must wait. A different profile
    /// clears the old deadline and bypasses it immediately.
    pub(super) fn blocks(&mut self, desired: &str, now: Instant) -> bool {
        let Some(deadline) = self.deadline else {
            return false;
        };
        if now >= deadline {
            self.deadline = None;
            if self.profile.as_deref() != Some(desired) {
                self.recovered();
            }
            return false;
        }
        if self.profile.as_deref() == Some(desired) {
            return true;
        }
        self.recovered();
        false
    }

    /// Records a failure and returns the delay applied to this profile.
    pub(super) fn failed(&mut self, desired: &str, now: Instant) -> Duration {
        if self.profile.as_deref() != Some(desired) {
            self.failures = 0;
        }
        self.profile = Some(desired.to_string());
        self.failures = self.failures.saturating_add(1).min(6);
        let shift = (self.failures - 1).min(4) as u32;
        let secs = 15u64.saturating_mul(1u64 << shift).min(300);
        self.deadline = Some(now + Duration::from_secs(secs));
        Duration::from_secs(secs)
    }

    pub(super) fn recovered(&mut self) {
        self.deadline = None;
        self.profile = None;
        self.failures = 0;
    }
}

/// Restore point for the first takeover. Leftover private indices are ours,
/// while every decoded MIUI scenario (including offsets) belongs to MIUI even
/// when the raw ID is absent from the vendor map.
pub(super) fn initial_saved_sconfig(live: Option<i32>, marker_before: bool) -> i32 {
    match live {
        Some(v) if is_custom_index(v) || (marker_before && is_legacy_index(v)) => 0,
        Some(v) if thermal_scenario::is_miui_owned(v) => v,
        _ => 0,
    }
}

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
    let live = read_i32(&sconfig_path()).unwrap_or(0);
    if !marker_path().exists() && !map_contains_owned() && !is_custom_index(live) {
        return Ok(());
    }
    let owned_live = is_custom_index(live) || is_legacy_index(live);
    if let Some(target) = orphan_cleanup_sconfig_target(live, owned_live) {
        write_text(&sconfig_path(), &format!("{target}\n"))?;
    }
    if remove_owned() {
        Ok(())
    } else {
        Err("thermal cleanup incomplete (map or marker removal failed)".into())
    }
}

/// Stock value to write during orphan cleanup, or `None` for a live MIUI
/// scenario. Legacy indices are always treated as ours during CLI cleanup.
pub(crate) fn orphan_cleanup_sconfig_target(live: i32, owned_live: bool) -> Option<i32> {
    (owned_live || thermal_scenario::is_default(live) || !thermal_scenario::is_miui_owned(live))
        .then_some(0)
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
