//! Battery/thermal/GPU sampler.

use super::sysfs::{discover_zones, read_trim, CoolingPaths};
use super::*;

pub struct Sampler {
    root: PathBuf,
    cpu_temp: Option<PathBuf>,
    gpu_temp: Option<PathBuf>,
    quiet_temp: Option<PathBuf>,
    nvm_temp: Option<PathBuf>,
    cpu6_cooling: Option<CoolingPaths>,
    cpu7_cooling: Option<CoolingPaths>,
}

impl Sampler {
    pub fn new(root: &Path) -> Self {
        let zones = discover_zones(root);
        let pick = |names: &[&str]| {
            names
                .iter()
                .find_map(|n| zones.iter().find(|(t, _)| t == n).map(|(_, p)| p.clone()))
        };
        Sampler {
            root: root.to_path_buf(),
            cpu_temp: pick(CPU_ZONES),
            gpu_temp: pick(GPU_ZONES),
            quiet_temp: pick(&[QUIET_ZONE]),
            nvm_temp: pick(&[NVM_ZONE]),
            cpu6_cooling: discover_cooling(root, CPU6_COOLING),
            cpu7_cooling: discover_cooling(root, CPU7_COOLING),
        }
    }

    pub fn sample(&self) -> EnvSnapshot {
        let thermal_sconfig =
            self.read_parse("sys/class/thermal/thermal_message/sconfig", first_i32);
        EnvSnapshot {
            battery_pct: self.read_parse("sys/class/power_supply/battery/capacity", parse_pct),
            charging: self.read_parse("sys/class/power_supply/battery/status", parse_charging),
            battery_temp_c: self
                .read_parse("sys/class/power_supply/battery/temp", parse_temp_tenths),
            cpu_temp_c: self
                .cpu_temp
                .as_deref()
                .and_then(read_trim)
                .and_then(|s| parse_temp_milli(&s)),
            gpu_temp_c: self
                .gpu_temp
                .as_deref()
                .and_then(read_trim)
                .and_then(|s| parse_temp_milli(&s)),
            gpu_busy_pct: self.read_parse(
                "sys/class/kgsl/kgsl-3d0/gpu_busy_percentage",
                parse_gpu_busy,
            ),
            screen_fps: self
                .read_parse("sys/class/drm/sde-crtc-0/measured_fps", parse_measured_fps),
            battery_current_ua: self
                .read_parse("sys/class/power_supply/battery/current_now", first_i64),
            battery_voltage_uv: self
                .read_parse("sys/class/power_supply/battery/voltage_now", first_u32),
            little_freq_mhz: self.read_parse(
                "sys/devices/system/cpu/cpufreq/policy0/cpuinfo_cur_freq",
                khz_to_mhz,
            ),
            big_freq_mhz: self.read_parse(
                "sys/devices/system/cpu/cpufreq/policy6/cpuinfo_cur_freq",
                khz_to_mhz,
            ),
            gpu_freq_mhz: self.read_parse("sys/class/kgsl/kgsl-3d0/gpuclk", hz_to_mhz),
            storage_written_kb: self
                .read_parse("sys/fs/f2fs/sda16/lifetime_write_kbytes", first_u64),
            charge_full_mah: self
                .read_parse("sys/class/power_supply/battery/charge_full", uah_to_mah),
            cycle_count: self.read_parse("sys/class/power_supply/battery/cycle_count", first_u32),
            thermal_sconfig,
            thermal_mode: thermal_sconfig
                .and_then(thermal_mode_name)
                .map(str::to_string),
            thermal_scenario: thermal_sconfig.map(thermal_scenario_label),
            thermal_global_mode: self
                .read_parse("data/vendor/thermal/thermal-global-mode", first_i32),
            thermal_perf_config: self
                .read_parse("data/property/persist.sys.thermal.config", first_i32)
                .or_else(|| self.read_prop_i32("persist.sys.thermal.config")),
            thermal_temp_state: self
                .read_parse("sys/class/thermal/thermal_message/temp_state", first_i32),
            thermal_screen_state: self
                .read_parse("sys/class/thermal/thermal_message/screen_state", first_i32),
            quiet_temp_c: self
                .quiet_temp
                .as_deref()
                .and_then(read_trim)
                .and_then(|s| parse_temp_milli(&s)),
            nvm_temp_c: self
                .nvm_temp
                .as_deref()
                .and_then(read_trim)
                .and_then(|s| parse_temp_milli(&s)),
            cpu6_cooling: read_cooling(self.cpu6_cooling.as_ref(), false),
            cpu6_cooling_max: read_cooling(self.cpu6_cooling.as_ref(), true),
            cpu7_cooling: read_cooling(self.cpu7_cooling.as_ref(), false),
            cpu7_cooling_max: read_cooling(self.cpu7_cooling.as_ref(), true),
            little_max_mhz: self.read_parse(
                "sys/devices/system/cpu/cpufreq/policy0/scaling_max_freq",
                khz_to_mhz,
            ),
            big_max_mhz: self.read_parse(
                "sys/devices/system/cpu/cpufreq/policy6/scaling_max_freq",
                khz_to_mhz,
            ),
        }
    }

    fn read_parse<T>(&self, rel: &str, f: fn(&str) -> Option<T>) -> Option<T> {
        read_trim(&self.root.join(rel)).and_then(|s| f(&s))
    }

    /// Read a persistent property through `getprop` when the daemon runs on
    /// the real device. Fake host trees never spawn a process.
    fn read_prop_i32(&self, name: &str) -> Option<i32> {
        if self.root != Path::new("/") {
            return None;
        }
        let out = crate::platform::run(
            "/system/bin/getprop",
            &[name],
            std::time::Duration::from_secs(2),
        )?;
        if !out.success() {
            return None;
        }
        out.stdout_lossy().trim().parse().ok()
    }
}

/// Reads a cooling device's current or maximum state.
fn read_cooling(paths: Option<&CoolingPaths>, max: bool) -> Option<i32> {
    let paths = paths?;
    let path = if max { &paths.max } else { &paths.cur };
    read_trim(path).and_then(|s| first_i32(&s))
}

/// Best cooling-device match for a cpufreq cooling type.
fn discover_cooling(root: &Path, type_name: &str) -> Option<CoolingPaths> {
    let base = root.join("sys/class/thermal");
    let rd = fs::read_dir(&base).ok()?;
    for e in rd.flatten() {
        let p = e.path();
        let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.starts_with("cooling_device") {
            continue;
        }
        if read_trim(&p.join("type")).as_deref() == Some(type_name) {
            return Some(CoolingPaths {
                cur: p.join("cur_state"),
                max: p.join("max_state"),
            });
        }
    }
    None
}
