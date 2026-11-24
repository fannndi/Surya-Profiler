//! `doctor` — one-shot environment self-check for device debugging.
//!
//! Reports what the daemon needs to function: root, helper binaries, state
//! dir, catalog presence, profile pack, config/holds parse. JSON output so
//! tools and the app can consume it; the exit code is non-zero when any
//! critical check fails.

use crate::engine::apply::Store;
use crate::engine::probe;
use crate::engine::profile::parse_profiles;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

struct Report {
    ok: bool,
    checks: Vec<Value>,
}

impl Report {
    fn new() -> Self {
        Report {
            ok: true,
            checks: Vec::new(),
        }
    }

    fn check(&mut self, name: &str, pass: bool, critical: bool, detail: Value) {
        if critical && !pass {
            self.ok = false;
        }
        self.checks.push(json!({
            "name": name,
            "pass": pass,
            "critical": critical,
            "detail": detail,
        }));
    }
}

fn euid_is_root() -> bool {
    fs::read_to_string("/proc/self/status")
        .map(|s| {
            s.lines()
                .find(|l| l.starts_with("Uid:"))
                .and_then(|l| l.split_whitespace().nth(2))
                .map(|euid| euid == "0")
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

pub fn run(state_dir: &Path, config_path: Option<&Path>, fix_thermal: bool) -> (bool, Value) {
    let mut r = Report::new();

    // --- root ---------------------------------------------------------------
    r.check(
        "root",
        euid_is_root(),
        true,
        json!({ "hint": "run via su" }),
    );

    // --- helper binaries (absolute paths; the su context has no PATH) -------
    for bin in [
        "/system/bin/logcat",
        "/system/bin/settings",
        "/system/bin/getprop",
    ] {
        r.check(
            "binary",
            Path::new(bin).exists(),
            false,
            json!({ "path": bin, "exists": Path::new(bin).exists() }),
        );
    }

    // --- state dir ------------------------------------------------------------
    let dir_ok = fs::create_dir_all(state_dir).is_ok();
    let dir_writable = dir_ok
        && fs::metadata(state_dir)
            .map(|m| !m.permissions().readonly())
            .unwrap_or(false);
    r.check(
        "state_dir",
        dir_writable,
        true,
        json!({ "path": state_dir.display().to_string(), "writable": dir_writable }),
    );

    // --- profile pack (the DEPLOYED file, not just the embedded default) ------
    let local_pack = state_dir.join("profiles.json");
    let (pack_text, pack_source) = if local_pack.exists() {
        (
            fs::read_to_string(&local_pack).unwrap_or_default(),
            local_pack.display().to_string(),
        )
    } else {
        (
            crate::DEFAULT_PROFILES_JSON.to_string(),
            "embedded".to_string(),
        )
    };
    match parse_profiles(&pack_text) {
        Ok(f) => r.check(
            "profiles",
            true,
            true,
            json!({
                "source": pack_source,
                "count": f.profiles.len(),
                "ids": f.profiles.iter().map(|p| &p.id).collect::<Vec<_>>(),
            }),
        ),
        Err(e) => r.check(
            "profiles",
            false,
            true,
            json!({ "source": pack_source, "error": e }),
        ),
    }

    // --- catalog presence on this device ---------------------------------------
    let p = probe::probe();
    let present = p.entries.values().filter(|e| e.exists).count();
    let total = p.entries.len();
    r.check(
        "catalog",
        present > total / 2, // a healthy surya shows most nodes present
        false,
        json!({ "present": present, "total": total, "device": p.device.device }),
    );

    let root = crate::engine::env::default_root();
    let env = crate::engine::env::Sampler::new(&root).sample();
    env_checks::run(&mut r, &env);
    thermal_checks::run(&mut r, &root);
    thermal_files::run(&mut r, fix_thermal);

    // --- UFS health (informational, read-only) ---------------------------------
    let (storage_life, storage_eol) = crate::engine::env::storage_health();
    r.check(
        "storage_health",
        storage_life.is_some() || storage_eol.is_some(),
        false,
        json!({
            "life_time_estimation_a": storage_life,
            "eol_info": storage_eol,
            "hint": "0x06 = 50–60% life used; eol 0x01 = <80% reserved blocks consumed",
        }),
    );

    // --- MIUI background readiness (informational) ------------------------------
    let (whitelist_ok, whitelist_detail) = battery_whitelist_check();
    r.check("battery_whitelist", whitelist_ok, false, whitelist_detail);
    let (appops_ok, appops_detail) = background_appops_check();
    r.check("appops_background", appops_ok, false, appops_detail);
    let (frozen_ok, frozen_detail) = millet_frozen_check();
    r.check("millet_frozen", frozen_ok, false, frozen_detail);

    // --- future knob surface (informational; Phase 12 candidates) --------------
    let optional = [
        ("devfreq_cpubw", "sys/class/devfreq/soc:qcom,cpubw"),
        ("kgsl_idle_timer", "sys/class/kgsl/kgsl-3d0/idle_timer"),
        ("io_read_ahead_kb", "sys/block/sda/queue/read_ahead_kb"),
    ];
    let present: Vec<Value> = optional
        .iter()
        .map(|(n, rel)| json!({ "name": n, "exists": root.join(rel).exists() }))
        .collect();
    r.check("future_knobs", true, false, json!({ "paths": present }));

    // --- logcat epoch format (freshness arithmetic) -----------------------------
    let epoch_ok = crate::platform::run(
        "/system/bin/logcat",
        &["-b", "events", "-v", "epoch", "-d", "-t", "5"],
        std::time::Duration::from_secs(5),
    )
    .map(|o| {
        o.stdout_lossy().lines().any(|l| {
            l.split_whitespace()
                .next()
                .and_then(|t| t.split('.').next())
                .and_then(|s| s.parse::<u64>().ok())
                .map(|v| v > 1_000_000_000)
                .unwrap_or(false)
        })
    })
    .unwrap_or(false);
    r.check(
        "logcat_epoch",
        epoch_ok,
        false,
        json!({ "hint": "-v epoch must print epoch.seconds as the first token" }),
    );

    // --- config.json -------------------------------------------------------------
    if let Some(cp) = config_path {
        match fs::read_to_string(cp) {
            Ok(raw) => match serde_json::from_str::<crate::daemon::DaemonConfig>(&raw) {
                Ok(cfg) => r.check(
                    "config",
                    cfg.schema <= crate::daemon::CONFIG_SCHEMA,
                    false,
                    json!({
                        "path": cp.display().to_string(),
                        "schema": cfg.schema,
                        "enabled": cfg.enabled,
                        "dynamic": cfg.dynamic,
                        "base": cfg.base_profile,
                        "mapped": cfg.app_map.len(),
                    }),
                ),
                Err(e) => r.check(
                    "config",
                    false,
                    false,
                    json!({ "path": cp.display().to_string(), "error": e.to_string() }),
                ),
            },
            Err(e) => r.check(
                "config",
                false,
                false,
                json!({ "path": cp.display().to_string(), "error": e.to_string() }),
            ),
        }
    }

    // --- holds.json (bridge crash recovery) ----------------------------------------
    let holds = state_dir.join("holds.json");
    if holds.exists() {
        let parsed = fs::read_to_string(&holds)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok());
        r.check(
            "holds",
            parsed.is_some(),
            false,
            json!({ "path": holds.display().to_string() }),
        );
    } else {
        r.check("holds", true, false, json!({ "present": false }));
    }

    // --- engine state ---------------------------------------------------------------
    let store = Store::new(state_dir);
    let st = store.load_state();
    let snap = store.load_snapshot();
    let stock = store.load_stock_strict();
    r.check(
        "engine_state",
        stock.is_ok(),
        false,
        json!({
            "active": st.active,
            "snapshot": snap.map(|s| json!({ "created": s.created, "keys": s.values.len() })),
            "stock": stock.as_ref().ok().map(|s| s.as_ref().map(|s| {
                json!({ "created": s.created, "keys": s.values.len() })
            })),
        }),
    );
    r.check(
        "restore_baseline",
        stock.is_ok(),
        true,
        match stock {
            Ok(Some(s)) => json!({ "present": true, "keys": s.values.len() }),
            Ok(None) => json!({ "present": false, "hint": "no profile has been applied yet" }),
            Err(e) => json!({ "present": true, "error": e }),
        },
    );

    (r.ok, json!({ "ok": r.ok, "checks": r.checks }))
}

mod battery;
mod debug_env;
mod env_checks;
mod miui;
mod thermal_checks;
mod thermal_files;

#[cfg(test)]
mod tests;

pub(crate) use battery::{battery_service_check, dumpsys_bin};
pub use debug_env::debug_env;
pub(crate) use miui::{background_appops_check, battery_whitelist_check, millet_frozen_check};
