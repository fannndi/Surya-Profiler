//! MIUI performance-mode detection and yield policy helpers.

use super::*;

pub(crate) fn getprop_i32(name: &str) -> Option<i32> {
    let out = crate::platform::run(
        "/system/bin/getprop",
        &[name],
        std::time::Duration::from_secs(2),
    )?;
    if !out.success() {
        return None;
    }
    out.stdout_lossy().trim().parse().ok()
}

pub(crate) fn miui_performance_mode() -> bool {
    // The two files are authoritative when readable; only the getprop
    // fallback is cached (it is a subprocess and changes rarely).
    let global = read_i32(&global_mode_path());
    let perf = read_i32(&perf_config_path());
    if global == Some(1) || perf == Some(1) {
        return true;
    }
    if global.is_some() && perf.is_some() {
        return false;
    }
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    static CACHE: Mutex<Option<(Instant, bool)>> = Mutex::new(None);
    if let Ok(guard) = CACHE.lock() {
        if let Some((at, value)) = *guard {
            if at.elapsed() < Duration::from_secs(10) {
                return value;
            }
        }
    }
    let value = getprop_i32("persist.sys.thermal.config") == Some(1);
    if let Ok(mut guard) = CACHE.lock() {
        *guard = Some((Instant::now(), value));
    }
    value
}

pub(crate) fn yield_reason(live: Option<i32>, perf: bool) -> String {
    if perf {
        return "performance mode".into();
    }
    live.map(thermal_scenario::label)
        .unwrap_or_else(|| "unknown".into())
}

pub(crate) fn yield_value(live: Option<i32>, perf: bool) -> Option<i32> {
    if perf {
        Some(live.filter(|v| *v != 0).unwrap_or(1))
    } else {
        live
    }
}

/// Force a stock scenario while MIUI's performance mode is active. Camera /
/// phone / ARVR and every known MIUI scenario stay as MIUI set them; our
/// private indices, default values and unknown IDs move to performance
/// `sconfig=1` (which resolves to vendor normal on this ROM).
///
/// The value to force, or `None` when the live MIUI scenario must be preserved.
pub(crate) fn perf_stock_target(live: Option<i32>, legacy_owned: bool) -> Option<i32> {
    match live {
        Some(1) => None,
        Some(v) if is_custom_index(v) || (legacy_owned && is_legacy_index(v)) => Some(1),
        Some(0) => Some(1),
        Some(v) if thermal_scenario::is_miui_owned(v) => None,
        _ => Some(1),
    }
}

pub(crate) fn force_perf_stock(live: Option<i32>) -> Result<Option<i32>, String> {
    if let Some(target) = perf_stock_target(live, marker_path().exists()) {
        write_text(&sconfig_path(), &format!("{target}\n"))?;
        Ok(Some(target))
    } else {
        Ok(live)
    }
}

// --- map generation ---------------------------------------------------------
