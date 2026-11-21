//! Apply a validated plan: snapshot -> guarded writes -> read-back verify.

use super::store::{ensure_snapshot, ensure_stock, Store};

use super::verify::verified_readback;
use super::{now_secs, ApplyReport, LockedKey, WriteResult};
use crate::engine::catalog;
use crate::engine::plan::{build_plan_opt, OpStatus, Plan, PlannedOp};
use crate::engine::probe::{self, ProbeData};

pub(super) fn write_one(op_path: &str, resolved: &str) -> Result<(), String> {
    catalog::guard_path(op_path)?;
    probe::write(op_path, resolved)
}

/// Pairs where the kernel enforces `second > first` on every write, and the
/// default (alphabetical) order writes `first` before `second`.
const KERNEL_SAFE_PAIRS: [(&str, &str); 3] = [
    ("kernel.sched_downmigrate", "kernel.sched_upmigrate"),
    (
        "kernel.sched_group_downmigrate",
        "kernel.sched_group_upmigrate",
    ),
    ("gpu.max_pwrlevel", "gpu.min_pwrlevel"),
];

/// Reorder items so every kernel-validated pair is written safely:
/// if `want_second > cur_first` write `second` first (then `second > first`
/// holds); otherwise `first` first (then `first < cur_second` holds, since
/// `want_first < want_second <= cur_first < cur_second`).
pub(super) fn ordered_pairs_with<T, K, W>(
    mut items: Vec<T>,
    key_of: K,
    want_of: W,
    current: impl Fn(&str) -> Option<i64>,
) -> Vec<T>
where
    K: Fn(&T) -> &str,
    W: Fn(&T) -> Option<i64>,
{
    for (first_key, second_key) in KERNEL_SAFE_PAIRS {
        let i_first = items.iter().position(|t| key_of(t) == first_key);
        let i_second = items.iter().position(|t| key_of(t) == second_key);
        if let (Some(f), Some(s)) = (i_first, i_second) {
            if f < s {
                let want_second = want_of(&items[s]).unwrap_or(0);
                let cur_first = current(first_key).unwrap_or(0);
                if want_second > cur_first {
                    let second = items.remove(s);
                    items.insert(f, second);
                }
            }
            // precondition: input lists `first` before `second` for each pair
            // (callers sort by write_rank then key). A reversed input is
            // already in the safe (second-first) order and needs no reorder.
        }
    }
    items
}

fn plan_ops_ordered<'a>(plan: &'a Plan, probe: &ProbeData) -> Vec<&'a PlannedOp> {
    ordered_pairs_with(
        plan.ops.iter().collect::<Vec<_>>(),
        |o| o.key.as_str(),
        |o| o.resolved.parse().ok(),
        |key| {
            probe
                .entries
                .get(key)
                .and_then(|e| e.value.clone())
                .and_then(|v| v.parse().ok())
        },
    )
}

/// Apply a validated plan: snapshot -> guarded writes -> read-back verify.
/// Returns the report; `report.ok == false` only when a real (non-transient)
/// mismatch or write error occurred.
pub fn apply_plan(store: &Store, plan: &Plan, probe: &ProbeData) -> Result<ApplyReport, String> {
    if !plan.ok {
        return Err(format!("plan rejected: {}", plan.errors.join("; ")));
    }
    let (_, snapshot_created) = ensure_snapshot(store, probe, &plan.ops)?;

    let mut report = ApplyReport {
        mode: "apply".into(),
        profile: Some(plan.profile_id.clone()),
        wrote: 0,
        unchanged: 0,
        verified: 0,
        failed: 0,
        locked: Vec::new(),
        results: Vec::new(),
        errors: Vec::new(),
        ok: true,
        snapshot_created,
        active: Some(plan.profile_id.clone()),
    };

    for op in &plan_ops_ordered(plan, probe) {
        match &op.status {
            OpStatus::Locked(reason) => {
                report.locked.push(LockedKey {
                    key: op.key.clone(),
                    reason: reason.clone(),
                });
                continue;
            }
            OpStatus::Unchanged => {
                report.unchanged += 1;
                continue;
            }
            OpStatus::Ok => {}
        }

        let mut res = WriteResult {
            key: op.key.clone(),
            path: op.path.clone(),
            resolved: op.resolved.clone(),
            before: op.current.clone(),
            written: false,
            verified: false,
            error: None,
        };
        match write_one(&op.path, &op.resolved) {
            Ok(()) => {
                res.written = true;
                report.wrote += 1;
                let (ok, err) = verified_readback(&op.key, &op.path, &op.resolved);
                if ok {
                    res.verified = true;
                    report.verified += 1;
                } else {
                    res.error = err;
                    report.failed += 1;
                    report.ok = false;
                }
            }
            Err(e) => {
                // The node exists but the kernel/SELinux rejected this value
                // on this device (read-only file, value range, avc denial).
                // Report as LOCKED — never silently skipped, never a hard
                // failure: the rest of the profile applied fine.
                report.locked.push(LockedKey {
                    key: op.key.clone(),
                    reason: format!("write rejected: {e}"),
                });
            }
        }
        if res.written {
            report.results.push(res);
        }
    }

    if report.failed == 0 {
        let mut st = store.load_state();
        st.active = Some(plan.profile_id.clone());
        st.updated = now_secs();
        st.last_mode = "apply".into();
        store.save_state(&st)?;
    } else {
        report.active = store.load_state().active;
    }
    Ok(report)
}

/// Apply a profile, then run the automatic pass-2 when the first pass hit
/// nodes that were missing under the previous governor/scheduler: switching
/// `scaling_governor` materializes `policyN/schedutil/*`, switching
/// `io.scheduler` materializes `queue/iosched/*` (cfq tunables).
/// Active profile params + the persistent stock value for every union key
/// the profile does not set (reconcile pass: heals leftover values from an
/// earlier profile / a crashed apply / MIUI races).
fn merged_profile(
    base: &crate::engine::profile::Profile,
    stock: Option<std::collections::BTreeMap<String, super::SnapValue>>,
) -> crate::engine::profile::Profile {
    let mut params = base.params.clone();
    if let Some(stock) = stock {
        for (k, v) in stock {
            if !params.contains_key(&k) && !v.value.is_empty() {
                params.insert(k, v.value);
            }
        }
    }
    crate::engine::profile::Profile {
        id: base.id.clone(),
        label: base.label.clone(),
        desc: base.desc.clone(),
        short: base.short.clone(),
        hidden: base.hidden,
        params,
    }
}

pub fn apply_with_pass2(
    store: &Store,
    profile: &crate::engine::profile::Profile,
    reconcile: bool,
) -> Result<ApplyReport, String> {
    let p = probe::probe();
    ensure_stock(store, &p, None)?;
    let reconciled = if reconcile {
        Some(merged_profile(
            profile,
            store.load_stock_strict()?.map(|s| s.values),
        ))
    } else {
        None
    };
    let eff = reconciled.as_ref().unwrap_or(profile);
    let plan = build_plan_opt(eff, &p, reconcile);
    let mut report = apply_plan(store, &plan, &p)?;

    let pass1_locked: std::collections::BTreeSet<String> =
        report.locked.iter().map(|l| l.key.clone()).collect();
    if report
        .locked
        .iter()
        .any(|l| l.reason.contains("node missing"))
    {
        let p2 = probe::probe();
        let plan2 = build_plan_opt(eff, &p2, reconcile);
        if plan2.ok {
            let r2 = apply_plan(store, &plan2, &p2)?;
            // Aggregate only keys pass-2 actually re-wrote. A pass-2
            // "unchanged" is new information only for a key pass-1 had
            // locked; everything else was already counted in pass 1.
            let already: std::collections::BTreeSet<String> =
                report.results.iter().map(|r| r.key.clone()).collect();
            for r in &r2.results {
                if !already.contains(&r.key) {
                    report.results.push(r.clone());
                    if r.written {
                        report.wrote += 1;
                    }
                    if r.verified {
                        report.verified += 1;
                    }
                }
            }
            report.failed += r2
                .results
                .iter()
                .filter(|r| r.error.is_some() && !already.contains(&r.key))
                .count();
            report.unchanged += plan2
                .ops
                .iter()
                .filter(|o| {
                    matches!(o.status, OpStatus::Unchanged) && pass1_locked.contains(&o.key)
                })
                .count();
            report.locked = r2.locked; // fresh truth per key
            report.ok = report.ok && r2.ok;
            if r2.active.is_some() {
                report.active = r2.active;
            }
        }
    }

    // A wake/touch boost raises MIUI's effective min above a profile cap for
    // ~1–3 s. The cap is planned Locked (it cannot read back during the
    // boost); retry once after the boost decays so the profile is not left
    // without its cap until the 30 min watchdog.
    let floor_locked = report
        .locked
        .iter()
        .any(|l| l.reason.contains("below live floor"));
    let retry_ms = floor_retry_ms();
    if floor_locked && retry_ms > 0 {
        std::thread::sleep(std::time::Duration::from_millis(retry_ms));
        let p3 = probe::probe();
        let plan3 = build_plan_opt(eff, &p3, reconcile);
        if plan3.ok {
            let r3 = apply_plan(store, &plan3, &p3)?;
            if r3.ok {
                return Ok(r3);
            }
        }
    }
    Ok(report)
}

fn floor_retry_ms() -> u64 {
    std::env::var("SURYAPROFILER_FLOOR_RETRY_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3_500)
}

#[cfg(test)]
mod tests;
