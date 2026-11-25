//! One-minute bucket aggregation (pure; no IO).

use super::model::PowerBucket;
use crate::engine::env::PowerReading;
use std::collections::HashMap;

/// Mutable in-progress bucket. Owned by `PowerHistory` in the main loop.
pub struct Aggregator {
    pub t_ms: u64,
    samples: u32,
    dur_ms: u64,
    on_ms: u64,
    sum_ua: i64,
    min_ua: i64,
    max_ua: i64,
    sum_uv: u64,
    sum_temp: f64,
    temp_n: u32,
    max_temp: Option<f32>,
    pct: Option<u8>,
    pkg_counts: HashMap<String, u32>,
    prof_counts: HashMap<String, u32>,
}

impl Aggregator {
    pub fn new(t_ms: u64) -> Self {
        Aggregator {
            t_ms,
            samples: 0,
            dur_ms: 0,
            on_ms: 0,
            sum_ua: 0,
            min_ua: 0,
            max_ua: 0,
            sum_uv: 0,
            sum_temp: 0.0,
            temp_n: 0,
            max_temp: None,
            pct: None,
            pkg_counts: HashMap::new(),
            prof_counts: HashMap::new(),
        }
    }

    /// Adds one reading. Requires current + voltage (the caller skips
    /// history for a sensor that reports neither).
    pub fn add(
        &mut self,
        reading: &PowerReading,
        screen_on: bool,
        pkg: Option<&str>,
        profile: Option<&str>,
        interval_ms: u64,
    ) -> bool {
        let (Some(ua), Some(uv)) = (reading.current_ua, reading.voltage_uv) else {
            return false;
        };
        if self.samples == 0 {
            self.min_ua = ua;
            self.max_ua = ua;
        } else {
            self.min_ua = self.min_ua.min(ua);
            self.max_ua = self.max_ua.max(ua);
        }
        self.sum_ua = self.sum_ua.saturating_add(ua);
        self.sum_uv = self.sum_uv.saturating_add(uv as u64);
        if let Some(t) = reading.temp_c {
            self.sum_temp += t as f64;
            self.temp_n += 1;
            self.max_temp = Some(self.max_temp.map_or(t, |m| m.max(t)));
        }
        if let Some(p) = reading.pct {
            self.pct = Some(p);
        }
        let interval = interval_ms.max(1);
        self.dur_ms = self.dur_ms.saturating_add(interval);
        if screen_on {
            self.on_ms = self.on_ms.saturating_add(interval);
            if let Some(pkg) = pkg {
                *self.pkg_counts.entry(pkg.to_string()).or_insert(0) += 1;
            }
        }
        if let Some(profile) = profile {
            *self.prof_counts.entry(profile.to_string()).or_insert(0) += 1;
        }
        self.samples += 1;
        true
    }

    pub fn is_empty(&self) -> bool {
        self.samples == 0
    }

    /// Closes (does not consume) the bucket. `t_ms` is the bucket start.
    pub fn to_bucket(&self) -> PowerBucket {
        let n = self.samples.max(1) as i64;
        let screen_on = self.dur_ms > 0 && self.on_ms.saturating_mul(2) >= self.dur_ms;
        PowerBucket {
            t_ms: self.t_ms,
            pct: self.pct,
            avg_ua: self.sum_ua / n,
            min_ua: self.min_ua,
            max_ua: self.max_ua,
            avg_uv: (self.sum_uv / n as u64) as u32,
            avg_temp_c: (self.temp_n > 0).then(|| (self.sum_temp / self.temp_n as f64) as f32),
            max_temp_c: self.max_temp,
            screen_on,
            dur_ms: self.dur_ms,
            samples: self.samples,
            pkg: if screen_on {
                mode(&self.pkg_counts)
            } else {
                None
            },
            profile: mode(&self.prof_counts),
        }
    }
}

/// Most frequent key; ties resolve to the lexicographically smaller key so
/// host tests and repeated runs are deterministic.
fn mode(counts: &HashMap<String, u32>) -> Option<String> {
    counts
        .iter()
        .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
        .map(|(k, _)| k.clone())
}
