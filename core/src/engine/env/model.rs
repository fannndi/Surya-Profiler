//! Environment telemetry DTO.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EnvSnapshot {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_pct: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charging: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_temp_c: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_temp_c: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_temp_c: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_busy_pct: Option<u8>,
    /// Panel frame rate from the read-only `measured_fps` node (DRM CRTC).
    /// Read-only telemetry: SuryaProfiler never writes it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_fps: Option<f32>,
    /// Battery current in µA (read-only FG node): negative = charging,
    /// positive = discharging. Sign verified 2026-10-09 with a controlled
    /// R&D `input_suspend` test (suspend: +600 mA; restore: negative).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_current_ua: Option<i64>,
    /// Battery voltage in µV (~4 100 000 near full charge).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_voltage_uv: Option<u32>,
    /// Little/policy0 and big/policy6 current CPU frequency in MHz
    /// (`cpuinfo_cur_freq` is kHz and 0400 root-only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub little_freq_mhz: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub big_freq_mhz: Option<u32>,
    /// GPU current clock in MHz (`gpuclk` reports Hz on this kernel).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_freq_mhz: Option<u32>,
    /// F2FS userdata lifetime write counter in KB (read-only, updates lazily).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_written_kb: Option<u64>,
    /// Battery full-charge capacity in mAh (`charge_full` reports µAh) —
    /// battery-health readout; a broken/negative node stays absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charge_full_mah: Option<u32>,
    /// Battery cycle count (`cycle_count`; absent on some fuel gauges).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cycle_count: Option<u32>,
    /// MIUI thermal scenario selector (`thermal_message/sconfig`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thermal_sconfig: Option<i32>,
    /// Resolved config name from the ROM's `thermal-map.conf`; unknown
    /// values stay absent (the daemon never guesses).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thermal_mode: Option<String>,
    /// Full MIUI scenario label decoded from PowerKeeper's `THERMAL_ID_*`
    /// table and its offsets (`video + foreground`, `camera-4k`, ...).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thermal_scenario: Option<String>,
    /// MIUI performance-mode flag (`/data/vendor/thermal/thermal-global-mode`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thermal_global_mode: Option<i32>,
    /// MIUI performance-mode property (`persist.sys.thermal.config`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thermal_perf_config: Option<i32>,
    /// MIUI thermal severity state (`thermal_message/temp_state`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thermal_temp_state: Option<i32>,
    /// MIUI screen state mirrored into thermal_message (0 off, 1 on).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thermal_screen_state: Option<i32>,
    /// Quiet-therm skin temperature (battery/LCD monitor in `thermal-normal`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quiet_temp_c: Option<f32>,
    /// NVM thermistor virtual sensor used by the CPU SS algorithm.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nvm_temp_c: Option<f32>,
    /// CPU6/CPU7 cpufreq cooling states (0 = no cap; higher = more capped).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu6_cooling: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu6_cooling_max: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu7_cooling: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu7_cooling_max: Option<i32>,
    /// Effective policy max frequencies after thermal/perf clamping (MHz).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub little_max_mhz: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub big_max_mhz: Option<u32>,
}
