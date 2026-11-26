//! Wire/DTOs + constants for the Power usage history.

use crate::engine::env::{PowerProbe, PowerReading};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub const HISTORY_FILE: &str = "power-history.jsonl";
pub const PEAKS_FILE: &str = "power-peaks.json";
pub const PEAKS_SCHEMA: u32 = 1;

/// Default snapshot window (24 h) and the hard bounds accepted from the app.
pub const DEFAULT_WINDOW_S: u64 = 86_400;
pub const MIN_WINDOW_S: u64 = 600;
pub const MAX_WINDOW_S: u64 = 7 * 86_400;

/// One-minute buckets and a seven-day ring: ~150–250 KB/day append-only.
pub const DEFAULT_BUCKET_MS: u64 = 60_000;
pub const DEFAULT_RETENTION_MS: u64 = 7 * 24 * 3_600_000;

/// Epoch milliseconds (display timeline; decisions use `Instant`).
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Bucket width, overridable for host tests.
pub fn bucket_ms() -> u64 {
    env_u64("SURYAPROFILER_POWER_BUCKET_MS", DEFAULT_BUCKET_MS).max(100)
}

/// Ring retention, overridable for host tests.
pub fn retention_ms() -> u64 {
    env_u64("SURYAPROFILER_POWER_RETAIN_MS", DEFAULT_RETENTION_MS).max(60_000)
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// One live sample: raw reading + daemon context.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PowerSample {
    pub t_ms: u64,
    #[serde(flatten)]
    pub reading: PowerReading,
    pub screen_on: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pkg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
}

impl PowerSample {
    pub fn from_reading(
        t_ms: u64,
        reading: PowerReading,
        screen_on: bool,
        pkg: Option<String>,
        profile: Option<String>,
    ) -> Self {
        PowerSample {
            t_ms,
            reading,
            screen_on,
            pkg,
            profile,
        }
    }
}

/// Per-minute aggregate persisted in `power-history.jsonl`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PowerBucket {
    pub t_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pct: Option<u8>,
    pub avg_ua: i64,
    pub min_ua: i64,
    pub max_ua: i64,
    pub avg_uv: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_temp_c: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_temp_c: Option<f32>,
    pub screen_on: bool,
    pub dur_ms: u64,
    pub samples: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pkg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// Optional segment bounds/energy for newer rows; legacy rows omit them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub energy_mwh: Option<f64>,
    /// Identifies one in-progress segment across cumulative live updates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segment_id: Option<String>,
}

/// All-time peak (until the user clears history). `ua` keeps the device sign:
/// negative for the charge peak, positive for the discharge peak.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Peak {
    pub ua: i64,
    pub uv: u32,
    pub t_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pct: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TempPeak {
    pub c: f32,
    pub t_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pct: Option<u8>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PowerPeaks {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charge: Option<Peak>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discharge: Option<Peak>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temp: Option<TempPeak>,
}

/// On-disk peaks wrapper (schema for future migrations).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeaksFile {
    #[serde(default = "default_peaks_schema")]
    pub schema: u32,
    #[serde(default)]
    pub peaks: PowerPeaks,
}

fn default_peaks_schema() -> u32 {
    PEAKS_SCHEMA
}

impl Default for PeaksFile {
    fn default() -> Self {
        PeaksFile {
            schema: PEAKS_SCHEMA,
            peaks: PowerPeaks::default(),
        }
    }
}

/// `power` reply: live sample + the requested window + all-time peaks.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PowerSnapshot {
    pub window_s: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live: Option<PowerSample>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_bucket: Option<PowerBucket>,
    pub history: Vec<PowerBucket>,
    pub peaks: PowerPeaks,
}

/// Sampler -> main loop message.
#[derive(Debug, Clone)]
pub struct PowerTick {
    pub reading: PowerReading,
    /// Nominal cadence that produced this tick (retained for diagnostics).
    #[allow(dead_code)]
    pub interval_ms: u64,
    /// Wall-clock capture time used as the sample timestamp.
    pub captured_at_ms: u64,
    /// Actual time since the previous capture. The first tick measures from
    /// sampler startup (rounded up to 1 ms so the reading is not discarded).
    pub elapsed_ms: u64,
}

/// Read-only probe path helper (kept here so tests use the same root rule).
pub fn probe_root() -> std::path::PathBuf {
    crate::engine::env::default_root()
}

/// Convenience for the sampler thread.
pub fn probe() -> PowerProbe {
    PowerProbe::new(&probe_root())
}
