//! Host E2E for the Power usage sampler/history: fake battery nodes,
//! fast cadence, snapshot + live events + clear.

mod common;

use common::{tmp, Daemon};
use serde_json::json;
use std::path::Path;
use std::time::Duration;

fn write_node(root: &Path, rel: &str, body: &str) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, body).unwrap();
}

fn set_battery(root: &Path, ua: i64, status: &str, pct: u8) {
    write_node(
        root,
        "sys/class/power_supply/battery/current_now",
        &format!("{ua}\n"),
    );
    write_node(
        root,
        "sys/class/power_supply/battery/status",
        &format!("{status}\n"),
    );
    write_node(
        root,
        "sys/class/power_supply/battery/capacity",
        &format!("{pct}\n"),
    );
}

#[test]
fn power_samples_stream_history_and_clear() {
    let dir = tmp("power-e2e");
    let state = dir.join("state");
    let root = dir.join("root");
    std::fs::create_dir_all(&root).unwrap();
    set_battery(&root, -1_240_000, "Charging", 78);
    write_node(
        &root,
        "sys/class/power_supply/battery/voltage_now",
        "4120000\n",
    );
    write_node(&root, "sys/class/power_supply/battery/temp", "324\n");
    write_node(
        &root,
        "sys/class/power_supply/battery/charge_full",
        "5008000\n",
    );
    write_node(&root, "sys/class/power_supply/battery/cycle_count", "34\n");
    write_node(&root, "sys/class/power_supply/battery/input_suspend", "0\n");
    write_node(
        &root,
        "sys/class/power_supply/battery/battery_charging_enabled",
        "1\n",
    );
    write_node(&root, "sys/class/power_supply/main/online", "1\n");
    write_node(
        &root,
        "sys/class/power_supply/main/voltage_now",
        "9000000\n",
    );
    write_node(
        &root,
        "sys/class/power_supply/main/current_now",
        "1500000\n",
    );

    let cfg_path = dir.join("config.json");
    std::fs::write(
        &cfg_path,
        r#"{"schema":2,"enabled":false,"dynamic":false,"base_profile":"balance"}"#,
    )
    .unwrap();

    let mut d = Daemon::spawn_env(
        &state,
        &cfg_path,
        &[
            ("SURYAPROFILER_SYSFS_ROOT", root.to_str().unwrap()),
            ("SURYAPROFILER_POWER_SAMPLE_MS", "50"),
            ("SURYAPROFILER_POWER_SAMPLE_OFF_MS", "80"),
            ("SURYAPROFILER_POWER_BUCKET_MS", "200"),
        ],
    );
    d.wait_for(|v| v["event"] == "hello", Duration::from_secs(5));

    // Snapshot request + live stream.
    d.send(json!({"cmd":"power","watch":true,"window_s":600}));
    let snap = d.wait_for(|v| v["event"] == "power", Duration::from_secs(5));
    assert_eq!(snap["snapshot"]["window_s"], 600);

    let live = d.wait_for(
        |v| v["event"] == "power_live" && v["sample"]["current_ua"].as_i64() == Some(-1_240_000),
        Duration::from_secs(5),
    );
    assert_eq!(live["sample"]["status"], "charging");
    assert_eq!(live["sample"]["voltage_uv"].as_u64(), Some(4_120_000));
    assert_eq!(live["sample"]["temp_c"].as_f64(), Some(32.4));
    assert_eq!(live["sample"]["screen_on"], true);
    assert_eq!(live["sample"]["input"]["name"], "main");
    assert_eq!(live["sample"]["input"]["online"], true);
    assert_eq!(live["sample"]["cycle_count"].as_u64(), Some(34));

    // Fresh read on the next tick (no caching).
    set_battery(&root, 800_000, "Discharging", 77);
    let live = d.wait_for(
        |v| v["event"] == "power_live" && v["sample"]["current_ua"].as_i64() == Some(800_000),
        Duration::from_secs(5),
    );
    assert_eq!(live["sample"]["status"], "discharging");
    assert!(live["bucket"].is_object(), "live carries a partial bucket");

    // Buckets close and persist.
    std::thread::sleep(Duration::from_millis(700));
    d.send(json!({"cmd":"power","watch":true,"window_s":600}));
    let snap = d.wait_for(|v| v["event"] == "power", Duration::from_secs(5));
    let history = snap["snapshot"]["history"].as_array().unwrap();
    assert!(!history.is_empty(), "bucket history must be recorded");
    assert!(history.iter().all(|b| b["samples"].as_u64().unwrap() > 0));
    assert!(
        snap["snapshot"]["peaks"]["charge"].is_object(),
        "charge peak recorded"
    );
    assert!(
        snap["snapshot"]["peaks"]["discharge"].is_object(),
        "discharge peak recorded"
    );
    assert!(state.join("power-history.jsonl").exists());

    // Screen-off cadence still records (context flag flips).
    d.send(json!({"cmd":"screen","on":false,"locked":true}));
    d.wait_for(
        |v| v["event"] == "power_live" && v["sample"]["screen_on"] == false,
        Duration::from_secs(5),
    );
    d.send(json!({"cmd":"screen","on":true,"locked":false}));

    // Clear drops history + peaks + file.
    d.send(json!({"cmd":"power_clear"}));
    let cleared = d.wait_for(|v| v["event"] == "power", Duration::from_secs(5));
    assert_eq!(cleared["snapshot"]["history"].as_array().unwrap().len(), 0);
    assert!(cleared["snapshot"]["peaks"]["charge"].is_null());
    assert!(!state.join("power-history.jsonl").exists());
    assert!(!state.join("power-peaks.json").exists());

    d.send(json!({"cmd":"shutdown"}));
    d.wait_for(|v| v["event"] == "bye", Duration::from_secs(10));
}
