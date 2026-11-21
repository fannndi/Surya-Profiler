//! BatteryService vs fuel-gauge consistency check.

use super::*;

#[derive(Debug, PartialEq)]
pub(crate) struct BatteryDump {
    pub(crate) updates_stopped: bool,
    pub(crate) level: Option<u8>,
    pub(crate) scale: Option<u8>,
}

pub(crate) fn dumpsys_bin() -> String {
    if probe::allow_env_overrides() {
        if let Ok(v) = std::env::var("SURYAPROFILER_DUMPSYS_BIN") {
            return v;
        }
    }
    "/system/bin/dumpsys".to_string()
}

pub(crate) fn parse_battery_dump(text: &str) -> BatteryDump {
    let mut level = None;
    let mut scale = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("level:") {
            level = v.trim().parse::<u8>().ok();
        } else if let Some(v) = line.strip_prefix("scale:") {
            scale = v.trim().parse::<u8>().ok();
        }
    }
    BatteryDump {
        updates_stopped: text.contains("UPDATES STOPPED"),
        level,
        scale,
    }
}

/// BatteryService consistency check. Informational (non-critical): a stale
/// debug override does not break tuning, but it explains a status-bar vs
/// Diagnostics mismatch.
pub(crate) fn battery_service_check(sysfs_pct: Option<u8>) -> (bool, Value) {
    let Some(out) = crate::platform::run(
        &dumpsys_bin(),
        &["battery"],
        std::time::Duration::from_secs(5),
    ) else {
        return (
            true,
            json!({ "available": false, "hint": "dumpsys unavailable" }),
        );
    };
    if !out.success() {
        return (
            true,
            json!({ "available": false, "hint": "dumpsys battery failed" }),
        );
    }
    let dump = parse_battery_dump(&out.stdout_lossy());
    let service_pct = match (dump.level, dump.scale) {
        (Some(level), Some(scale)) if scale > 0 => {
            Some(((level as u32 * 100) + (scale as u32 / 2)) / scale as u32)
        }
        _ => None,
    };
    let matches = match (service_pct, sysfs_pct) {
        (Some(service), Some(sysfs)) => Some(service.abs_diff(sysfs as u32) <= 2),
        _ => None,
    };
    let pass = !dump.updates_stopped && matches != Some(false);
    let hint = if dump.updates_stopped {
        "BatteryService updates are stopped by a debug override; run: dumpsys battery reset"
    } else if matches == Some(false) {
        "BatteryService and sysfs differ; check dumpsys battery / fuel gauge"
    } else {
        "BatteryService follows the fuel gauge"
    };
    (
        pass,
        json!({
            "available": true,
            "updates_stopped": dump.updates_stopped,
            "service_level": dump.level,
            "scale": dump.scale,
            "service_pct": service_pct,
            "sysfs_capacity": sysfs_pct,
            "match": matches,
            "hint": hint,
        }),
    )
}
