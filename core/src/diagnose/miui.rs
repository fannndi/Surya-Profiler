//! MIUI background-readiness checks (whitelist/appops/freezer).

use super::*;

fn parse_pid_list(text: &str) -> Vec<u32> {
    text.split_whitespace()
        .filter_map(|s| s.parse::<u32>().ok())
        .collect()
}

fn command_text(bin: &str, args: &[&str]) -> Option<String> {
    let out = crate::platform::run(bin, args, std::time::Duration::from_secs(5))?;
    if !out.success() {
        return None;
    }
    Some(out.stdout_lossy())
}

/// MIUI battery-optimization whitelist (`dumpsys deviceidle whitelist`).
pub(crate) fn battery_whitelist_check() -> (bool, Value) {
    let Some(text) = command_text(&dumpsys_bin(), &["deviceidle", "whitelist"]) else {
        return (
            true,
            json!({ "available": false, "hint": "dumpsys deviceidle unavailable" }),
        );
    };
    let whitelisted = text.contains("com.suryaprofiler");
    (
        whitelisted,
        json!({
            "available": true,
            "whitelisted": whitelisted,
            "hint": if whitelisted {
                "SuryaProfiler is exempt from Doze/App Standby"
            } else {
                "allow battery optimization exemption in Settings"
            },
        }),
    )
}

/// Android background app-op. MIUI normally leaves this at allow for an FGS.
pub(crate) fn background_appops_check() -> (bool, Value) {
    let Some(text) = command_text(
        "/system/bin/cmd",
        &[
            "appops",
            "get",
            "com.suryaprofiler",
            "RUN_ANY_IN_BACKGROUND",
        ],
    ) else {
        return (
            true,
            json!({ "available": false, "hint": "cmd appops unavailable" }),
        );
    };
    let lower = text.to_ascii_lowercase();
    let denied = lower.contains("ignore") || lower.contains("deny");
    (
        !denied,
        json!({
            "available": true,
            "raw": text.trim(),
            "denied": denied,
            "hint": if denied {
                "background app-op is restricted; allow background activity"
            } else {
                "background app-op is not restricted"
            },
        }),
    )
}

/// MIUI `millet_monitor` freezer: the app's PIDs must not be in the frozen
/// cgroup while the foreground service is supposed to run.
pub(crate) fn millet_frozen_check() -> (bool, Value) {
    let pids = command_text("/system/bin/pidof", &["com.suryaprofiler"])
        .or_else(|| command_text("pidof", &["com.suryaprofiler"]))
        .map(|s| parse_pid_list(&s))
        .unwrap_or_default();
    if pids.is_empty() {
        return (
            true,
            json!({ "available": false, "hint": "app process not running" }),
        );
    }
    let frozen = fs::read_to_string("/sys/fs/cgroup/frozen/cgroup.procs").unwrap_or_default();
    let frozen_pids = parse_pid_list(&frozen);
    let hits: Vec<u32> = pids
        .iter()
        .copied()
        .filter(|pid| frozen_pids.contains(pid))
        .collect();
    (
        hits.is_empty(),
        json!({
            "available": true,
            "app_pids": pids,
            "frozen_pids": hits,
            "hint": if hits.is_empty() {
                "app is not frozen by millet_monitor"
            } else {
                "app is frozen; check MIUI autostart/battery restrictions"
            },
        }),
    )
}
