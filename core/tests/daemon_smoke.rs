//! End-to-end daemon smoke tests on the host: spawn the real `surya-prof serve`
//! binary, exchange JSON-lines, assert the decision pipeline.
//!
//! Runs against fake sysfs (host): every engine op reports "node missing"
//! (locked), which is a clean apply — perfect for exercising the protocol,
//! arbiter, worker, config, bridge and env-guard paths without a device.

mod common;

use common::{tmp, Daemon, WaitTimeout};
use serde_json::{json, Value};
use std::process::{Command, Stdio};
use std::time::Duration;
use surya_profiler_core::engine::crypt;

fn wait_log(path: &std::path::Path, needle: &str, timeout: Duration) {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        if text.contains(needle) {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timeout waiting for log {needle:?}; log:\n{text}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn wait_thermal_profile(d: &mut Daemon, profile: &str, timeout: Duration) {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        d.send(json!({"cmd":"diag"}));
        let ev = d.wait_for(|v| v["event"] == "diag", Duration::from_secs(2));
        if ev["diag"]["holds"]["thermal_profile"] == profile {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timeout waiting for thermal_profile {profile}: {ev:?}"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn daemon_smoke_full_decision_path() {
    let dir = tmp("daemon");
    let cfg_path = dir.join("config.json");
    std::fs::write(
        &cfg_path,
        r#"{"schema":2,"enabled":true,"dynamic":true,"base_profile":"balance",
            "app_map":{"com.YoStarEN.AzurLane":"game","com.google.android.youtube":"powersave"}}"#,
    )
    .unwrap();

    let mut d = Daemon::spawn(&dir, &cfg_path);

    // 1. hello handshake -> hello + initial state
    d.send(json!({"cmd":"hello"}));
    let hello = d.wait_for(|v| v["event"] == "hello", Duration::from_secs(5));
    assert_eq!(hello["version"], 3);

    // 2. screen on/unlocked + foreground whatsapp -> applies base (balance)
    d.send(json!({"cmd":"screen","on":true,"locked":false}));
    d.send(json!({"cmd":"fg","pkg":"com.whatsapp"}));
    let applied = d.wait_for(
        |v| v["event"] == "applied" && v["profile"] == "balance",
        Duration::from_secs(10),
    );
    assert_eq!(applied["reason"], "base");
    assert_eq!(applied["ok"], true);

    // 3. mapped app in front -> game
    d.send(json!({"cmd":"fg","pkg":"com.YoStarEN.AzurLane"}));
    let applied = d.wait_for(
        |v| v["event"] == "applied" && v["profile"] == "game",
        Duration::from_secs(10),
    );
    assert_eq!(applied["reason"], "app");
    assert_eq!(applied["src_pkg"], "com.YoStarEN.AzurLane");

    // 4. state snapshot reflects active profile
    let st = d.wait_for(
        |v| v["event"] == "state" && v["state"]["active"] == "game",
        Duration::from_secs(5),
    );
    assert_eq!(st["state"]["foreground"], "com.YoStarEN.AzurLane");

    // 5. dynamic off -> mapped app falls back to base
    //    (app writes config.json first, then pings config_changed)
    std::fs::write(
        &cfg_path,
        r#"{"schema":2,"enabled":true,"dynamic":false,"base_profile":"balance",
            "app_map":{"com.YoStarEN.AzurLane":"game"}}"#,
    )
    .unwrap();
    d.send(json!({"cmd":"config_changed"}));
    let applied = d.wait_for(
        |v| v["event"] == "applied" && v["profile"] == "balance",
        Duration::from_secs(10),
    );
    assert_eq!(applied["reason"], "base");

    // 6. screen off -> sleep after the grace period (10 s + settle)
    d.send(json!({"cmd":"screen","on":false,"locked":true}));
    let applied = d.wait_for(
        |v| v["event"] == "applied" && v["profile"] == "sleep",
        Duration::from_secs(16),
    );
    assert_eq!(applied["reason"], "screen off");

    // 7. restore (service-off path) then shutdown
    d.send(json!({"cmd":"restore"}));
    let restored = d.wait_for(|v| v["event"] == "restored", Duration::from_secs(10));
    // The host has no device nodes, so individual writes may fail; the
    // protocol must still answer with the real counters.
    assert!(restored["ok"].is_boolean());
    assert!(restored["wrote"].is_u64());
    assert!(restored["failed"].is_u64());

    d.send(json!({"cmd":"shutdown"}));
    let bye = d.wait_for(|v| v["event"] == "bye", Duration::from_secs(5));
    assert_eq!(bye["event"], "bye");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn own_app_returns_to_universal_base() {
    let dir = tmp("ownapp");
    let cfg_path = dir.join("config.json");
    std::fs::write(
        &cfg_path,
        r#"{"schema":2,"enabled":true,"dynamic":true,"base_profile":"balance",
            "app_map":{"com.google.android.youtube":"powersave"}}"#,
    )
    .unwrap();

    let mut d = Daemon::spawn(&dir, &cfg_path);
    d.send(json!({"cmd":"hello"}));
    d.wait_for(|v| v["event"] == "hello", Duration::from_secs(5));
    d.send(json!({"cmd":"screen","on":true,"locked":false}));

    d.send(json!({"cmd":"fg","pkg":"com.google.android.youtube"}));
    let applied = d.wait_for(
        |v| v["event"] == "applied" && v["profile"] == "powersave",
        Duration::from_secs(10),
    );
    assert_eq!(applied["reason"], "app");

    // Returning to Surya Profiler itself is a real foreground change; it is
    // unmapped, so the universal base must win. The config_changed hint the
    // app sends on resume must not resurrect YouTube's mapping.
    d.send(json!({"cmd":"fg","pkg":"com.suryaprofiler"}));
    d.send(json!({"cmd":"config_changed"}));
    let applied = d.wait_for(|v| v["event"] == "applied", Duration::from_secs(10));
    assert_eq!(applied["profile"], "balance");
    assert_eq!(applied["reason"], "base");

    d.send(json!({"cmd":"config_changed"}));
    let applied = d.wait_for(|v| v["event"] == "applied", Duration::from_secs(10));
    assert_eq!(applied["profile"], "balance");
    assert_eq!(applied["reason"], "base");

    // Dynamic OFF: even the mapped app now uses the base.
    std::fs::write(
        &cfg_path,
        r#"{"schema":2,"enabled":true,"dynamic":false,"base_profile":"balance",
            "app_map":{"com.google.android.youtube":"powersave"}}"#,
    )
    .unwrap();
    d.send(json!({"cmd":"config_changed"}));
    d.wait_for(
        |v| v["event"] == "applied" && v["profile"] == "balance",
        Duration::from_secs(10),
    );
    d.send(json!({"cmd":"fg","pkg":"com.google.android.youtube"}));
    let applied = d.wait_for(
        |v| v["event"] == "applied" && v["profile"] == "balance",
        Duration::from_secs(10),
    );
    assert_eq!(applied["reason"], "base");

    d.send(json!({"cmd":"shutdown"}));
    d.wait_for(|v| v["event"] == "bye", Duration::from_secs(5));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn multi_window_probe_forces_balance_and_releases() {
    let dir = tmp("mw");
    let cfg_path = dir.join("config.json");
    std::fs::write(
        &cfg_path,
        r#"{"schema":2,"enabled":true,"dynamic":true,"base_profile":"game"}"#,
    )
    .unwrap();

    let window = dir.join("window.txt");
    let script = dir.join("dumpsys");
    std::fs::write(&script, format!("#!/bin/sh\ncat '{}'\n", window.display())).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perm = std::fs::metadata(&script).unwrap().permissions();
        perm.set_mode(0o755);
        std::fs::set_permissions(&script, perm).unwrap();
    }
    // A= lines carry the package affinity the parser reads.
    let split = "  * Task{a #6 type=standard A=10155:com.android.chrome U=0 visible=true mode=split-screen-primary translucent=false sz=1}\n  * Task{b #7 type=standard A=1000:com.android.settings.root U=0 visible=true mode=split-screen-secondary translucent=false sz=1}\n";
    std::fs::write(&window, split).unwrap();

    let script_s = script.to_str().unwrap().to_owned();
    let mut d = Daemon::spawn_env(&dir, &cfg_path, &[("SURYAPROFILER_DUMPSYS_BIN", &script_s)]);
    d.send(json!({"cmd":"hello"}));
    d.wait_for(|v| v["event"] == "hello", Duration::from_secs(5));
    d.send(json!({"cmd":"screen","on":true,"locked":false}));
    d.send(json!({"cmd":"fg","pkg":"com.whatsapp"}));

    let applied = d.wait_for(
        |v| v["event"] == "applied" && v["profile"] == "balance" && v["reason"] == "multi-window",
        Duration::from_secs(10),
    );
    assert_eq!(applied["reason"], "multi-window");
    let st = d.wait_for(
        |v| v["event"] == "state" && v["state"]["multi_window"] == true,
        Duration::from_secs(5),
    );
    assert_eq!(st["state"]["second_window"], "com.android.settings");

    // Collapse the split; a foreground change probes again.
    std::fs::write(&window, "  * Task{a #6 type=standard A=10155:com.android.chrome U=0 visible=true mode=fullscreen translucent=false sz=1}\n").unwrap();
    d.send(json!({"cmd":"fg","pkg":"com.miui.home"}));
    let applied = d.wait_for(
        |v| v["event"] == "applied" && v["profile"] == "game" && v["reason"] == "base",
        Duration::from_secs(10),
    );
    assert_eq!(applied["reason"], "base");
    d.wait_for(
        |v| v["event"] == "state" && v["state"]["multi_window"] == false,
        Duration::from_secs(5),
    );

    d.send(json!({"cmd":"shutdown"}));
    d.wait_for(|v| v["event"] == "bye", Duration::from_secs(5));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn daemon_diag_stats_and_env_events() {
    let dir = tmp("diag");
    let cfg_path = dir.join("config.json");
    std::fs::write(
        &cfg_path,
        r#"{"schema":2,"enabled":true,"dynamic":true,"base_profile":"balance"}"#,
    )
    .unwrap();

    // fake sysfs tree: battery + thermal + GPU (host has no device nodes)
    let root = dir.join("fake-root");
    let w = |rel: &str, body: &str| {
        let p = root.join(rel.trim_start_matches('/'));
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    };
    w("sys/class/power_supply/battery/capacity", "95\n");
    w("sys/class/power_supply/battery/status", "Charging\n");
    w("sys/class/power_supply/battery/temp", "320\n");
    w("sys/class/thermal/thermal_zone24/type", "cpuss-0-usr\n");
    w("sys/class/thermal/thermal_zone24/temp", "38800\n");
    w("sys/class/kgsl/kgsl-3d0/gpu_busy_percentage", "3 %\n");

    let mut d = Daemon::spawn_env(
        &dir,
        &cfg_path,
        &[("SURYAPROFILER_SYSFS_ROOT", root.to_str().unwrap())],
    );

    // startup handshake: hello -> state -> env (the sampler emits immediately)
    let hello = d.wait_for(|v| v["event"] == "hello", Duration::from_secs(5));
    assert_eq!(hello["version"], 3);
    let env_ev = d.wait_for(|v| v["event"] == "env", Duration::from_secs(5));
    assert_eq!(env_ev["env"]["battery_pct"], 95);
    assert_eq!(env_ev["env"]["charging"], true);
    assert_eq!(env_ev["env"]["battery_temp_c"], 32.0);
    assert_eq!(env_ev["env"]["cpu_temp_c"], 38.8);
    assert_eq!(env_ev["env"]["gpu_busy_pct"], 3);

    // one real switch so the history has an entry
    d.send(json!({"cmd":"screen","on":true,"locked":false}));
    d.send(json!({"cmd":"fg","pkg":"com.whatsapp"}));
    d.wait_for(
        |v| v["event"] == "applied" && v["profile"] == "balance",
        Duration::from_secs(10),
    );

    d.send(json!({"cmd":"diag"}));
    let diag = d.wait_for(|v| v["event"] == "diag", Duration::from_secs(5));
    assert!(diag["diag"]["pid"].is_u64());
    assert!(diag["diag"]["uptime_s"].is_u64());
    assert_eq!(diag["diag"]["config"]["base_profile"], "balance");
    assert_eq!(diag["diag"]["env"]["battery_pct"], 95);
    assert!(diag["diag"]["watchers"]["fg"].is_boolean());
    assert_eq!(diag["diag"]["holds"]["saver_held"], false);
    assert_eq!(diag["diag"]["holds"]["input_suspend_held"], false);
    // REA 2026-11-23: WhatsApp is not a PowerKeeper SP-game package, so the
    // virtual-sensor compensation flag must be an explicit false.
    assert_eq!(diag["diag"]["miui_sp_game"], false);

    d.send(json!({"cmd":"stats"}));
    let stats = d.wait_for(|v| v["event"] == "stats", Duration::from_secs(5));
    let entries = stats["entries"].as_array().expect("entries array");
    assert!(!entries.is_empty(), "one applied switch must be recorded");
    let last = entries.last().unwrap();
    assert_eq!(last["to"], "balance");
    assert_eq!(last["reason"], "base");
    assert_eq!(last["battery"], 95);

    // the history survives on disk for the next daemon
    let on_disk = std::fs::read_to_string(dir.join("stats.json")).unwrap();
    assert!(on_disk.contains("\"balance\""));

    d.send(json!({"cmd":"shutdown"}));
    d.wait_for(|v| v["event"] == "bye", Duration::from_secs(5));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn profile_pack_change_triggers_reevaluation() {
    let dir = tmp("profiles");
    let cfg_path = dir.join("config.json");
    std::fs::write(
        &cfg_path,
        r#"{"schema":2,"enabled":true,"dynamic":true,"base_profile":"balance"}"#,
    )
    .unwrap();

    let mut d = Daemon::spawn(&dir, &cfg_path);
    d.wait_for(|v| v["event"] == "hello", Duration::from_secs(5));
    d.send(json!({"cmd":"screen","on":true,"locked":false}));
    d.send(json!({"cmd":"fg","pkg":"com.whatsapp"}));
    d.wait_for(
        |v| v["event"] == "applied" && v["profile"] == "balance",
        Duration::from_secs(10),
    );

    // the app deploys a new profile pack while the daemon runs
    std::fs::write(
        dir.join("profiles.json"),
        r#"{"schema":1,"name":"test","device":"surya","rom":"test","profiles":[
            {"id":"balance","label":"B","desc":"","params":{"kernel.sched_nr_migrate":"24"}},
            {"id":"sleep","label":"S","desc":"","params":{"kernel.sched_nr_migrate":"8"}}]}"#,
    )
    .unwrap();

    // the mtime poll must re-evaluate with the new pack (no switch needed)
    let dec = d.wait_for(
        |v| v["event"] == "decision" && v["trigger"] == "profiles",
        Duration::from_secs(10),
    );
    assert_eq!(dec["action"], "apply");
    assert_eq!(dec["profile"], "balance");
    let applied = d.wait_for(
        |v| v["event"] == "applied" && v["profile"] == "balance",
        Duration::from_secs(10),
    );
    assert_eq!(applied["ok"], true, "reloaded pack must apply: {applied:?}");

    d.send(json!({"cmd":"shutdown"}));
    d.wait_for(|v| v["event"] == "bye", Duration::from_secs(5));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn daemon_exits_on_stdin_eof() {
    let dir = tmp("eof");
    let cfg_path = dir.join("config.json");
    std::fs::write(&cfg_path, "{}").unwrap();

    let bin = env!("CARGO_BIN_EXE_surya-prof");
    let mut child = Command::new(bin)
        .args([
            "serve",
            "--state-dir",
            dir.to_str().unwrap(),
            "--config",
            cfg_path.to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    // drop stdin immediately -> EOF -> daemon must exit
    drop(child.stdin.take());
    let status = child.wait_timeout(Duration::from_secs(5)).unwrap();
    assert!(status.is_some(), "daemon must exit when stdin closes");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn shutdown_worker_restores_an_active_profile() {
    let dir = tmp("shutdown-restore");
    let cfg_path = dir.join("config.json");
    std::fs::write(
        &cfg_path,
        r#"{"schema":2,"enabled":true,"dynamic":true,"base_profile":"balance"}"#,
    )
    .unwrap();

    // One real fake node gives ensure_stock a restore point even when the
    // rest of the catalog is absent on the host.
    let gov = dir.join("fake-root/sys/devices/system/cpu/cpufreq/policy0/scaling_governor");
    std::fs::create_dir_all(gov.parent().unwrap()).unwrap();
    std::fs::write(&gov, "schedutil\n").unwrap();

    let mut d = Daemon::spawn(&dir, &cfg_path);
    d.send(json!({"cmd":"hello"}));
    d.wait_for(|v| v["event"] == "hello", Duration::from_secs(5));
    d.send(json!({"cmd":"screen","on":true,"locked":false}));
    d.send(json!({"cmd":"fg","pkg":"com.whatsapp"}));
    d.wait_for(
        |v| v["event"] == "applied" && v["profile"] == "balance" && v["ok"] == true,
        Duration::from_secs(10),
    );
    let active =
        serde_json::from_str::<Value>(&std::fs::read_to_string(dir.join("state.json")).unwrap())
            .unwrap()["active"]
            .clone();
    assert_eq!(active, "balance");

    d.send(json!({"cmd":"shutdown"}));
    d.wait_for(|v| v["event"] == "bye", Duration::from_secs(10));
    let state: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("state.json")).unwrap()).unwrap();
    assert!(
        state["active"].is_null(),
        "exit cleanup must restore: {state}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn restore_skips_unknown_stock_keys_instead_of_failing() {
    let dir = tmp("restore-unknown");
    let cfg_path = dir.join("config.json");
    std::fs::write(&cfg_path, r#"{"schema":2,"enabled":true}"#).unwrap();
    std::fs::write(
        dir.join("stock.json"),
        serde_json::to_string(&json!({
            "created": 1,
            "device": "surya",
            "values": {
                "removed.key": {
                    "path": "/sys/class/thermal/thermal_message/sconfig",
                    "value": "0"
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let mut d = Daemon::spawn(&dir, &cfg_path);
    d.send(json!({"cmd":"hello"}));
    d.wait_for(|v| v["event"] == "hello", Duration::from_secs(5));
    d.send(json!({"cmd":"restore"}));
    let restored = d.wait_for(|v| v["event"] == "restored", Duration::from_secs(10));
    assert_eq!(
        restored["ok"], true,
        "a removed catalog key must be skipped, not block restore: {restored:?}"
    );
    d.send(json!({"cmd":"shutdown"}));
    d.wait_for(|v| v["event"] == "bye", Duration::from_secs(5));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn restore_fails_closed_on_corrupt_stock() {
    let dir = tmp("restore-corrupt");
    let cfg_path = dir.join("config.json");
    std::fs::write(&cfg_path, r#"{"schema":2,"enabled":true}"#).unwrap();
    std::fs::write(dir.join("stock.json"), "{broken").unwrap();

    let mut d = Daemon::spawn(&dir, &cfg_path);
    d.send(json!({"cmd":"hello"}));
    d.wait_for(|v| v["event"] == "hello", Duration::from_secs(5));
    d.send(json!({"cmd":"restore"}));
    let restored = d.wait_for(|v| v["event"] == "restored", Duration::from_secs(10));
    assert_eq!(
        restored["ok"], false,
        "a corrupt restore baseline must fail closed: {restored:?}"
    );
    d.send(json!({"cmd":"shutdown"}));
    d.wait_for(|v| v["event"] == "bye", Duration::from_secs(5));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn doctor_reports_valid_json() {
    let dir = tmp("doctor");
    let bin = env!("CARGO_BIN_EXE_surya-prof");
    let out = Command::new(bin)
        .args(["doctor", "--state-dir", dir.to_str().unwrap()])
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).expect("doctor output is JSON");
    assert!(v["ok"].is_boolean());
    let checks = v["checks"].as_array().expect("checks array");
    assert!(checks.iter().any(|c| c["name"] == "catalog"));
    assert!(checks.iter().any(|c| c["name"] == "profiles"));
    let _ = std::fs::remove_dir_all(&dir);
}

// ===== v0.11: reconcile + auto-revive watchdog =====

/// Write the fake tree nodes used by the reconcile E2E. Values start as a
/// realistic stale mix (a leftover from an old game/boost apply) so the first
/// forced apply must heal every union key.
fn reconciled_tree(dir: &std::path::Path) -> std::path::PathBuf {
    let root = dir.join("fake-root");
    let w = |rel: &str, body: &str| {
        let p = root.join(rel.trim_start_matches('/'));
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    };
    let stock: &[(&str, &str)] = &[
        // cpusets (capacity-wise identical to stock MIUI)
        ("/dev/cpuset/background/cpus", "0-5"),
        ("/dev/cpuset/foreground/cpus", "0-7"),
        ("/dev/cpuset/system-background/cpus", "0-5"),
        ("/dev/cpuset/top-app/cpus", "0-7"),
        // l3-lat floors
        (
            "/sys/class/devfreq/soc:qcom,cpu0-cpu-l3-lat/min_freq",
            "300000000",
        ),
        (
            "/sys/class/devfreq/soc:qcom,cpu6-cpu-l3-lat/min_freq",
            "300000000",
        ),
        // GPU levels
        ("/sys/class/kgsl/kgsl-3d0/min_pwrlevel", "6"),
        ("/sys/class/kgsl/kgsl-3d0/max_pwrlevel", "0"),
        ("sys/class/kgsl/kgsl-3d0/min_pwrlevel", "6"),
        ("sys/class/kgsl/kgsl-3d0/max_pwrlevel", "0"),
        // io
        ("sys/block/sda/queue/iostats", "1"),
        ("sys/block/sda/queue/nomerges", "0"),
        ("sys/block/sda/queue/nr_requests", "128"),
        ("sys/block/sda/queue/read_ahead_kb", "512"),
        ("sys/block/sda/queue/scheduler", "noop deadline [cfq]"),
        // kernel sysctls (stale mix: leftover up/downmigrate + pl)
        ("proc/sys/kernel/sched_conservative_pl", "1"),
        ("proc/sys/kernel/sched_downmigrate", "50"),
        ("proc/sys/kernel/sched_latency_ns", "6000000"),
        ("proc/sys/kernel/sched_migration_cost_ns", "500000"),
        ("proc/sys/kernel/sched_min_granularity_ns", "1000000"),
        ("proc/sys/kernel/sched_min_task_util_for_boost", "51"),
        ("proc/sys/kernel/sched_min_task_util_for_colocation", "35"),
        ("proc/sys/kernel/sched_nr_migrate", "8"),
        ("proc/sys/kernel/sched_upmigrate", "60"),
        ("proc/sys/kernel/sched_wakeup_granularity_ns", "500000"),
        // policy0 (cpubw shares cluster cpus, stale mix too)
        ("sys/devices/system/cpu/cpu0/core_ctl/max_cpus", "6"),
        ("sys/devices/system/cpu/cpu0/core_ctl/min_cpus", "6"),
        ("sys/devices/system/cpu/cpu0/core_ctl/task_thres", "12"),
        (
            "sys/devices/system/cpu/cpufreq/policy0/scaling_governor",
            "schedutil",
        ),
        (
            "sys/devices/system/cpu/cpufreq/policy0/scaling_max_freq",
            "1248000",
        ),
        (
            "sys/devices/system/cpu/cpufreq/policy0/scaling_min_freq",
            "768000",
        ),
        (
            "sys/devices/system/cpu/cpufreq/policy0/schedutil/hispeed_freq",
            "1497600",
        ),
        (
            "sys/devices/system/cpu/cpufreq/policy0/schedutil/hispeed_load",
            "75",
        ),
        (
            "sys/devices/system/cpu/cpufreq/policy0/schedutil/up_rate_limit_us",
            "0",
        ),
        (
            "sys/devices/system/cpu/cpufreq/policy0/schedutil/down_rate_limit_us",
            "0",
        ),
        ("sys/devices/system/cpu/cpufreq/policy0/schedutil/pl", "1"),
        // policy6 (stale legacy)
        ("sys/devices/system/cpu/cpu6/core_ctl/min_cpus", "1"),
        (
            "sys/devices/system/cpu/cpufreq/policy6/scaling_governor",
            "schedutil",
        ),
        (
            "sys/devices/system/cpu/cpufreq/policy6/scaling_max_freq",
            "1555200",
        ),
        (
            "sys/devices/system/cpu/cpufreq/policy6/scaling_min_freq",
            "1094400",
        ),
        (
            "sys/devices/system/cpu/cpufreq/policy6/schedutil/hispeed_freq",
            "1555200",
        ),
        (
            "sys/devices/system/cpu/cpufreq/policy6/schedutil/hispeed_load",
            "85",
        ),
        (
            "sys/devices/system/cpu/cpufreq/policy6/schedutil/up_rate_limit_us",
            "0",
        ),
        (
            "sys/devices/system/cpu/cpufreq/policy6/schedutil/down_rate_limit_us",
            "0",
        ),
        ("sys/devices/system/cpu/cpufreq/policy6/schedutil/pl", "1"),
        // stune
        ("/dev/stune/top-app/boost", "0"),
        // probe options: OPP / governor / cluster lists + related cpus
        (
            "sys/devices/system/cpu/cpufreq/policy0/scaling_available_governors",
            "powersave schedutil",
        ),
        (
            "sys/devices/system/cpu/cpufreq/policy0/scaling_available_frequencies",
            "300000 576000 768000 1017600 1248000 1324800 1497600 1612800 1804800",
        ),
        (
            "sys/devices/system/cpu/cpufreq/policy0/related_cpus",
            "0 1 2 3 4 5",
        ),
        (
            "sys/devices/system/cpu/cpufreq/policy6/scaling_available_governors",
            "powersave schedutil",
        ),
        (
            "sys/devices/system/cpu/cpufreq/policy6/scaling_available_frequencies",
            "300000 652800 806400 979200 1094400 1209600 1324800 1555200 1708800 2304000",
        ),
        ("sys/devices/system/cpu/cpufreq/policy6/related_cpus", "6 7"),
        (
            "sys/class/kgsl/kgsl-3d0/devfreq/available_governors",
            "simple_ondemand msm-adreno-tz powersave",
        ),
        (
            "sys/class/kgsl/kgsl-3d0/gpu_available_frequencies",
            "180000000 266000000 305000000 380000000 430000000 575000000 825000000",
        ),
        (
            "proc/sys/net/ipv4/tcp_available_congestion_control",
            "cubic reno",
        ),
        // vm
        ("proc/sys/vm/dirty_background_ratio", "10"),
        ("proc/sys/vm/dirty_expire_centisecs", "600"),
        ("proc/sys/vm/dirty_ratio", "30"),
        ("proc/sys/vm/dirty_writeback_centisecs", "1500"),
        ("proc/sys/vm/stat_interval", "3"),
        ("proc/sys/vm/vfs_cache_pressure", "50"),
        // v0.12 round-2 keys (stale MIUI leftovers to be reconciled)
        (
            "sys/devices/system/cpu/cpu0/core_ctl/busy_up_thres",
            "60 60 60 60 60 60",
        ),
        (
            "sys/devices/system/cpu/cpu0/core_ctl/busy_down_thres",
            "40 40 40 40 40 40",
        ),
        (
            "sys/devices/system/cpu/cpu0/core_ctl/offline_delay_ms",
            "100",
        ),
        ("sys/block/sda/queue/rq_affinity", "1"),
        ("sys/block/sda/queue/iosched/fifo_expire_async", "250"),
        ("sys/block/sda/queue/iosched/slice_async", "40"),
        ("sys/block/sda/queue/iosched/low_latency", "1"),
        ("proc/sys/vm/watermark_scale_factor", "1"),
        ("sys/class/kgsl/kgsl-3d0/default_pwrlevel", "6"),
        ("proc/sys/kernel/sched_group_upmigrate", "100"),
        ("proc/sys/kernel/sched_group_downmigrate", "85"),
        (
            "proc/sys/kernel/sched_little_cluster_coloc_fmin_khz",
            "740000",
        ),
        ("proc/sys/kernel/sched_walt_rotate_big_tasks", "1"),
        ("proc/sys/net/ipv4/tcp_slow_start_after_idle", "1"),
    ];
    for (rel, body) in stock {
        if *rel == "ext4" {
            continue;
        }
        w(rel, body);
    }
    // stune boost file direct
    w("/dev/stune/top-app/boost", "0");
    root
}

fn read_tree_node(root: &std::path::Path, rel: &str) -> String {
    std::fs::read_to_string(root.join(rel))
        .unwrap_or_else(|_| "<missing>".into())
        .trim()
        .to_string()
}

#[test]
fn watchdog_and_force_apply_reconcile_every_profile_key() {
    let dir = tmp("tok");
    let cfg_path = dir.join("config.json");
    std::fs::write(
        &cfg_path,
        r#"{"schema":2,"enabled":true,"dynamic":true,"base_profile":"balance"}"#,
    )
    .unwrap();
    let root = reconciled_tree(&dir);

    let sysfs_root = root.to_str().unwrap().to_owned();
    let envs: Vec<(&str, &str)> = vec![
        ("SURYAPROFILER_SYSFS_ROOT", &sysfs_root),
        ("SURYAPROFILER_WATCHDOG_SECS", "2"),
        ("SURYAPROFILER_ENV_SAMPLE_MS", "1000"),
    ];
    let mut d = Daemon::spawn_env(&dir, &cfg_path, &envs);
    d.send(serde_json::json!({"cmd": "hello"}));
    d.wait_for(|v| v["event"] == "hello", Duration::from_secs(5));

    // screen on + whatsapp -> balance (first apply of the run = force)
    d.send(serde_json::json!({"cmd": "screen", "on": true, "locked": false}));
    d.send(serde_json::json!({"cmd": "fg", "pkg": "app.balance"}));
    d.wait_for(
        |v| v["event"] == "applied" && v["profile"] == "balance",
        Duration::from_secs(10),
    );

    // forced apply side effects: every PROFILE key reaches the profile value
    assert_eq!(
        read_tree_node(
            &root,
            "sys/devices/system/cpu/cpufreq/policy0/scaling_min_freq"
        ),
        "576000"
    );
    assert_eq!(
        read_tree_node(
            &root,
            "sys/devices/system/cpu/cpufreq/policy6/scaling_min_freq"
        ),
        "652800"
    );
    assert_eq!(
        read_tree_node(&root, "sys/devices/system/cpu/cpu0/core_ctl/task_thres"),
        "8"
    );
    // v0.12 round-2 keys: balance identity reaches the node
    assert_eq!(
        read_tree_node(&root, "sys/devices/system/cpu/cpu0/core_ctl/busy_up_thres"),
        "55"
    );
    assert_eq!(
        read_tree_node(
            &root,
            "sys/devices/system/cpu/cpu0/core_ctl/busy_down_thres"
        ),
        "35"
    );
    assert_eq!(
        read_tree_node(&root, "sys/block/sda/queue/rq_affinity"),
        "2"
    );
    // v0.16 balance no longer touches tcp_slow_start_after_idle: the union
    // stock value (1 = stock) comes back through the reconcile.
    assert_eq!(
        read_tree_node(&root, "proc/sys/net/ipv4/tcp_slow_start_after_idle"),
        "1"
    );
    // keys balance does not set stay on the captured union stock
    assert_eq!(
        read_tree_node(
            &root,
            "sys/devices/system/cpu/cpu0/core_ctl/offline_delay_ms"
        ),
        "120"
    );
    assert_eq!(
        read_tree_node(&root, "sys/block/sda/queue/iosched/fifo_expire_async"),
        "250"
    );
    // union keys that balance doesn't set get stock back (vm.pl)
    assert_eq!(
        read_tree_node(&root, "proc/sys/kernel/sched_conservative_pl"),
        "0"
    );
    // cpu6 core_ctl stays untouched (not in any profile params)
    assert_eq!(
        read_tree_node(&root, "sys/devices/system/cpu/cpu6/core_ctl/min_cpus"),
        "1"
    );

    // external drift now: MIUI/interference pokes a stale value
    std::fs::write(
        root.join("sys/devices/system/cpu/cpufreq/policy6/scaling_min_freq"),
        "1094400\n",
    )
    .unwrap();

    // watchdog fires within ~3s (2s interval >= one 3s supervise pass)
    d.wait_for(
        |v| {
            v["event"] == "applied"
                && v["profile"] == "balance"
                && v["wrote"].as_u64().unwrap_or(0) >= 1
        },
        Duration::from_secs(10),
    );
    assert_eq!(
        read_tree_node(
            &root,
            "sys/devices/system/cpu/cpufreq/policy6/scaling_min_freq"
        ),
        "652800",
        "watchdog reconcile must heal the drifted freq floor"
    );
    // the auto-revive counter records the heal (stats reply carries it)
    d.send(serde_json::json!({"cmd": "stats"}));
    let st = d.wait_for(|v| v["event"] == "stats", Duration::from_secs(5));
    assert!(
        st["heals_total"].as_u64().unwrap_or(0) >= 1,
        "watchdog heal must be counted: {st:?}"
    );
    assert!(st["heals_last_keys"].as_u64().unwrap_or(0) >= 1);
    d.send(serde_json::json!({"cmd": "shutdown"}));
    d.wait_for(|v| v["event"] == "bye", Duration::from_secs(5));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn thermal_takeover_keeps_custom_over_miui_video_and_yields_to_camera() {
    let dir = tmp("thermal-policy");
    let cfg_path = dir.join("config.json");
    std::fs::write(
        &cfg_path,
        r#"{"schema":2,"enabled":true,"dynamic":true,"custom_thermal":true,
            "base_profile":"balance"}"#,
    )
    .unwrap();

    let root = dir.join("fake-root");
    let w = |rel: &str, body: &str| {
        let p = root.join(rel.trim_start_matches('/'));
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    };
    w("sys/class/thermal/thermal_message/sconfig", "0\n");
    w("data/vendor/thermal/thermal-global-mode", "0\n");
    w("data/property/persist.sys.thermal.config", "0\n");
    // MIUI vendor map blob (encrypted, same format as /vendor/etc).
    let vendor_map = root.join("vendor/etc/thermal-map.conf");
    std::fs::create_dir_all(vendor_map.parent().unwrap()).unwrap();
    std::fs::write(
        &vendor_map,
        crypt::encrypt(
            b"[0:thermal-normal.conf]\n[8:thermal-phone.conf]\n[12:thermal-camera.conf]\n",
        ),
    )
    .unwrap();

    let mut d = Daemon::spawn_env(
        &dir,
        &cfg_path,
        &[("SURYAPROFILER_SYSFS_ROOT", root.to_str().unwrap())],
    );
    d.send(json!({"cmd":"hello"}));
    d.wait_for(|v| v["event"] == "hello", Duration::from_secs(5));
    d.send(json!({"cmd":"screen","on":true,"locked":false}));
    d.send(json!({"cmd":"fg","pkg":"com.whatsapp"}));
    d.wait_for(
        |v| v["event"] == "applied" && v["profile"] == "balance",
        Duration::from_secs(10),
    );

    let sconfig = || {
        std::fs::read_to_string(root.join("sys/class/thermal/thermal_message/sconfig"))
            .unwrap_or_default()
            .trim()
            .to_string()
    };
    let map_path = root.join("data/vendor/thermal/config/thermal-map.conf");
    for _ in 0..50 {
        if sconfig() == "60" && map_path.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(sconfig(), "60", "balance custom index must be selected");
    assert!(map_path.exists(), "custom map must be installed");
    let marker = root.join("data/vendor/thermal/config/.suryaprofiler");
    assert!(marker.exists());
    // Wait for the startup sync to finish before injecting MIUI values; a
    // late select_profile would otherwise overwrite our 21/12 test value.
    wait_thermal_profile(&mut d, "balance", Duration::from_secs(5));

    // MIUI video (YouTube is mapped to 21 in PowerKeeper): keep custom.
    w("sys/class/thermal/thermal_message/sconfig", "21\n");
    assert_eq!(
        sconfig(),
        "21",
        "test write must land before config_changed"
    );
    d.send(json!({"cmd":"config_changed"}));
    wait_log(
        &dir.join("daemon-stderr.log"),
        "keeping custom over MIUI video",
        Duration::from_secs(5),
    );
    for _ in 0..50 {
        if sconfig() == "60" {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(sconfig(), "60", "video must keep the custom profile");
    std::thread::sleep(Duration::from_millis(200));

    // Camera: yield to MIUI and leave its sconfig alone.
    w("sys/class/thermal/thermal_message/sconfig", "12\n");
    d.send(json!({"cmd":"config_changed"}));
    wait_log(
        &dir.join("daemon-stderr.log"),
        "yielding to MIUI camera",
        Duration::from_secs(5),
    );
    assert_eq!(sconfig(), "12", "camera must be yielded to MIUI");
    std::thread::sleep(Duration::from_millis(200));

    // Back to default: re-assert balance.
    w("sys/class/thermal/thermal_message/sconfig", "0\n");
    d.send(json!({"cmd":"config_changed"}));
    for _ in 0..50 {
        if sconfig() == "60" {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(sconfig(), "60", "default must re-assert custom");
    std::thread::sleep(Duration::from_millis(200));

    // MIUI performance mode wins even when sconfig says default.
    w("data/vendor/thermal/thermal-global-mode", "1\n");
    w("sys/class/thermal/thermal_message/sconfig", "0\n");
    d.send(json!({"cmd":"config_changed"}));
    wait_log(
        &dir.join("daemon-stderr.log"),
        "yielding to MIUI performance mode",
        Duration::from_secs(5),
    );
    assert_eq!(
        sconfig(),
        "1",
        "performance mode must release our custom index to sconfig 1"
    );
    w("data/vendor/thermal/thermal-global-mode", "0\n");
    // Deliberately leave sconfig at 1: MIUI's performance *scenario* is
    // stale once the performance-mode flag is off. The daemon must reassert
    // the custom profile instead of yielding forever.
    d.send(json!({"cmd":"config_changed"}));
    for _ in 0..50 {
        if sconfig() == "60" {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(
        sconfig(),
        "60",
        "stale sconfig=1 after performance mode off must reassert custom"
    );

    // Service OFF: remove marker-owned files and force stock normal.
    d.send(json!({"cmd":"restore"}));
    d.wait_for(|v| v["event"] == "restored", Duration::from_secs(10));
    // The fake tree has no stock.json, so the engine restore itself reports
    // ok=false; the thermal hold must still be released (release_holds runs
    // before the engine restore).
    assert_eq!(sconfig(), "0", "service off must restore stock normal");
    assert!(!marker.exists(), "marker must be removed on service off");
    assert!(
        !map_path.exists(),
        "custom map must be removed on service off"
    );

    d.send(json!({"cmd":"shutdown"}));
    d.wait_for(|v| v["event"] == "bye", Duration::from_secs(5));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn clean_releases_input_suspend_hold_without_a_daemon() {
    let dir = tmp("clean-holds");
    let root = dir.join("fake-root");
    let node = root.join("sys/class/power_supply/battery/input_suspend");
    std::fs::create_dir_all(node.parent().unwrap()).unwrap();
    std::fs::write(&node, "1\n").unwrap();
    std::fs::write(
        dir.join("holds.json"),
        r#"{"saver_held":false,"saver_saved":false,
            "input_suspend_held":true,"input_suspend_saved":"0",
            "thermal_held":false,"thermal_saved_sconfig":null,
            "thermal_profile":null,"thermal_yield":null}"#,
    )
    .unwrap();

    let out = std::process::Command::new(env!("CARGO_BIN_EXE_surya-prof"))
        .args(["clean", "--state-dir", dir.to_str().unwrap()])
        .env("SURYAPROFILER_DEBUG", "1")
        .env("SURYAPROFILER_SYSFS_ROOT", &root)
        .output()
        .expect("run clean");
    assert!(
        out.status.success(),
        "clean failed: {}\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    let v: Value = serde_json::from_slice(&out.stdout).expect("clean JSON");
    assert_eq!(v["ok"], true, "{v:?}");
    assert_eq!(v["input_suspend_ok"], true, "{v:?}");
    assert_eq!(
        std::fs::read_to_string(&node).unwrap().trim(),
        "0",
        "input_suspend must be restored without a daemon"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
