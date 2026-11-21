//! Restore: write the stock snapshot back (engine OFF equivalent).

use super::store::Store;
use super::verify::verified_readback;
use super::write::{ordered_pairs_with, write_one};
use super::{now_secs, ApplyReport, LockedKey, SnapValue, WriteResult};
use crate::engine::catalog;
use crate::engine::plan::write_rank;
use crate::engine::probe::{self, ProbeData};

/// Restore every stock.json key to its stock value, converging over a few
/// passes because switching a governor/scheduler materializes dependent nodes
/// (`schedutil/*`, `queue/iosched/*`) only after the owner is written.
///
/// This is the **single** restore entry point; CLI, daemon worker and the
/// daemon exit path must not re-implement the retry loop.
pub fn restore(store: &Store) -> Result<ApplyReport, String> {
    let mut last: Option<ApplyReport> = None;
    for _ in 0..3 {
        let p = probe::probe();
        let report = restore_once(store, &p)?;
        let ok = report.ok;
        last = Some(report);
        if ok {
            break;
        }
    }
    Ok(last.unwrap_or_else(empty_report))
}

fn empty_report() -> ApplyReport {
    ApplyReport {
        mode: "restore".into(),
        profile: None,
        wrote: 0,
        unchanged: 0,
        verified: 0,
        failed: 0,
        locked: Vec::new(),
        results: Vec::new(),
        errors: Vec::new(),
        ok: true,
        snapshot_created: false,
        active: None,
    }
}

/// One restore pass. The stock baseline is mandatory when a profile is
/// active: never capture the current (tuned) device values as "stock".
fn restore_once(store: &Store, probe: &ProbeData) -> Result<ApplyReport, String> {
    let (mut snap, migrated_snapshot) = match store.load_stock_strict()? {
        Some(s) => (s, false),
        None => match store.load_snapshot_strict()? {
            Some(s) => (s, true),
            None => {
                // No snapshot at all: a clean install that was never tuned.
                // Refuse to fabricate one while the daemon still thinks a
                // profile is active (baseline lost).
                if store.load_state().active.is_some() {
                    return Err(
                        "stock.json is missing while a profile is active; refusing to guess".into(),
                    );
                }
                return Ok(empty_report());
            }
        },
    };
    if migrated_snapshot {
        store.save_stock(&snap)?;
    }

    // Never write a key that is no longer audited. These entries are kept in
    // stock.json for history but skipped on restore; dropping them is safer
    // than writing a path whose catalog contract changed.
    let mut entries: Vec<(String, SnapValue)> =
        std::mem::take(&mut snap.values).into_iter().collect();
    entries.sort_by(|a, b| write_rank(&a.0).cmp(&write_rank(&b.0)).then(a.0.cmp(&b.0)));

    // Kernel-validated pairs: decide safe write order against LIVE values.
    entries = ordered_pairs_with(
        entries,
        |(k, _)| k.as_str(),
        |(_, sv)| sv.value.parse().ok(),
        |key| {
            probe
                .entries
                .get(key)
                .and_then(|e| e.value.clone())
                .and_then(|v| v.parse().ok())
        },
    );

    let mut report = empty_report();
    for (key, sv) in entries {
        if sv.value.is_empty() {
            // never write an empty value: it would truncate/clear a node
            continue;
        }
        let Some(entry) = catalog::find(&key) else {
            report.locked.push(LockedKey {
                key: key.clone(),
                reason: "not in catalog; skipped on restore".into(),
            });
            continue;
        };
        let path = entry.path;
        if let Err(e) = catalog::guard_path(path) {
            report.locked.push(LockedKey {
                key: key.clone(),
                reason: format!("guard rejected stock entry: {e}"),
            });
            continue;
        }
        let current = probe
            .entries
            .get(&key)
            .and_then(|e| e.value.clone())
            .or_else(|| probe::read(path));
        if current.as_deref() == Some(sv.value.as_str()) {
            report.unchanged += 1;
            continue;
        }
        let mut res = WriteResult {
            key: key.clone(),
            path: path.to_string(),
            resolved: sv.value.clone(),
            before: current,
            written: false,
            verified: false,
            error: None,
        };
        match write_one(path, &sv.value) {
            Ok(()) => {
                res.written = true;
                report.wrote += 1;
                let (ok, err) = verified_readback(&key, path, &sv.value);
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
                res.error = Some(e);
                report.failed += 1;
                report.ok = false;
            }
        }
        report.results.push(res);
    }

    if report.failed == 0 {
        // stock.json stays on disk: it is the reconcile baseline, not a
        // one-shot session capture. Only the engine state parks off.
        let mut st = store.load_state();
        st.active = None;
        st.updated = now_secs();
        st.last_mode = "restore".into();
        store.save_state(&st)?;
        report.active = None;
    } else {
        report.active = store.load_state().active;
    }
    Ok(report)
}
