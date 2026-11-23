//! thermal_log unit tests (split from thermal_log.rs).

use super::*;

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "surya-thermal-log-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn parses_battery_event() {
    let ev = parse_line("11-19 20:22:45[MONITOR-BAT][quiet-therm-adc 39545][battery 602]").unwrap();
    assert_eq!(ev.time, "11-19 20:22:45");
    assert_eq!(ev.algo, "MONITOR-BAT");
    assert_eq!(ev.sensor.as_deref(), Some("quiet-therm-adc"));
    assert_eq!(ev.sensor_value.as_deref(), Some("39545"));
    assert_eq!(ev.device.as_deref(), Some("battery"));
    assert_eq!(ev.device_value.as_deref(), Some("602"));
}

#[test]
fn parses_custom_ss_and_plus_targets() {
    let ev = parse_line("11-19 20:23:14[BAL-SS-CPU6][nvm-therm-adc 51200][cpu6 2169600]").unwrap();
    assert_eq!(ev.algo, "BAL-SS-CPU6");
    assert_eq!(ev.device_value.as_deref(), Some("2169600"));

    let ev = parse_line("11-19 20:22:45[MONITOR-BCL][bat_soc 56][hotplug_cpu0+hotplug_cpu1 1+1]")
        .unwrap();
    assert_eq!(ev.device.as_deref(), Some("hotplug_cpu0+hotplug_cpu1"));
    assert_eq!(ev.device_value.as_deref(), Some("1+1"));
}

#[test]
fn skips_malformed_lines() {
    assert!(parse_line("").is_none());
    assert!(parse_line("garbage").is_none());
    assert!(parse_line("[MONITOR-BAT][battery 1]").is_none());
}

#[test]
fn load_recent_merges_old_then_new_and_caps() {
    let root = tmp("merge");
    fs::create_dir_all(root.join("data/vendor/thermal")).unwrap();
    fs::write(
        root.join("data/vendor/thermal/last_thermal.dump"),
        "11-19 20:00:00[MONITOR-BAT][quiet-therm-adc 38000][battery 201]\n",
    )
    .unwrap();
    fs::write(
        root.join("data/vendor/thermal/thermal.dump"),
        "11-19 20:01:00[MONITOR-BAT][quiet-therm-adc 39000][battery 602]\n\
         11-19 20:02:00[MONITOR-BAT][quiet-therm-adc 40000][battery 907]\n",
    )
    .unwrap();
    let all = load_recent(&root, 10);
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].time, "11-19 20:00:00");
    assert_eq!(all[2].time, "11-19 20:02:00");
    let last = load_recent(&root, 2);
    assert_eq!(last.len(), 2);
    assert_eq!(last[0].time, "11-19 20:01:00");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn missing_files_are_empty() {
    let root = tmp("missing");
    assert!(load_recent(&root, 5).is_empty());
    let _ = fs::remove_dir_all(&root);
}
