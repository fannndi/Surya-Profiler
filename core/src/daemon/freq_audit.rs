//! Freq floor audit: the sentinel that notices an externally raised CPU
//! floor and hands the drift back to the normal apply path.
//!
//! Evidence (2026-11-24, device): MIUI's screen-on floor is a bounded
//! overlay, not a permanent pin — a wake pins ~1.25/0.98 GHz for ~10 s and
//! an app-launch boost for ~40+ s, then perfd RELEASES and the previous
//! floor (ours) reads back. Fighting it on screen would cut legitimate
//! boost windows (hard rule 1). What MIUI never fixes by itself is a floor
//! left high while the screen is OFF — and nothing re-wrote it until the
//! 30-minute watchdog. So the audit acts ONLY during screen-off:
//!
//! - screen-off + floor above the active (sleep) profile floor for >30 s
//!   → log full context → re-assert through `evaluate("freq_audit")` (a
//!   forced, guarded, single-writer apply; only drifted keys are rewritten);
//! - a re-assert that does not stick grows a backoff (30 s → 300 s cap)
//!   and is reported as "external floor holder suspected" instead of
//!   looping;
//! - screen-on drift is logged once per episode (visibility), never fought.

use crate::engine::apply::Store;
use std::path::Path;
use std::time::{Duration, Instant};

/// One observation fed to the state machine.
pub struct Observe {
    /// Live floor is above the active profile's floor.
    pub high: bool,
    /// The audit may act here (no mapped app in front, not locked while
    /// awake, no MIUI performance mode, service enabled, not restoring).
    pub allowed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloorAction {
    None,
    /// Grace elapsed: log + trigger a forced re-apply of the active profile.
    Reassert,
    /// The floor returned to the profile value after a re-assert.
    Restored,
}

/// CPU floors in kHz as they live in the nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Floors {
    pub little_khz: u32,
    pub big_khz: u32,
}

pub struct FloorAudit {
    /// Sample cadence (default 15 s; the tick itself is nearly free).
    interval: Duration,
    /// How long the floor must stay high before acting (default 30 s —
    /// comfortably beyond MIUI's ~3 s interaction boost decay).
    grace: Duration,
    last_run: Option<Instant>,
    high_since: Option<Instant>,
    attempts: u32,
    next_attempt: Option<Instant>,
    /// "Yielding to MIUI performance mode" is logged once per episode.
    yield_logged: bool,
    /// "MIUI holds the screen-on floor" is logged once per episode.
    holder_noted: bool,
}

impl FloorAudit {
    pub fn new() -> Self {
        Self::with(
            env_ms("SURYAPROFILER_AUDIT_INTERVAL_MS", 15_000),
            env_ms("SURYAPROFILER_AUDIT_GRACE_MS", 30_000),
        )
    }

    pub fn with(interval: Duration, grace: Duration) -> Self {
        FloorAudit {
            interval,
            grace,
            last_run: None,
            high_since: None,
            attempts: 0,
            next_attempt: None,
            yield_logged: false,
            holder_noted: false,
        }
    }

    /// Rate gate: at most one observation per `interval`.
    pub fn due(&mut self, now: Instant) -> bool {
        if self
            .last_run
            .is_some_and(|t| now.duration_since(t) < self.interval)
        {
            return false;
        }
        self.last_run = Some(now);
        true
    }

    /// Feed one observation; returns what the daemon should do.
    pub fn observe(&mut self, o: Observe, now: Instant) -> FloorAction {
        if !o.allowed {
            self.reset_episode();
            return FloorAction::None;
        }
        if !o.high {
            if self.attempts > 0 {
                self.reset_episode();
                return FloorAction::Restored;
            }
            self.reset_episode();
            return FloorAction::None;
        }
        if self.next_attempt.is_some_and(|t| now < t) {
            return FloorAction::None;
        }
        let Some(since) = self.high_since else {
            self.high_since = Some(now);
            return FloorAction::None;
        };
        if now.duration_since(since) < self.grace {
            return FloorAction::None;
        }
        self.attempts += 1;
        self.next_attempt = Some(now + Self::backoff(self.attempts));
        FloorAction::Reassert
    }

    /// Wait before the next attempt after the Nth re-assert: 30 s, 60 s,
    /// 120 s, 240 s, capped at 300 s. Slows a fight we cannot win.
    fn backoff(attempts: u32) -> Duration {
        let secs = 30u64.saturating_mul(1 << (attempts - 1).min(4)).min(300);
        Duration::from_secs(secs)
    }

    /// Full reset (new episode, yield/holder notes forgotten).
    pub fn reset(&mut self) {
        self.reset_episode();
        self.yield_logged = false;
        self.holder_noted = false;
    }

    fn reset_episode(&mut self) {
        self.high_since = None;
        self.attempts = 0;
        self.next_attempt = None;
    }

    /// True the first time a yield is observed in the current stretch.
    pub fn mark_yield(&mut self) -> bool {
        if self.yield_logged {
            return false;
        }
        self.yield_logged = true;
        true
    }

    /// True the first time a screen-on floor hold is noted in a stretch.
    pub fn note_holder(&mut self) -> bool {
        if self.holder_noted {
            return false;
        }
        self.holder_noted = true;
        true
    }

    /// Whether the screen-on floor hold has been noted (for the release log).
    pub fn holder_noted(&self) -> bool {
        self.holder_noted
    }

    /// Re-asserts issued in the current episode (1 = first, clean shot).
    pub fn attempts(&self) -> u32 {
        self.attempts
    }
}

fn env_ms(name: &str, default_ms: u64) -> Duration {
    Duration::from_millis(
        std::env::var(name)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default_ms),
    )
}

// --- device reads (all read-only) --------------------------------------------

fn read_khz(root: &Path, rel: &str) -> u32 {
    std::fs::read_to_string(root.join(rel))
        .ok()
        .and_then(|s| s.split_whitespace().next().and_then(|t| t.parse().ok()))
        .unwrap_or(0)
}

/// Live `scaling_min_freq` of both policies (0 when the node is absent —
/// host fakes without the node simply never look "high").
pub fn live_floors() -> Floors {
    let root = crate::engine::env::default_root();
    Floors {
        little_khz: read_khz(
            &root,
            "sys/devices/system/cpu/cpufreq/policy0/scaling_min_freq",
        ),
        big_khz: read_khz(
            &root,
            "sys/devices/system/cpu/cpufreq/policy6/scaling_min_freq",
        ),
    }
}

/// The active profile's own floors from the deployed pack (None → the
/// profile does not manage min freq; the audit must not guess).
pub fn desired_floors(state_dir: &Path, profile_id: &str) -> Option<Floors> {
    let files = Store::new(state_dir).load_profiles(None).ok()?;
    let p = files.profiles.iter().find(|p| p.id == profile_id)?;
    Some(Floors {
        little_khz: p
            .params
            .get("policy0.scaling_min_freq")?
            .trim()
            .parse()
            .ok()?,
        big_khz: p
            .params
            .get("policy6.scaling_min_freq")?
            .trim()
            .parse()
            .ok()?,
    })
}

// --- runtime glue -------------------------------------------------------------
// Lives in `freq_audit/runtime.rs` (file-size cap); the state machine and
// the device reads stay here.

pub(super) mod runtime;

#[cfg(test)]
mod tests;
