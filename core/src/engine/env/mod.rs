//! Read-only environment telemetry: battery, thermal zones, GPU busy.
//!
//! Everything in this file is READ-ONLY — no writes, no framework fights.
//! The daemon uses it for diagnostics (UI) and env-aware guards; `doctor`
//! uses it to verify the device exposes what the daemon needs.
//!
//! Paths are rooted at a configurable prefix (`SURYAPROFILER_SYSFS_ROOT`,
//! default `/`) so host tests and simulations can point the sampler at a
//! fake tree — the same trick `SURYAPROFILER_LOGCAT_BIN` uses for logcat.

use std::fs;
use std::path::{Path, PathBuf};

/// Env override for the sysfs prefix (host tests / simulation).
pub const SYSFS_ROOT_ENV: &str = "SURYAPROFILER_SYSFS_ROOT";

/// Preferred thermal zone types, in priority order (surya / sm6150 names).
const CPU_ZONES: &[&str] = &["cpuss-0-usr", "cpu-0-0-usr", "pm6150-tz"];
const GPU_ZONES: &[&str] = &["gpuss-0-usr", "gpuss-1-usr"];
/// MIUI thermal zones used by the ROM's `thermal-normal.conf` monitors.
const QUIET_ZONE: &str = "quiet-therm-adc";
const NVM_ZONE: &str = "nvm-therm-adc";
/// Cooling-device types for the two big-core cpufreq caps.
const CPU6_COOLING: &str = "thermal-cpufreq-6";
const CPU7_COOLING: &str = "thermal-cpufreq-7";

/// One telemetry sample. Every field is optional: a missing node must never
/// break the daemon — the field is simply absent.
mod model;
mod parsers;
mod power;
mod sampler;
mod scenario;
mod sysfs;

#[cfg(test)]
mod tests;

pub use model::EnvSnapshot;
pub use parsers::*;
pub use power::{parse_battery_status, parse_bool01, InputSupply, PowerProbe, PowerReading};
pub use sampler::Sampler;
pub use scenario::{thermal_mode_name, thermal_scenario_label};
pub use sysfs::{default_root, storage_health};
