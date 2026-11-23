//! Parameter catalog — the single source of truth for what SuryaProfiler may write.
//!
//! Responsibility: types + lookup + the forbidden-path guard.
//! - `entries.rs`   the registry table (keys, paths, kinds, tiers)
//! - `forbidden.rs` framework-owned prefixes/keys — never written
//!
//! Non-goals: probing devices, profile logic, IO.
//!
//! Tiers:
//! - `Free`: no writer found in ROM scripts / perf configs (safe to tune).
//! - `Baseline`: written once by `init.qcom.post_boot.sh` or transiently by
//!   the perf HAL; we set it as a profile baseline and the framework may
//!   overlay it (coexist, never fight).
//! - forbidden: runtime-owned by MIUI (thermal, perf locks, PowerKeeper game
//!   cpuset, LMK/zram, charge, cpu_boost, SELinux). Rejected even if a
//!   profile mentions them — [`guard_path`].

use serde::{Deserialize, Serialize};

mod entries;
mod forbidden;

pub use entries::catalog;
pub use forbidden::{ALLOWED_EXACT, FORBIDDEN_KEYS, FORBIDDEN_PREFIXES};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    /// No runtime or boot writer in the ROM.
    Free,
    /// Boot/transient baseline — coexist model.
    Baseline,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Plain integer (single value, exact read-back). Optional `range`
    /// validates against kernel-confirmed bounds (see Entry.range).
    Int,
    /// core_ctl `min_cpus`: kernel silently clamps to `max_cpus`
    /// (`core_ctl.c store_min_cpus` -> `min(val, state->max_cpus)`) — we
    /// pre-clamp to the live max so read-back matches.
    MinCpus,
    /// Integer written to a per-CPU array node (kernel expands it, e.g.
    /// `core_ctl/busy_up_thres` -> `60 60 60 ...`); verify all equal.
    RepeatInt,
    /// CPU frequency, clamped to the policy's real OPP list.
    Freq,
    /// Frequency cap (`scaling_max_freq`): a read-back **lower** than requested
    /// is accepted as in-sync — an external stricter cap (thermal cooling,
    /// freq-QoS) wins, per the harmony rule. Read-back higher = real drift.
    /// Empirically established 2026-10-07: thermal held gold at 1209600 while
    /// apply wanted 1555200.
    FreqMax,
    /// Frequency floor (`scaling_min_freq`): a read-back **higher** than
    /// requested is accepted as in-sync — an external boost (perf HAL
    /// `msm_performance` cpu_min_freq QoS) wins. Kernel mechanism proven in
    /// `drivers/soc/qcom/msm_performance.c: perf_adjust_notify` →
    /// `CPUFREQ_ADJUST → cpufreq_verify_within_limits(policy, min, max)`
    /// clamps `policy->min` before the store; `show_one(scaling_min_freq,
    /// min)` then reports the clamped value. Verified live 2026-10-07:
    /// with QoS min 1248000 active, writing 576000 read back 1248000, and
    /// after removing the QoS the node returned to the written value by
    /// itself. Lives < want (floor lost) = real drift.
    FreqMin,
    /// GPU pwrlevel index, clamped to `0..level_count-1`.
    PwrLevel,
    /// Governor name, must be in `scaling_available_governors`.
    Gov,
    /// GPU devfreq governor, must be in `available_governors`.
    GpuGov,
    /// Block scheduler token, must be offered by the queue.
    IoSched,
    /// TCP congestion control, must be in `tcp_available_congestion_control`.
    TcpCc,
    /// Space-separated list of integers (e.g. `tcp_rmem`).
    Ints,
    /// CPU bitmask string (`0-5`, `0-7`).
    Mask,
    /// Y/N flag (power_efficient style nodes).
    FlagYN,
    /// Free-form token compared verbatim (fallback).
    Text,
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub key: &'static str,
    pub path: &'static str,
    pub tier: Tier,
    pub kind: Kind,
    /// Scope for value lookup: "policy0" | "policy6" | "" (no clamp source).
    pub scope: &'static str,
    /// Kernel-confirmed inclusive bounds for `Kind::Int`
    /// (source-cited where enforced).
    pub range: Option<(i64, i64)>,
    /// For `core_ctl.task_thres`: validate against the cluster's CPU count —
    /// kernel rejects `val < num_cpus` (`core_ctl.c store_task_thres`).
    pub min_cpus_of: Option<&'static str>,
}

pub fn find(key: &str) -> Option<&'static Entry> {
    use std::collections::HashMap;
    use std::sync::OnceLock;
    static INDEX: OnceLock<HashMap<&'static str, &'static Entry>> = OnceLock::new();
    INDEX
        .get_or_init(|| catalog().iter().map(|e| (e.key, e)).collect())
        .get(key)
        .copied()
}

/// Returns `Err(reason)` when the path is framework-owned. Every write path
/// passes through here — profiles cannot bypass the catalog.
/// A prefix matches on a path boundary: exact, `/`, or a digit (so
/// `/sys/block/zram` also covers `/sys/block/zram0/disksize`).
pub fn guard_path(path: &str) -> Result<(), String> {
    let path = normalize_path(path);
    // Membership is the primary gate: a root write may only target a node
    // that was audited and added to the catalog. Prefix rules are defence in
    // depth for catalog entries that would otherwise match a framework path.
    if !catalog().iter().any(|e| e.path == path) {
        return Err(format!(
            "FORBIDDEN: {path} is not a cataloged writable node"
        ));
    }
    // explicitly audited exact paths win over their prefix (catalog evidence)
    if ALLOWED_EXACT.contains(&path.as_str()) {
        return Ok(());
    }
    for p in FORBIDDEN_PREFIXES {
        // Normalize a trailing slash so a prefix such as `.../cpuidle/`
        // still rejects descendants (`.../cpuidle/state0/disable`).
        let p = p.trim_end_matches('/');
        if path == p {
            return Err(format!("FORBIDDEN: {path} is owned by the MIUI framework"));
        }
        if let Some(rest) = path.strip_prefix(p) {
            if rest.is_empty()
                || rest
                    .chars()
                    .next()
                    .map(|c| c == '/' || c.is_ascii_digit())
                    .unwrap_or(false)
            {
                return Err(format!("FORBIDDEN: {path} is owned by the MIUI framework"));
            }
        }
    }
    Ok(())
}

/// Lexically normalize `//`, `.` and `..` so a path cannot dodge a prefix
/// check (e.g. `/sys/class/../module/msm_performance/...`). This is not a
/// symlink resolver; sysfs/proc nodes are direct kernel paths.
fn normalize_path(path: &str) -> String {
    use std::path::{Component, Path, PathBuf};
    let mut out = PathBuf::new();
    for c in Path::new(path).components() {
        match c {
            Component::RootDir => out.push("/"),
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(p) => out.push(p),
            Component::Prefix(_) => {}
        }
    }
    out.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests;
