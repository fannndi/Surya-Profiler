//! One-minute bucket aggregation (pure; no IO).

use super::model::PowerBucket;
use crate::engine::env::PowerReading;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

static SEGMENT_SEQ: AtomicU64 = AtomicU64::new(0);

fn segment_id(t_ms: u64) -> String {
    format!(
        "power-{}-{t_ms}-{}",
        std::process::id(),
        SEGMENT_SEQ.fetch_add(1, Ordering::Relaxed)
    )
}

/// Mutable in-progress bucket. Owned by `PowerHistory` in the main loop.
pub struct Aggregator {
    pub t_ms: u64,
    segment_id: String,
    samples: u32,
    dur_ms: u64,
    on_ms: u64,
    sum_ua: f64,
    min_ua: i64,
    max_ua: i64,
    sum_uv: f64,
    sum_temp: f64,
    temp_weight_ms: u64,
    max_temp: Option<f32>,
    pct: Option<u8>,
    pkg_counts: HashMap<String, u64>,
    prof_counts: HashMap<String, u64>,
    pkg_unknown_ms: u64,
    prof_unknown_ms: u64,
}

impl Aggregator {
    pub fn new(t_ms: u64) -> Self {
        Aggregator {
            t_ms,
            segment_id: segment_id(t_ms),
            samples: 0,
            dur_ms: 0,
            on_ms: 0,
            sum_ua: 0.0,
            min_ua: 0,
            max_ua: 0,
            sum_uv: 0.0,
            sum_temp: 0.0,
            temp_weight_ms: 0,
            max_temp: None,
            pct: None,
            pkg_counts: HashMap::new(),
            prof_counts: HashMap::new(),
            pkg_unknown_ms: 0,
            prof_unknown_ms: 0,
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
        if let Some(t) = reading.temp_c {
            self.max_temp = Some(self.max_temp.map_or(t, |m| m.max(t)));
        }
        if let Some(p) = reading.pct {
            self.pct = Some(p);
        }
        if interval_ms > 0 {
            self.sum_ua += ua as f64 * interval_ms as f64;
            self.sum_uv += uv as f64 * interval_ms as f64;
            if let Some(t) = reading.temp_c {
                self.sum_temp += t as f64 * interval_ms as f64;
                self.temp_weight_ms = self.temp_weight_ms.saturating_add(interval_ms);
            }
            self.dur_ms = self.dur_ms.saturating_add(interval_ms);
            if screen_on {
                self.on_ms = self.on_ms.saturating_add(interval_ms);
                if let Some(pkg) = pkg {
                    *self.pkg_counts.entry(pkg.to_string()).or_insert(0) += interval_ms;
                } else {
                    self.pkg_unknown_ms = self.pkg_unknown_ms.saturating_add(interval_ms);
                }
            }
            if let Some(profile) = profile {
                *self.prof_counts.entry(profile.to_string()).or_insert(0) += interval_ms;
            } else {
                self.prof_unknown_ms = self.prof_unknown_ms.saturating_add(interval_ms);
            }
        }
        self.samples += 1;
        true
    }

    pub fn is_empty(&self) -> bool {
        self.samples == 0
    }

    /// True once at least one real elapsed interval is represented.
    pub fn has_duration(&self) -> bool {
        self.dur_ms > 0
    }

    /// Closes (does not consume) the bucket. `t_ms` is the bucket start.
    pub fn to_bucket(&self) -> PowerBucket {
        let n = self.dur_ms.max(1) as f64;
        let screen_on = self.dur_ms > 0 && self.on_ms.saturating_mul(2) >= self.dur_ms;
        let avg_ua = (self.sum_ua / n).round() as i64;
        let avg_uv = (self.sum_uv / n).round() as u32;
        PowerBucket {
            t_ms: self.t_ms,
            pct: self.pct,
            avg_ua,
            min_ua: self.min_ua,
            max_ua: self.max_ua,
            avg_uv,
            avg_temp_c: (self.temp_weight_ms > 0)
                .then(|| (self.sum_temp / self.temp_weight_ms as f64) as f32),
            max_temp_c: self.max_temp,
            screen_on,
            dur_ms: self.dur_ms,
            samples: self.samples,
            pkg: if screen_on {
                mode(&self.pkg_counts, self.pkg_unknown_ms)
            } else {
                None
            },
            profile: mode(&self.prof_counts, self.prof_unknown_ms),
            start_ms: Some(self.t_ms),
            end_ms: Some(self.t_ms.saturating_add(self.dur_ms)),
            energy_mwh: Some(
                avg_ua as f64 * avg_uv as f64 / 1_000_000_000.0
                    * (self.dur_ms as f64 / 3_600_000.0),
            ),
            segment_id: Some(self.segment_id.clone()),
        }
    }
}

/// Combines closed segments that landed in the same minute. Duration is the
/// weight for every average and for foreground/profile dominance; unknown
/// attribution still contributes to the bucket's total duration.
pub(super) fn merge_buckets(a: PowerBucket, b: PowerBucket) -> PowerBucket {
    let wa = a.dur_ms.max(1) as f64;
    let wb = b.dur_ms.max(1) as f64;
    let weight = wa + wb;
    let on_ms = if a.screen_on { a.dur_ms } else { 0 }.saturating_add(if b.screen_on {
        b.dur_ms
    } else {
        0
    });
    let dur_ms = a.dur_ms.saturating_add(b.dur_ms);
    let screen_on = dur_ms > 0 && on_ms.saturating_mul(2) >= dur_ms;
    let avg_temp_c = match (a.avg_temp_c, b.avg_temp_c) {
        (Some(ta), Some(tb)) => Some(((ta as f64 * wa + tb as f64 * wb) / weight) as f32),
        (Some(t), None) => Some(t),
        (None, Some(t)) => Some(t),
        (None, None) => None,
    };
    let avg_ua = ((a.avg_ua as f64 * wa + b.avg_ua as f64 * wb) / weight).round() as i64;
    let avg_uv = ((a.avg_uv as f64 * wa + b.avg_uv as f64 * wb) / weight).round() as u32;
    let start_ms = match (a.start_ms, b.start_ms) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    };
    let end_ms = match (a.end_ms, b.end_ms) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    };
    let segment_id = match (&a.segment_id, &b.segment_id) {
        (Some(a), Some(b)) if a == b => Some(a.clone()),
        _ => None,
    };
    let pkg_unknown_ms = (if a.screen_on && a.pkg.is_none() {
        a.dur_ms
    } else {
        0
    })
    .saturating_add(if b.screen_on && b.pkg.is_none() {
        b.dur_ms
    } else {
        0
    });
    let prof_unknown_ms = if a.profile.is_none() { a.dur_ms } else { 0 }
        .saturating_add(if b.profile.is_none() { b.dur_ms } else { 0 });
    let energy_mwh = match (a.energy_mwh, b.energy_mwh) {
        (Some(a), Some(b)) => Some(a + b),
        _ if a.energy_mwh.is_some() || b.energy_mwh.is_some() => {
            Some(avg_ua as f64 * avg_uv as f64 / 1_000_000_000.0 * (dur_ms as f64 / 3_600_000.0))
        }
        _ => None,
    };
    PowerBucket {
        t_ms: a.t_ms,
        pct: b.pct.or(a.pct),
        avg_ua,
        min_ua: a.min_ua.min(b.min_ua),
        max_ua: a.max_ua.max(b.max_ua),
        avg_uv,
        avg_temp_c,
        max_temp_c: match (a.max_temp_c, b.max_temp_c) {
            (Some(ta), Some(tb)) => Some(ta.max(tb)),
            (ta, tb) => ta.or(tb),
        },
        screen_on,
        dur_ms,
        samples: a.samples.saturating_add(b.samples),
        pkg: if screen_on {
            dominant(
                [
                    a.pkg
                        .map(|pkg| (pkg, if a.screen_on { a.dur_ms } else { 0 })),
                    b.pkg
                        .map(|pkg| (pkg, if b.screen_on { b.dur_ms } else { 0 })),
                ],
                pkg_unknown_ms,
            )
        } else {
            None
        },
        profile: dominant(
            [
                a.profile.map(|profile| (profile, a.dur_ms)),
                b.profile.map(|profile| (profile, b.dur_ms)),
            ],
            prof_unknown_ms,
        ),
        start_ms,
        end_ms,
        energy_mwh,
        segment_id,
    }
}

fn dominant(rows: [Option<(String, u64)>; 2], unknown_ms: u64) -> Option<String> {
    rows.into_iter()
        .flatten()
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .filter(|(_, duration)| *duration > unknown_ms)
        .map(|(key, _)| key)
}

/// Most frequent known key; unknown/unattributed time wins ties so a merge
/// never invents attribution. Known-key ties choose the lexicographically
/// smaller key for deterministic host tests and repeated runs.
fn mode(counts: &HashMap<String, u64>, unknown_ms: u64) -> Option<String> {
    counts
        .iter()
        .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
        .filter(|(_, duration)| **duration > unknown_ms)
        .map(|(k, _)| k.clone())
}
