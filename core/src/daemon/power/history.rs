//! In-memory ring, persistence and all-time peaks for power history.

use super::aggregate::{merge_buckets, Aggregator};
use super::model::*;
use super::store;
use super::{MAX_WINDOW_S, MIN_WINDOW_S};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

/// Atomic rewrite cadence after retention trimming.
const REWRITE_MS: u64 = 24 * 3_600_000;

/// In-memory + on-disk power history. Owned by the daemon main loop.
pub struct PowerHistory {
    state_dir: PathBuf,
    bucket_ms: u64,
    retention_ms: u64,
    pub(super) buckets: VecDeque<PowerBucket>,
    current: Option<Aggregator>,
    peaks: PowerPeaks,
    last: Option<PowerSample>,
    pub(super) last_rewrite_ms: u64,
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
        let mut history = Self {
            state_dir: state_dir.to_path_buf(),
            bucket_ms,
            retention_ms,
            buckets: VecDeque::new(),
            current: None,
            peaks: store::load_peaks(state_dir),
            last: None,
            last_rewrite_ms: now,
        };
        let mut peaks_changed = false;
        for bucket in &loaded {
            peaks_changed |= history.update_peaks(bucket);
        }
        if peaks_changed {
            let _ = store::save_peaks(state_dir, &history.peaks);
        }
        history.buckets = loaded.into();
        trim(&mut history.buckets, now, retention_ms, bucket_ms);
        if history.buckets.len() != loaded_len
            && store::rewrite(state_dir, &history.buckets).is_err()
        {
            history.last_rewrite_ms = 0;
        }
        history
    }

    /// Adds one sample using the tick's actual `elapsed_ms` and returns the
    /// live (in-progress) bucket. Gaps over two bucket widths contribute no
    /// duration, so suspend does not fabricate usage.
    ///
    /// A missing current/voltage node still updates `last` (the live card
    /// shows status/temperature) but never creates a history bucket.
    pub fn record(&mut self, sample: &PowerSample, elapsed_ms: u64) -> Option<PowerBucket> {
        self.last = Some(sample.clone());
        // A suspend can span many minutes. Keep the live reading, but never
        // pretend that wall time was represented by this cadence.
        let elapsed_ms = if elapsed_ms > self.bucket_ms.saturating_mul(2) {
            0
        } else {
            elapsed_ms
        };
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
            elapsed_ms,
        );
        // The first reading (or the first after a suspend gap) has no elapsed
        // interval. Keep it for extrema/peaks, but do not fabricate a visible
        // or durable zero-duration bucket.
        if agg.is_empty() || !agg.has_duration() {
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
            .filter(|a| a.t_ms >= cutoff && a.has_duration())
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
    pub fn clear(&mut self) -> std::io::Result<()> {
        store::clear(&self.state_dir)?;
        self.buckets.clear();
        self.current = None;
        self.peaks = PowerPeaks::default();
        self.last = None;
        self.last_rewrite_ms = now_ms();
        Ok(())
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
        // A zero-duration segment still carries a real instantaneous reading,
        // so reconcile peaks, but never persist it as represented time.
        if self.update_peaks(&bucket) {
            let _ = store::save_peaks(&self.state_dir, &self.peaks);
        }
        if bucket.dur_ms == 0 {
            return;
        }
        if self
            .buckets
            .back()
            .is_some_and(|last| last.t_ms == bucket.t_ms)
        {
            let previous = self.buckets.pop_back().expect("same-minute bucket");
            let merged = merge_buckets(previous, bucket);
            self.buckets.push_back(merged);
            // A replacement append would double-count after reload. Rewrite
            // the whole ring instead; on failure the old file stays intact.
            let _ = store::rewrite(&self.state_dir, &self.buckets);
        } else {
            self.buckets.push_back(bucket.clone());
            let _ = store::append(&self.state_dir, &bucket);
        }
        let before = self.buckets.len();
        trim(&mut self.buckets, now, self.retention_ms, self.bucket_ms);
        let trimmed = self.buckets.len() != before;
        let rewrite_due = now.saturating_sub(self.last_rewrite_ms) >= REWRITE_MS;
        if (trimmed || rewrite_due) && store::rewrite(&self.state_dir, &self.buckets).is_ok() {
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
