//! Environment sampler tests.

use super::*;

fn tmpdir(tag: &str) -> PathBuf {
    let mut d = std::env::temp_dir();
    d.push(format!("suryaprofiler-env-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn fake_tree(tag: &str) -> PathBuf {
    let root = tmpdir(tag);
    let w = |rel: &str, body: &str| {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    };
    w("sys/class/power_supply/battery/capacity", "95\n");
    w("sys/class/power_supply/battery/status", "Charging\n");
    w("sys/class/power_supply/battery/temp", "320\n");
    w("sys/class/thermal/thermal_zone0/type", "pm6150-tz\n");
    w("sys/class/thermal/thermal_zone0/temp", "37000\n");
    w("sys/class/thermal/thermal_zone24/type", "cpuss-0-usr\n");
    w("sys/class/thermal/thermal_zone24/temp", "38800\n");
    w("sys/class/thermal/thermal_zone30/type", "gpuss-0-usr\n");
    w("sys/class/thermal/thermal_zone30/temp", "41200\n");
    w("sys/class/kgsl/kgsl-3d0/gpu_busy_percentage", "3 %\n");
    w(
        "sys/class/drm/sde-crtc-0/measured_fps",
        "fps: 75.1 duration:1000000 frame_count:97\n",
    );
    w("sys/class/power_supply/battery/current_now", "-580123\n");
    w("sys/class/power_supply/battery/voltage_now", "3985000\n");
    w(
        "sys/devices/system/cpu/cpufreq/policy0/cpuinfo_cur_freq",
        "1804800\n",
    );
    w(
        "sys/devices/system/cpu/cpufreq/policy6/cpuinfo_cur_freq",
        "2304000\n",
    );
    w("sys/class/kgsl/kgsl-3d0/gpuclk", "430000000\n");
    w("sys/fs/f2fs/sda16/lifetime_write_kbytes", "53193232\n");
    w("sys/class/power_supply/battery/charge_full", "5008000\n");
    w("sys/class/power_supply/battery/cycle_count", "562\n");
    // MIUI thermal telemetry (ROM thermal-map.conf scenario ids).
    w("sys/class/thermal/thermal_message/sconfig", "0\n");
    w("sys/class/thermal/thermal_message/temp_state", "0\n");
    w("sys/class/thermal/thermal_message/screen_state", "1\n");
    w("data/vendor/thermal/thermal-global-mode", "0\n");
    w("data/property/persist.sys.thermal.config", "0\n");
    w("sys/class/thermal/thermal_zone45/type", "quiet-therm-adc\n");
    w("sys/class/thermal/thermal_zone45/temp", "45100\n");
    w("sys/class/thermal/thermal_zone48/type", "nvm-therm-adc\n");
    w("sys/class/thermal/thermal_zone48/temp", "48200\n");
    w(
        "sys/class/thermal/cooling_device8/type",
        "thermal-cpufreq-6\n",
    );
    w("sys/class/thermal/cooling_device8/cur_state", "8\n");
    w("sys/class/thermal/cooling_device8/max_state", "14\n");
    w(
        "sys/class/thermal/cooling_device9/type",
        "thermal-cpufreq-7\n",
    );
    w("sys/class/thermal/cooling_device9/cur_state", "0\n");
    w("sys/class/thermal/cooling_device9/max_state", "14\n");
    w(
        "sys/devices/system/cpu/cpufreq/policy0/scaling_max_freq",
        "1708800\n",
    );
    w(
        "sys/devices/system/cpu/cpufreq/policy6/scaling_max_freq",
        "1209600\n",
    );
    w(
        "sys/devices/system/cpu/cpufreq/policy0/scaling_min_freq",
        "1612800\n",
    );
    w(
        "sys/devices/system/cpu/cpufreq/policy6/scaling_min_freq",
        "1843200\n",
    );
    w(
        "sys/devices/system/cpu/cpufreq/policy0/scaling_governor",
        "schedutil\n",
    );
    root
}

#[test]
fn parsers_accept_device_formats() {
    assert_eq!(parse_pct("95\n"), Some(95));
    assert_eq!(parse_pct("101"), Some(100));
    assert_eq!(parse_pct("-5"), Some(0));
    assert_eq!(parse_pct("abc"), None);

    assert_eq!(parse_temp_tenths("320"), Some(32.0));
    assert_eq!(parse_temp_tenths("0"), None);
    assert_eq!(parse_temp_tenths("-100"), None);

    assert_eq!(parse_temp_milli("38800"), Some(38.8));
    assert_eq!(
        parse_temp_milli("-312"),
        None,
        "level zones are not temperatures"
    );

    assert_eq!(parse_charging("Charging"), Some(true));
    assert_eq!(parse_charging("Full"), Some(true));
    assert_eq!(parse_charging("Discharging"), Some(false));
    assert_eq!(parse_charging("Not charging"), Some(false));
    assert_eq!(parse_charging("Unknown"), None);

    assert_eq!(parse_gpu_busy("3 %"), Some(3));
    assert_eq!(parse_gpu_busy("100"), Some(100));
    assert_eq!(parse_gpu_busy("bogus"), None);

    assert_eq!(
        parse_measured_fps("fps: 75.1 duration:1000000 frame_count:97"),
        Some(75.1)
    );
    assert_eq!(parse_measured_fps("fps: 0.0 duration:1000000"), Some(0.0));
    assert_eq!(parse_measured_fps("garbage"), None);

    assert_eq!(first_i32("9\n"), Some(9));
    assert_eq!(first_i32("-1"), Some(-1));
    assert_eq!(first_i32("garbage"), None);
    assert_eq!(first_i64(" -121765\n"), Some(-121765));
    assert_eq!(first_i64("garbage"), None);
    assert_eq!(first_u32("4105283\n"), Some(4105283));
    assert_eq!(first_u64("53193232\n"), Some(53193232));
    assert_eq!(uah_to_mah("5008000\n"), Some(5008));
    assert_eq!(uah_to_mah("-203568155"), None, "broken FG stays absent");
    assert_eq!(uah_to_mah("0"), None);
    assert_eq!(uah_to_mah("garbage"), None);
    assert_eq!(khz_to_mhz("1804800\n"), Some(1804));
    assert_eq!(khz_to_mhz("768000"), Some(768));
    assert_eq!(hz_to_mhz("430000000\n"), Some(430));
    assert_eq!(hz_to_mhz("garbage"), None);
}

#[test]
fn thermal_mode_map_matches_rom() {
    // Verified on surya 2026-11-20 against thermal-map.conf + decrypt.txt.
    for v in [0, 1, 2, 11, 14] {
        assert_eq!(thermal_mode_name(v), Some("normal"), "sconfig {v}");
    }
    assert_eq!(thermal_mode_name(8), Some("phone"));
    for v in [9, 13, 16] {
        assert_eq!(thermal_mode_name(v), Some("tgame"), "sconfig {v}");
    }
    assert_eq!(thermal_mode_name(10), Some("nolimits"));
    assert_eq!(thermal_mode_name(12), Some("camera"));
    assert_eq!(thermal_mode_name(15), Some("arvr"));
    assert_eq!(thermal_mode_name(60), Some("custom-balance"));
    assert_eq!(thermal_mode_name(61), Some("custom-game"));
    assert_eq!(thermal_mode_name(62), Some("custom-powersave"));
    assert_eq!(thermal_mode_name(63), Some("custom-sleep"));
    assert_eq!(thermal_mode_name(20), None, "MIUI video, not custom");
    assert_eq!(thermal_mode_name(21), None, "MIUI video, not custom");
    assert_eq!(thermal_mode_name(3), None);
    assert_eq!(thermal_mode_name(-1), None);
    assert_eq!(thermal_scenario_label(0), "default");
    assert_eq!(thermal_scenario_label(12), "camera");
    assert_eq!(thermal_scenario_label(21), "video");
    assert_eq!(thermal_scenario_label(121), "video + foreground");
    assert_eq!(thermal_scenario_label(60), "custom-balance");
    assert_eq!(thermal_scenario_label(999), "unknown(999)");
}

#[test]
fn sampler_reads_a_fake_tree() {
    let root = fake_tree("sample");
    let s = Sampler::new(&root).sample();
    assert_eq!(s.battery_pct, Some(95));
    assert_eq!(s.charging, Some(true));
    assert_eq!(s.battery_temp_c, Some(32.0));
    assert_eq!(
        s.cpu_temp_c,
        Some(38.8),
        "cpuss-0-usr must win over pm6150-tz"
    );
    assert_eq!(s.gpu_temp_c, Some(41.2));
    assert_eq!(s.gpu_busy_pct, Some(3));
    assert_eq!(s.screen_fps, Some(75.1));
    assert_eq!(s.battery_current_ua, Some(-580123));
    assert_eq!(s.battery_voltage_uv, Some(3985000));
    assert_eq!(s.little_freq_mhz, Some(1804));
    assert_eq!(s.big_freq_mhz, Some(2304));
    assert_eq!(s.gpu_freq_mhz, Some(430));
    assert_eq!(s.storage_written_kb, Some(53193232));
    assert_eq!(s.charge_full_mah, Some(5008));
    assert_eq!(s.cycle_count, Some(562));
    assert_eq!(s.thermal_sconfig, Some(0));
    assert_eq!(s.thermal_mode.as_deref(), Some("normal"));
    assert_eq!(s.thermal_scenario.as_deref(), Some("default"));
    assert_eq!(s.thermal_global_mode, Some(0));
    assert_eq!(s.thermal_perf_config, Some(0));
    assert_eq!(s.thermal_temp_state, Some(0));
    assert_eq!(s.thermal_screen_state, Some(1));
    assert_eq!(s.quiet_temp_c, Some(45.1));
    assert_eq!(s.nvm_temp_c, Some(48.2));
    assert_eq!(s.cpu6_cooling, Some(8));
    assert_eq!(s.cpu6_cooling_max, Some(14));
    assert_eq!(s.cpu7_cooling, Some(0));
    assert_eq!(s.cpu7_cooling_max, Some(14));
    assert_eq!(s.little_max_mhz, Some(1708));
    assert_eq!(s.big_max_mhz, Some(1209));
    assert_eq!(s.little_min_mhz, Some(1612), "the MIUI screen-on floor");
    assert_eq!(s.big_min_mhz, Some(1843));
    assert_eq!(s.little_governor.as_deref(), Some("schedutil"));
    assert_eq!(s.big_governor, None, "absent node stays absent");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn missing_nodes_yield_none_not_panic() {
    let root = tmpdir("empty");
    let s = Sampler::new(&root).sample();
    assert_eq!(s, EnvSnapshot::default());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn zone_discovery_falls_back_in_priority_order() {
    let root = tmpdir("zones");
    let w = |rel: &str, body: &str| {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    };
    w("sys/class/thermal/thermal_zone0/type", "pm6150-tz\n");
    w("sys/class/thermal/thermal_zone0/temp", "37000\n");
    w("sys/class/thermal/thermal_zone7/type", "cpu-0-0-usr\n");
    w("sys/class/thermal/thermal_zone7/temp", "39900\n");
    let s = Sampler::new(&root).sample();
    assert_eq!(s.cpu_temp_c, Some(39.9), "cpu-0-0-usr beats pm6150-tz");
    assert_eq!(s.gpu_temp_c, None, "no gpu zone -> absent");
    let _ = fs::remove_dir_all(&root);
}
