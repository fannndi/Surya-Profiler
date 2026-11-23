//! `surya-prof debug env` — read-only live report for device debugging.
//!
//! Answers the "why is the CPU frequency high?" question in one shot:
//! live floors/cur/max + governors, the active profile's desired floors,
//! the stock snapshot's floors, `input_boost_freq` and the MIUI thermal
//! mode. No writes, safe while the daemon runs.

use crate::engine::apply::Store;
use serde_json::{json, Value};
use std::path::Path;

use crate::daemon::{desired_floors, live_floors};

fn read_trim(root: &Path, rel: &str) -> Option<String> {
    std::fs::read_to_string(root.join(rel))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Stock value for one key from `stock.json` (`values.<key>.value`), when
/// the snapshot exists.
fn stock_value(state_dir: &Path, key: &str) -> Option<String> {
    let raw = std::fs::read_to_string(state_dir.join("stock.json")).ok()?;
    let v: Value = serde_json::from_str(&raw).ok()?;
    v["values"][key]["value"].as_str().map(str::to_string)
}

pub fn debug_env(state_dir: &Path) -> Result<Value, String> {
    let store = Store::new(state_dir);
    let state = store.load_state();
    let files = store
        .load_profiles(None)
        .map_err(|e| format!("profiles: {e}"))?;
    let active = state.active.clone();

    // desired floors for the active profile (None when stock or the
    // profile does not manage min freq)
    let desired = active
        .as_deref()
        .and_then(|id| desired_floors(state_dir, id));

    let profile_summary = files
        .profiles
        .iter()
        .map(|p| {
            json!({
                "id": p.id,
                "label": p.label,
                "min_freq": {
                    "policy0": p.params.get("policy0.scaling_min_freq"),
                    "policy6": p.params.get("policy6.scaling_min_freq"),
                },
            })
        })
        .collect::<Vec<_>>();

    let env = crate::engine::env::Sampler::new(&crate::engine::env::default_root()).sample();

    let live = live_floors();
    let root = crate::engine::env::default_root();
    let node = |rel: &str| read_trim(&root, rel);
    let policy = |scope: &str| {
        json!({
            "cur_mhz": if scope == "policy0" { env.little_freq_mhz } else { env.big_freq_mhz },
            "min_khz": if scope == "policy0" { Some(live.little_khz) } else { Some(live.big_khz) },
            "max_mhz": if scope == "policy0" { env.little_max_mhz } else { env.big_max_mhz },
            "governor": if scope == "policy0" { &env.little_governor } else { &env.big_governor },
            "min_freq_node": node(&format!("sys/devices/system/cpu/cpufreq/{scope}/scaling_min_freq")),
            "governor_node": node(&format!("sys/devices/system/cpu/cpufreq/{scope}/scaling_governor")),
        })
    };
    let input_boost = node("sys/module/msm_performance/parameters/cpu_boost_input_freq");

    let diag_note = match (&desired, &active) {
        (None, None) => "stock: no profile holds the device".to_string(),
        (None, Some(a)) => format!("profile '{a}' does not manage scaling_min_freq"),
        (Some(d), Some(a)) => {
            let high_l = live.little_khz > d.little_khz;
            let high_b = live.big_khz > d.big_khz;
            if high_l || high_b {
                format!(
                    "floor DRIFT: profile '{a}' wants {}/{} kHz, live is {}/{} kHz{}",
                    d.little_khz,
                    d.big_khz,
                    live.little_khz,
                    live.big_khz,
                    if high_l && high_b {
                        " (both policies)"
                    } else {
                        ""
                    }
                )
            } else {
                format!("floor OK: profile '{a}' floor is live")
            }
        }
        (Some(_), None) => unreachable!("desired requires active"),
    };

    Ok(json!({
        "read_only": true,
        "state": {
            "active": active,
            "last_mode": state.last_mode,
            "updated": state.updated,
        },
        "freq": {
            "policy0": policy("policy0"),
            "policy6": policy("policy6"),
            "input_boost_freq": input_boost,
            "desired_min_khz": desired.map(|d| json!({
                "policy0": d.little_khz, "policy6": d.big_khz,
            })),
            "stock_min_khz": {
                "policy0": stock_value(state_dir, "policy0.scaling_min_freq"),
                "policy6": stock_value(state_dir, "policy6.scaling_min_freq"),
            },
            "note": diag_note,
        },
        "thermal": {
            "sconfig": env.thermal_sconfig,
            "mode": env.thermal_mode,
            "scenario": env.thermal_scenario,
            "global_mode": env.thermal_global_mode,
            "perf_config": env.thermal_perf_config,
        },
        "profiles": profile_summary,
    }))
}
