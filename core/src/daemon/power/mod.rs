//! Power usage history: fast read-only sampler + bounded per-minute buckets.
//!
//! The sampler only reads battery/charger sysfs and sends a tick; all state
//! (bucket ring, sessions' source data, peaks, files) is owned by the main
//! loop. History is display data: append-only, no per-line fsync, at most one
//! atomic rewrite per day.

mod aggregate;
mod model;
mod store;

#[cfg(test)]
mod tests;

use aggregate::Aggregator;
use model::*;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Duration;

pub use model::{
    now_ms, PowerBucket, PowerPeaks, PowerSample, PowerSnapshot, PowerTick, DEFAULT_WINDOW_S,
    MAX_WINDOW_S, MIN_WINDOW_S,
};

/// Atomic rewrite cadence after retention trimming.
const REWRITE_MS: u64 = 24 * 3_600_000;

/// In-memory + on-disk power history. Owned by the daemon main loop.
pub struct PowerHistory {
    state_dir: PathBuf,
    bucket_ms: u64,
    retention_ms: u64,
    buckets: VecDeque<PowerBucket>,
    current: Option<Aggregator>,
    peaks: PowerPeaks,
    last: Option<PowerSample>,
    last_rewrite_ms: u64,
}

impl PowerHistory {
    /// Device/default constructor (env overrides exist for host tests).
    pub fn load(state_dir: &Path) -> Self {
        Self::load_with(state_dir, bucket_ms(), retention_ms())
    }

    pub fn load_with(state_dir: &Path, bucket_ms: u64, retention_ms: u64) -> Self {
        let now = now_ms();
        let loaded = store::load_history(state_dir);
        let loaded_len = loaded.len();
        let mut buckets: VecDeque<PowerBucket> = loaded.into();
        trim(&mut buckets, now, retention_ms, bucket_ms);
        if buckets.len() != loaded_len {
            store::rewrite(state_dir, &buckets);
        }
        let peaks = store::load_peaks(state_dir);
        PowerHistory {
            state_dir: state_dir.to_path_buf(),
            bucket_ms,
            retention_ms,
            buckets,
            current: None,
            peaks,
            last: None,
            last_rewrite_ms: now,
        }
    }

    /// Adds one sample and returns the live (in-progress) bucket.
    ///
    /// A missing current/voltage node still updates `last` (the live card
    /// shows status/temperature) but never creates a history bucket.
    pub fn record(&mut self, sample: &PowerSample, interval_ms: u64) -> Option<PowerBucket> {
        self.last = Some(sample.clone());
        let bucket_t = sample.t_ms / self.bucket_ms * self.bucket_ms;
        if let Some(cur) = &self.current {
            if bucket_t < cur.t_ms {
                return Some(cur.to_bucket());
            }
            if bucket_t != cur.t_ms {
                if let Some(agg) = self.current.take() {
                    self.close(agg);
                }
            }
        }
        if self.current.is_none() {
            self.current = Some(Aggregator::new(bucket_t));
        }
        let agg = self.current.as_mut().expect("bucket just created");
        agg.add(
            &sample.reading,
            sample.screen_on,
            sample.pkg.as_deref(),
            sample.profile.as_deref(),
            interval_ms,
        );
        if agg.is_empty() {
            None
        } else {
            Some(agg.to_bucket())
        }
    }

    /// Closes the in-progress bucket (clean daemon exit). Idempotent.
    pub fn flush(&mut self) {
        if let Some(agg) = self.current.take() {
            self.close(agg);
        }
    }

    pub fn snapshot(&self, window_s: u64) -> PowerSnapshot {
        let window_s = window_s.clamp(MIN_WINDOW_S, MAX_WINDOW_S);
        let now = now_ms();
        let cutoff = now.saturating_sub(window_s.saturating_mul(1000));
        let history = self
            .buckets
            .iter()
            .filter(|b| b.t_ms >= cutoff)
            .cloned()
            .collect();
        let current_bucket = self
            .current
            .as_ref()
            .filter(|a| a.t_ms >= cutoff && !a.is_empty())
            .map(|a| a.to_bucket());
        PowerSnapshot {
            window_s,
            live: self.last.clone(),
            current_bucket,
            history,
            peaks: self.peaks.clone(),
        }
    }

    /// User Clear: drop history, peaks and the in-progress bucket.
    pub fn clear(&mut self) {
        self.buckets.clear();
        self.current = None;
        self.peaks = PowerPeaks::default();
        self.last = None;
        self.last_rewrite_ms = now_ms();
        store::clear(&self.state_dir);
    }

    #[cfg(test)]
    pub fn peaks(&self) -> &PowerPeaks {
        &self.peaks
    }

    #[cfg(test)]
    pub fn bucket_count(&self) -> usize {
        self.buckets.len()
    }

    fn close(&mut self, agg: Aggregator) {
        if agg.is_empty() {
            return;
        }
        let bucket = agg.to_bucket();
        let now = now_ms();
        if self
            .buckets
            .back()
            .is_some_and(|last| last.t_ms == bucket.t_ms)
        {
            self.buckets.pop_back();
        }
        self.buckets.push_back(bucket.clone());
        store::append(&self.state_dir, &bucket);
        if self.update_peaks(&bucket) {
            store::save_peaks(&self.state_dir, &self.peaks);
        }
        let before = self.buckets.len();
        trim(&mut self.buckets, now, self.retention_ms, self.bucket_ms);
        let trimmed = self.buckets.len() != before;
        let rewrite_due = now.saturating_sub(self.last_rewrite_ms) >= REWRITE_MS;
        if trimmed || rewrite_due {
            store::rewrite(&self.state_dir, &self.buckets);
            self.last_rewrite_ms = now;
        }
    }

    /// All-time peaks. A bucket's extreme current is paired with its average
    /// voltage; the error is small and the method is documented in PROTOCOL.
    fn update_peaks(&mut self, b: &PowerBucket) -> bool {
        let mut changed = false;
        if b.min_ua < 0 && self.peaks.charge.as_ref().is_none_or(|p| b.min_ua < p.ua) {
            self.peaks.charge = Some(Peak {
                ua: b.min_ua,
                uv: b.avg_uv,
                t_ms: b.t_ms,
                pct: b.pct,
            });
            changed = true;
        }
        if b.max_ua > 0
            && self
                .peaks
                .discharge
                .as_ref()
                .is_none_or(|p| b.max_ua > p.ua)
        {
            self.peaks.discharge = Some(Peak {
                ua: b.max_ua,
                uv: b.avg_uv,
                t_ms: b.t_ms,
                pct: b.pct,
            });
            changed = true;
        }
        if let Some(c) = b.max_temp_c {
            if self.peaks.temp.as_ref().is_none_or(|p| c > p.c) {
                self.peaks.temp = Some(TempPeak {
                    c,
                    t_ms: b.t_ms,
                    pct: b.pct,
                });
                changed = true;
            }
        }
        changed
    }
}

fn trim(buckets: &mut VecDeque<PowerBucket>, now: u64, retention_ms: u64, bucket_ms: u64) {
    let cutoff = now.saturating_sub(retention_ms);
    while buckets.front().is_some_and(|b| b.t_ms < cutoff) {
        buckets.pop_front();
    }
    let max_buckets = ((retention_ms / bucket_ms.max(1)) as usize).saturating_add(4);
    while buckets.len() > max_buckets {
        buckets.pop_front();
    }
}

/// Fast sampler: 3 s screen-on / 30 s screen-off by default. Both cadences
/// are test-overridable; the tick carries the effective interval so bucket
/// integration is exact even across a cadence change.
pub fn spawn(tx: Sender<super::Msg>, screen: Arc<AtomicBool>) {
    let probe = probe();
    let on_ms = sample_ms();
    let off_ms = sample_off_ms();
    let _ = std::thread::Builder::new()
        .name("power".into())
        .spawn(move || loop {
            let interval = if screen.load(Ordering::Relaxed) {
                on_ms
            } else {
                off_ms
            };
            let reading = probe.read();
            let tick = PowerTick {
                reading,
                interval_ms: interval,
            };
            if tx.send(super::Msg::Power(tick)).is_err() {
                break;
            }
            std::thread::sleep(Duration::from_millis(interval));
        });
}

pub fn sample_ms() -> u64 {
    env_ms("SURYAPROFILER_POWER_SAMPLE_MS", 3_000).max(50)
}

pub fn sample_off_ms() -> u64 {
    env_ms("SURYAPROFILER_POWER_SAMPLE_OFF_MS", 30_000).max(100)
}

fn env_ms(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
