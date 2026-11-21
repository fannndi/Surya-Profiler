//! Device probing — read-only capture of catalog state + validation options
//! + framework evidence (values MIUI owns and we must never write).
//!
//! Responsibility: reading the live device into [`ProbeData`].
//! Non-goals: deciding what to write (plan/apply own that).

use crate::engine::catalog::catalog;
use std::collections::BTreeMap;
use std::fs;

mod model;

/// True when this process is running as root (uid 0).
pub fn is_root() -> bool {
    static CACHED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *CACHED.get_or_init(|| {
        std::fs::read_to_string("/proc/self/status")
            .map(|s| {
                s.lines()
                    .find(|l| l.starts_with("Uid:"))
                    .and_then(|l| l.split_whitespace().nth(2))
                    .map(|euid| euid == "0")
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    })
}

/// Whether environment-based path overrides are honored. Production (root)
/// ignores them so a caller cannot redirect the root daemon at arbitrary
/// binaries/paths; host tests (non-root) and explicit debug runs keep them.
pub fn allow_env_overrides() -> bool {
    if std::env::var("SURYAPROFILER_DEBUG").ok().as_deref() == Some("1") {
        return true;
    }
    !is_root()
}

/// Absolute IO root: `/` on device, a fixture tree in host tests
/// (`SURYAPROFILER_SYSFS_ROOT` — same env as the env sampler uses).
fn io_root() -> std::path::PathBuf {
    static ROOT: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    ROOT.get_or_init(|| {
        if allow_env_overrides() {
            if let Some(v) = std::env::var_os("SURYAPROFILER_SYSFS_ROOT") {
                return std::path::PathBuf::from(v);
            }
        }
        std::path::PathBuf::from("/")
    })
    .clone()
}

fn resolve(path: &str) -> std::path::PathBuf {
    let root = io_root();
    let root_s = root.to_string_lossy();
    if !root_s.is_empty() && path.starts_with(&*root_s) {
        // Callers that pre-join `default_root()` (e.g. the bridge node paths)
        // would double-prefix under the fixture; strip the leading root.
        root.join(path[root_s.len()..].trim_start_matches('/'))
    } else {
        root.join(path.trim_start_matches('/'))
    }
}

pub fn read(path: &str) -> Option<String> {
    fs::read_to_string(resolve(path))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn write(path: &str, value: &str) -> Result<(), String> {
    fs::write(resolve(path), value).map_err(|e| format!("{path}: {e}"))
}

/// Whitespace tokens with the active-scheduler brackets stripped.
/// Used for governor/scheduler lists AND plain numeric lines alike.
fn tokenize(s: &str) -> Vec<String> {
    s.split_whitespace()
        .map(|t| t.trim_matches(|c| c == '[' || c == ']').to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

/// `getprop` by absolute path: the binary runs under `su -c` with a minimal
/// environment (same PATH quirk as `RootBridge.suBin` on the app side).
fn getprop(name: &str) -> Option<String> {
    let out = crate::platform::run(
        "/system/bin/getprop",
        &[name],
        std::time::Duration::from_secs(2),
    )?;
    if !out.success() {
        return None;
    }
    let s = out.stdout_lossy().trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Parse `/sys/devices/system/cpu/possible` ("0-7", "0-3,4-7", "0,2,4-7")
/// into a count.
pub fn parse_cpu_count(s: &str) -> usize {
    let mut max: Option<usize> = None;
    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let mut segs = part.split('-');
        let lo: usize = match segs.next().map(str::trim).and_then(|t| t.parse().ok()) {
            Some(v) => v,
            None => return 8,
        };
        let hi: usize = match segs.next() {
            Some(h) => match h.trim().parse().ok() {
                Some(v) => v,
                None => return 8,
            },
            None => lo,
        };
        if segs.next().is_some() {
            return 8; // malformed
        }
        let m = max.get_or_insert(0);
        *m = (*m).max(hi);
    }
    max.map(|m| m + 1).unwrap_or(8)
}

/// Parse `/sys/devices/system/cpu/possible` ("0-7") into a count.
fn cpu_count() -> usize {
    read("/sys/devices/system/cpu/possible")
        .map(|s| parse_cpu_count(&s))
        .unwrap_or(8)
}

pub fn probe() -> ProbeData {
    let mut entries = BTreeMap::new();
    for e in catalog() {
        // One read is enough for the normal case; only an unreadable node
        // needs a second syscall to distinguish "absent" from "empty".
        let value = read(e.path);
        let exists = value.is_some() || resolve(e.path).exists();
        entries.insert(e.key.to_string(), EntryState { exists, value });
    }

    let mut governors = BTreeMap::new();
    let mut freqs = BTreeMap::new();
    let mut cluster_cpus = BTreeMap::new();
    let mut core_ctl_max = BTreeMap::new();
    for policy in ["policy0", "policy6"] {
        let base = format!("/sys/devices/system/cpu/cpufreq/{policy}");
        if let Some(g) = read(&format!("{base}/scaling_available_governors")) {
            governors.insert(policy.to_string(), tokenize(&g));
        }
        if let Some(f) = read(&format!("{base}/scaling_available_frequencies")) {
            let mut v: Vec<u64> = f
                .split_whitespace()
                .filter_map(|t| t.parse().ok())
                .collect();
            v.sort_unstable();
            freqs.insert(policy.to_string(), v);
        }
        if let Some(cpus) = read(&format!("{base}/related_cpus")) {
            cluster_cpus.insert(policy.to_string(), cpus.split_whitespace().count());
        }
        // core_ctl lives under the policy's first CPU (cpu0, cpu6)
        if let Some(first) = policy.strip_prefix("policy") {
            if let Some(v) = read(&format!(
                "/sys/devices/system/cpu/cpu{first}/core_ctl/max_cpus"
            )) {
                core_ctl_max.insert(policy.to_string(), v.trim().parse().unwrap_or(i64::MAX));
            }
        }
    }

    let gpu_governors = read("/sys/class/kgsl/kgsl-3d0/devfreq/available_governors")
        .map(|s| tokenize(&s))
        .unwrap_or_default();
    let gpu_levels = read("/sys/class/kgsl/kgsl-3d0/gpu_available_frequencies")
        .map(|s| s.split_whitespace().count())
        .unwrap_or(0);
    let io_schedulers = read("/sys/block/sda/queue/scheduler")
        .map(|s| tokenize(&s))
        .unwrap_or_default();
    let tcp_cc = read("/proc/sys/net/ipv4/tcp_available_congestion_control")
        .map(|s| tokenize(&s))
        .unwrap_or_default();

    let options = Options {
        governors,
        freqs,
        gpu_governors,
        gpu_levels,
        io_schedulers,
        tcp_cc,
        cpu_count: cpu_count(),
        cluster_cpus,
        core_ctl_max,
    };

    // Device identity is immutable for the life of the process; cache it so
    // every plan/apply does not spawn three extra getprop processes.
    let device = {
        static DEVICE: std::sync::OnceLock<DeviceInfo> = std::sync::OnceLock::new();
        DEVICE
            .get_or_init(|| DeviceInfo {
                device: getprop("ro.product.device").unwrap_or_else(|| "unknown".into()),
                model: getprop("ro.product.model").unwrap_or_else(|| "unknown".into()),
                rom: getprop("ro.build.version.incremental").unwrap_or_else(|| "unknown".into()),
                soc_id: read("/sys/devices/soc0/soc_id").unwrap_or_else(|| "unknown".into()),
                kernel: read("/proc/version")
                    .map(|v| v.split_whitespace().nth(2).unwrap_or("unknown").to_string())
                    .unwrap_or_else(|| "unknown".into()),
            })
            .clone()
    };

    let framework = FrameworkEvidence {
        mi_thermald: getprop("init.svc.mi_thermald"),
        perf_hal: getprop("init.svc.perf-hal-2-0"),
        perfservice: getprop("init.svc.vendor.perfservice"),
        thermal_sconfig: read("/sys/class/thermal/thermal_message/sconfig"),
        msm_perf_locks: read("/sys/module/msm_performance/parameters/cpu_max_freq").map(|v| {
            tokenize(&v)
                .into_iter()
                .take(2)
                .collect::<Vec<_>>()
                .join(" ")
        }),
        input_boost: read("/sys/module/cpu_boost/parameters/input_boost_freq").map(|v| {
            format!(
                "{v} @{}ms",
                read("/sys/module/cpu_boost/parameters/input_boost_ms").unwrap_or_default()
            )
        }),
        sched_boost: read("/proc/sys/kernel/sched_boost"),
        game_cpuset: read("/dev/cpuset/game/cpus").filter(|s| !s.is_empty()),
    };

    ProbeData {
        device,
        entries,
        options,
        framework,
    }
}

#[cfg(test)]
mod tests;

pub use model::*;
