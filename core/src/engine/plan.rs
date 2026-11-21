//! Plan model + build_plan: profile + probe -> ordered, validated plan.
//!
//! Responsibility: per-key status (Ok/Unchanged/Locked), kernel-invariant
//! pair checks, write ordering; hard-fail on unknown/forbidden keys.
//! Non-goals: touching the device (`apply` owns writes).

use crate::engine::catalog::{self, Kind, Tier};
use crate::engine::probe::ProbeData;
use crate::engine::profile::Profile;
use crate::engine::readback::{readback_matches, readback_matches_exact};
use crate::engine::validate::validate_value;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OpStatus {
    /// Will be written (differs from current).
    Ok,
    /// Already the wanted value.
    Unchanged,
    /// Cannot apply: node missing or value invalid on this device.
    Locked(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct PlannedOp {
    pub key: String,
    pub path: String,
    pub tier: Tier,
    pub wanted: String,
    /// Wanted after clamping/validation (e.g. nearest OPP).
    pub resolved: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<String>,
    pub status: OpStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct Plan {
    pub profile_id: String,
    pub label: String,
    pub ops: Vec<PlannedOp>,
    /// Keys rejected as unknown/forbidden — non-empty means the plan is void.
    pub errors: Vec<String>,
    pub ok: bool,
}

/// Ordering of writes: framework-safety first (sysctls), CPU governor/freq
/// last (most impactful; max before min so a raise never reads a stale cap).
pub fn write_rank(key: &str) -> u8 {
    if key.starts_with("vm.") {
        10
    } else if key.starts_with("net.") {
        20
    } else if key.starts_with("kernel.") {
        30
    } else if key.starts_with("workqueue.") {
        40
    } else if key == "io.scheduler" {
        // Switching the elevator materializes queue/iosched/*; the owner
        // must be written before its dependent tunables.
        45
    } else if key.starts_with("io.") {
        50
    } else if key.starts_with("gpu.") {
        60
    } else if key.starts_with("cpuset.") {
        70
    } else if key.starts_with("stune.") {
        80
    } else if key.contains(".core_ctl.") {
        90
    } else if key.ends_with(".scaling_governor") {
        // The governor materializes its own tunables (schedutil/*); switch it
        // before writing those nodes, in both directions.
        95
    } else if key.contains(".schedutil.") {
        100
    } else if key.ends_with(".scaling_max_freq") {
        120
    } else if key.ends_with(".scaling_min_freq") {
        130
    } else {
        140
    }
}

pub fn build_plan(profile: &Profile, probe: &ProbeData) -> Plan {
    build_plan_opt(profile, probe, false)
}

/// Same as [`build_plan`] (`exact_freq = true` used by the reconcile pass:
/// leftover FreqMin/FreqMax values are rewritten, not accepted as "external
/// QoS winning").
pub fn build_plan_opt(profile: &Profile, probe: &ProbeData, exact_freq: bool) -> Plan {
    let mut errors = Vec::new();
    let mut ops = Vec::new();

    for (key, wanted) in &profile.params {
        let e = match catalog::find(key) {
            Some(e) => e,
            None => {
                // Distinguish "unknown typo" from "known framework key".
                let hint = if catalog::FORBIDDEN_KEYS.contains(&key.as_str()) {
                    "FORBIDDEN: owned by the MIUI framework"
                } else {
                    "unknown key (not in catalog)"
                };
                errors.push(format!("{key}: {hint}"));
                continue;
            }
        };
        if let Err(msg) = catalog::guard_path(e.path) {
            errors.push(format!("{key}: {msg}"));
            continue;
        }

        let state = probe.entries.get(key);
        let exists = state.map(|s| s.exists).unwrap_or(false);
        let current = state.and_then(|s| s.value.clone());

        let (resolved, problem) = if !exists {
            (
                wanted.clone(),
                Some("node missing on this device".to_string()),
            )
        } else {
            // core_ctl.min_cpus is clamped against max_cpus; use the value
            // this profile plans to write, not the previous profile's live
            // cap (sleep leaves 2, wake to game must still reach 6/6).
            let planned_max = (e.kind == Kind::MinCpus)
                .then(|| {
                    profile
                        .params
                        .get(&format!("{}.core_ctl.max_cpus", e.scope))
                        .and_then(|v| v.trim().parse::<i64>().ok())
                        .filter(|v| *v >= 0)
                })
                .flatten();
            validate_value(e, wanted, probe, planned_max)
        };

        let status = match problem {
            Some(msg) => OpStatus::Locked(msg),
            None => match &current {
                Some(cur) if exact_freq => {
                    if readback_matches_exact(e.kind, &resolved, cur) {
                        OpStatus::Unchanged
                    } else {
                        OpStatus::Ok
                    }
                }
                Some(cur) if readback_matches(e.kind, &resolved, cur) => OpStatus::Unchanged,
                _ => OpStatus::Ok,
            },
        };

        ops.push(PlannedOp {
            key: key.clone(),
            path: e.path.to_string(),
            tier: e.tier,
            wanted: wanted.clone(),
            resolved,
            current,
            status,
        });
    }

    // Kernel invariant: sched_downmigrate < sched_upmigrate (validated by
    // the kernel on every write, EINVAL otherwise). Both are free params in
    // a profile? enforce the pair before anything reaches the device.
    if let (Some(up), Some(down)) = (
        profile.params.get("kernel.sched_upmigrate"),
        profile.params.get("kernel.sched_downmigrate"),
    ) {
        match (up.trim().parse::<i64>(), down.trim().parse::<i64>()) {
            (Ok(u), Ok(d)) if d < u => {}
            (Ok(u), Ok(d)) => errors.push(format!(
                "kernel.sched pair invalid: downmigrate ({d}) must be < upmigrate ({u})"
            )),
            _ => {} // non-numeric values get caught by per-key validation
        }
    }

    // WALT group migration pair (pct values): sysctl.c registers
    // sched_group_upmigrate with min = downmigrate (extra1) and
    // sched_group_downmigrate with max = upmigrate (extra2) — the kernel
    // rejects down > up, equality is legal (999/999 is a real MIUI branch).
    if let (Some(up), Some(down)) = (
        profile.params.get("kernel.sched_group_upmigrate"),
        profile.params.get("kernel.sched_group_downmigrate"),
    ) {
        match (up.trim().parse::<i64>(), down.trim().parse::<i64>()) {
            (Ok(u), Ok(d)) if d <= u => {}
            (Ok(u), Ok(d)) => errors.push(format!(
                "kernel.sched_group pair invalid: downmigrate ({d}) must be <= upmigrate ({u})"
            )),
            _ => {} // non-numeric values get caught by per-key validation
        }
    }

    // GPU pwrlevel invariant: max_pwrlevel <= min_pwrlevel — the kgsl driver
    // silently clamps `level > min_pwrlevel` to min (kgsl_pwrctrl.c:692),
    // equality is legal (single allowed level).
    if let (Some(maxl), Some(minl)) = (
        profile.params.get("gpu.max_pwrlevel"),
        profile.params.get("gpu.min_pwrlevel"),
    ) {
        match (maxl.trim().parse::<i64>(), minl.trim().parse::<i64>()) {
            (Ok(mx), Ok(mn)) if mx <= mn => {}
            (Ok(mx), Ok(mn)) => errors.push(format!(
                "gpu pwrlevel pair invalid: min_pwrlevel ({mn}) must be >= max_pwrlevel ({mx}) — kgsl would clamp"
            )),
            _ => {}
        }
    }

    ops.sort_by(|a, b| {
        write_rank(&a.key)
            .cmp(&write_rank(&b.key))
            .then(a.key.cmp(&b.key))
    });

    Plan {
        profile_id: profile.id.clone(),
        label: profile.label.clone(),
        ok: errors.is_empty(),
        ops,
        errors,
    }
}

#[cfg(test)]
mod tests;
