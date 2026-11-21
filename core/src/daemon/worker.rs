//! Apply worker: the coalescing loop that decides *when* a profile is
//! applied. Runs on its own thread; the engine (in-process) executes.
//!
//! Semantics:
//! - one app transition emits its event pair within ~50 ms, so a short
//!   settle window absorbs bursts — the LATEST decision in the window wins;
//! - a decision arriving while an apply runs supersedes the pending one;
//! - a failed apply (MIUI race) is retried exactly once after a short pause;
//! - `Restore` drops pending applies, releases holds and writes the stock
//!   snapshot back (service off).
//!
//! Timing is measured with `Instant` (monotonic): wall-clock jumps must
//! never distort settle/latency numbers.

use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

/// Coalescing window for one app transition (measured on device 2026-10-08:
/// same accuracy as 700 ms, ~300 ms faster). Overridable via env for tuning.
pub const SETTLE_MS: u64 = 400;

pub fn settle_ms() -> u64 {
    std::env::var("SURYAPROFILER_SETTLE_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(SETTLE_MS)
}
/// Pause before the single retry of a failed apply.
pub const RETRY_MS: u64 = 2_000;

/// One decision queued for execution.
#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    pub profile: String,
    pub reason: String,
    pub src_pkg: Option<String>,
    pub used_saver: bool,
    /// When the producer queued this job (monotonic) — settles the latency.
    pub queued: Instant,
    /// Re-plan even when the profile is already active: a config/profile-pack
    /// change or an explicit user tap must reach the engine (new keys,
    /// drift), while periodic confirmations keep the fast path.
    pub force: bool,
    /// The auto-revive watchdog produced this job (drift reconciliation);
    /// a successful run that wrote keys counts as a heal for diagnostics.
    pub watchdog: bool,
    /// Explicit user tap (`set_base`). A later foreground decision in the
    /// settle window must not swallow the tap; the next event re-evaluates.
    pub sticky: bool,
}

/// Work items the worker accepts.
#[derive(Debug, Clone, PartialEq)]
pub enum Work {
    Apply(Job),
    /// Drop pending applies, release holds, write the stock snapshot back.
    Restore,
    /// Daemon exit: finish the in-flight apply, release holds and restore
    /// stock, then let the worker thread end. This is the only path allowed
    /// to run exit cleanup — the main loop must never race a writer.
    Shutdown,
}

/// Result of a completed apply (the app turns this into UI + notification).
#[derive(Debug, Clone, PartialEq)]
pub struct AppliedEvent {
    pub profile: String,
    pub reason: String,
    pub src_pkg: Option<String>,
    pub ok: bool,
    /// True when at least one key was written/verified or the profile was
    /// already in sync; false when every write was rejected by the kernel.
    pub applied: bool,
    pub wrote: usize,
    pub verified: usize,
    pub failed: usize,
    pub ms: u64,
    pub settle_ms: u64,
    /// Watchdog reconcile that wrote keys (auto-revive heal).
    pub watchdog: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RestoredEvent {
    pub ok: bool,
    pub wrote: usize,
    pub verified: usize,
    pub failed: usize,
    pub release: crate::daemon::bridge::ReleaseReport,
}

/// Result of the worker-owned exit cleanup.
#[derive(Debug, Clone, PartialEq)]
pub struct ShutdownReport {
    pub ok: bool,
    pub restored: bool,
    pub wrote: usize,
    pub verified: usize,
    pub failed: usize,
    pub release: crate::daemon::bridge::ReleaseReport,
}

/// Outcome of one engine apply/restore call.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub ok: bool,
    /// At least one key was written/verified or the target was already in
    /// sync. False when every attempted write was rejected by the device.
    pub applied: bool,
    pub wrote: usize,
    pub verified: usize,
    pub failed: usize,
    pub error: Option<String>,
}

impl Outcome {
    pub fn err(msg: String) -> Self {
        Outcome {
            ok: false,
            applied: false,
            wrote: 0,
            verified: 0,
            failed: 0,
            error: Some(msg),
        }
    }
}

/// Everything the worker needs from the engine layer — injected so the
/// coalescing logic is unit-testable without a device.
pub trait EngineDriver: Send {
    /// Currently active profile id (engine state), None when stock.
    fn active(&mut self) -> Option<String>;
    /// Apply a profile (engine handles snapshot/pass-2 internally).
    fn apply(&mut self, profile_id: &str, reconcile: bool) -> Outcome;
    /// Release bridge holds (MIUI saver / R&D input suspend / thermal).
    fn release_holds(&mut self) -> crate::daemon::bridge::ReleaseReport;
    /// Write the stock snapshot back and verify.
    fn restore(&mut self) -> Outcome;
}

// Worker loop (execution lives in `worker/run.rs`).
mod run;
pub use run::run_with_shutdown;

/// Test-only convenience wrapper.
#[cfg(test)]
pub fn run(
    rx: Receiver<Work>,
    engine: &mut dyn EngineDriver,
    settle: Duration,
    retry: Duration,
    on_applied: &mut dyn FnMut(AppliedEvent),
    on_restored: &mut dyn FnMut(RestoredEvent),
) {
    run_with_shutdown(
        rx,
        engine,
        settle,
        retry,
        on_applied,
        on_restored,
        &mut |_| {},
    );
}

#[cfg(test)]
mod tests;
