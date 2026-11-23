//! One-shot CLI commands.

use super::*;

pub(super) fn run(a: &Args) -> Result<(i32, String), String> {
    let store = Store::new(&a.state_dir);
    match a.cmd.as_str() {
        "probe" => {
            let p = probe::probe();
            let json = serde_json::to_string_pretty(&p).map_err(|e| e.to_string())?;
            Ok((0, json))
        }
        "profiles" => {
            let files = store.load_profiles(a.profiles.as_deref())?;
            let json = serde_json::to_string_pretty(
                &files
                    .profiles
                    .iter()
                    .map(|p| {
                        serde_json::json!({
                            "id": p.id, "label": p.label, "desc": p.desc, "hidden": p.hidden, "params": p.params.len()
                        })
                    })
                    .collect::<Vec<_>>(),
            )
            .map_err(|e| e.to_string())?;
            Ok((0, json))
        }
        "plan" | "apply" | "verify" => {
            let id =
                a.id.as_deref()
                    .ok_or_else(|| format!("missing profile id\n{USAGE}"))?;
            if a.cmd == "apply" {
                require_root()?;
                // Single writer: the daemon owns the device while it runs.
                daemon::ensure_no_daemon(&a.state_dir)?;
            }
            let files = store.load_profiles(a.profiles.as_deref())?;
            let profile = files.profiles.iter().find(|p| p.id == id).ok_or_else(|| {
                format!(
                    "unknown profile '{id}' (have: {})",
                    files
                        .profiles
                        .iter()
                        .map(|p| p.id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
            match a.cmd.as_str() {
                "plan" => {
                    let p = probe::probe();
                    let plan = build_plan(profile, &p);
                    let json = serde_json::to_string_pretty(&plan).map_err(|e| e.to_string())?;
                    let code = if plan.ok { 0 } else { 1 };
                    Ok((code, json))
                }
                "verify" => {
                    let p = probe::probe();
                    let plan = build_plan(profile, &p);
                    let report = apply::verify_plan(&store, &plan);
                    let json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
                    let code = if report.ok { 0 } else { 2 };
                    Ok((code, json))
                }
                _ => {
                    // apply probes + plans internally (pass-2 included)
                    let report = apply::apply_with_pass2(&store, profile, true)?;
                    let json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
                    let code = if report.ok { 0 } else { 2 };
                    Ok((code, json))
                }
            }
        }
        "restore" => {
            require_root()?;
            daemon::ensure_no_daemon(&a.state_dir)?;
            // Release holds first: a thermal cpufreq cap can clamp engine
            // restore writes and turn a clean stop into a false failure.
            let release = daemon::clean_holds(&a.state_dir);
            let report = apply::restore(&store)?;
            let mut payload = serde_json::to_value(&report).map_err(|e| e.to_string())?;
            payload["holds"] = serde_json::to_value(release).map_err(|e| e.to_string())?;
            let ok = report.ok && release.ok();
            payload["ok"] = serde_json::json!(ok);
            let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
            let code = if ok { 0 } else { 2 };
            Ok((code, json))
        }
        "clean" => {
            require_root()?;
            daemon::ensure_no_daemon(&a.state_dir)?;
            let release = daemon::clean_holds(&a.state_dir);
            let payload = serde_json::json!({
                "ok": release.ok(),
                "saver_ok": release.saver_ok,
                "input_suspend_ok": release.input_suspend_ok,
                "thermal_ok": release.thermal_ok,
                "failed_surfaces": release.failures(),
            });
            let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
            let code = if release.ok() { 0 } else { 2 };
            Ok((code, json))
        }
        "thermal-clean" => {
            require_root()?;
            daemon::ensure_no_daemon(&a.state_dir)?;
            daemon::cleanup_thermal()?;
            Ok((0, serde_json::json!({ "ok": true }).to_string()))
        }
        "catalog" => {
            // Full parameter catalog (used by tools/owner-map-audit.sh and
            // for UI debugging): every writable key with tier + constraints.
            let entries: Vec<serde_json::Value> = catalog::catalog()
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "key": e.key, "path": e.path, "tier": e.tier,
                        "kind": format!("{:?}", e.kind), "scope": e.scope,
                        "range": e.range, "min_cpus_of": e.min_cpus_of,
                    })
                })
                .collect();
            let payload = serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?;
            Ok((0, payload))
        }
        "doctor" => {
            // environment self-check (device debugging without the app);
            // --fix-thermal additionally removes foreign thermal ships.
            if a.fix_thermal {
                require_root()?;
                daemon::ensure_no_daemon(&a.state_dir)?;
            }
            let config = a
                .config
                .clone()
                .unwrap_or_else(|| a.state_dir.join("config.json"));
            let (ok, payload) = diagnose::run(&a.state_dir, Some(&config), a.fix_thermal);
            let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
            Ok((if ok { 0 } else { 1 }, json))
        }
        "status" => {
            let state = store.load_state();
            let snap = store.load_snapshot();
            let files = store.load_profiles(a.profiles.as_deref())?;
            let p = probe::probe();
            let status = serde_json::json!({
                "state_dir": store.dir(),
                "active": state.active,
                "updated": state.updated,
                "last_mode": state.last_mode,
                "snapshot": snap.as_ref().map(|s| serde_json::json!({
                    "created": s.created, "keys": s.values.len(), "device": s.device,
                })),
                "profiles": files.profiles.iter().map(|x| &x.id).collect::<Vec<_>>(),
                "device": p.device,
                "framework": p.framework,
                "root": is_root(),
                "catalog": {
                    "total": catalog::catalog().len(),
                    "free": catalog::catalog().iter().filter(|e| e.tier == catalog::Tier::Free).count(),
                    "baseline": catalog::catalog().iter().filter(|e| e.tier == catalog::Tier::Baseline).count(),
                    "present": p.entries.values().filter(|e| e.exists).count(),
                },
            });
            let json = serde_json::to_string_pretty(&status).map_err(|e| e.to_string())?;
            Ok((0, json))
        }
        other => Err(format!("unknown command '{other}'\n{USAGE}")),
    }
}
