//! Environment telemetry checks.

use super::*;
use crate::engine::env::EnvSnapshot;

pub(crate) fn run(r: &mut Report, env: &EnvSnapshot) {
    r.check(
        "env_battery",
        env.battery_pct.is_some() && env.charging.is_some(),
        false,
        json!({ "pct": env.battery_pct, "charging": env.charging, "temp_c": env.battery_temp_c }),
    );
    // BatteryService can diverge from the fuel gauge after R&D overrides
    // (`dumpsys battery unplug` / `set ac`); Diagnostics reads sysfs, the
    // status bar reads BatteryService. Flag "UPDATES STOPPED" explicitly.
    let (battery_ok, battery_detail) = battery_service_check(env.battery_pct);
    r.check("battery_service", battery_ok, false, battery_detail);
    r.check(
        "env_thermal",
        env.cpu_temp_c.is_some(),
        false,
        json!({ "cpu_c": env.cpu_temp_c, "gpu_c": env.gpu_temp_c }),
    );
    r.check(
        "env_gpu",
        env.gpu_busy_pct.is_some(),
        false,
        json!({ "busy_pct": env.gpu_busy_pct }),
    );
    r.check(
        "env_power",
        env.battery_current_ua.is_some() && env.battery_voltage_uv.is_some(),
        false,
        json!({ "ua": env.battery_current_ua, "uv": env.battery_voltage_uv }),
    );
    r.check(
        "env_freq",
        env.little_freq_mhz.is_some() && env.big_freq_mhz.is_some() && env.gpu_freq_mhz.is_some(),
        false,
        json!({ "little_mhz": env.little_freq_mhz, "big_mhz": env.big_freq_mhz, "gpu_mhz": env.gpu_freq_mhz }),
    );
    r.check(
        "env_thermal_config",
        env.thermal_sconfig.is_some()
            && (env.thermal_mode.is_some() || env.thermal_scenario.is_some()),
        false,
        json!({
            "sconfig": env.thermal_sconfig,
            "mode": env.thermal_mode.as_deref(),
            "scenario": env.thermal_scenario.as_deref(),
            "global_mode": env.thermal_global_mode,
            "perf_config": env.thermal_perf_config,
            "temp_state": env.thermal_temp_state,
            "screen_state": env.thermal_screen_state,
        }),
    );
    r.check(
        "env_thermal_caps",
        env.little_max_mhz.is_some() && env.big_max_mhz.is_some(),
        false,
        json!({
            "little_mhz": env.little_max_mhz,
            "big_mhz": env.big_max_mhz,
            "cpu6_cooling": env.cpu6_cooling,
            "cpu7_cooling": env.cpu7_cooling,
        }),
    );
}
