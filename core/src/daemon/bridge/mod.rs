//! MIUI bridge: battery-saver follow + the R&D input-suspend switch.
//!
//! Files:
//! - `holds.rs`    saver/input-suspend hold state machine + persisted file
//! - `sync.rs`     SyncCtx, the pure saver gate, the settings CLI sync
//! - `input_suspend.rs` R&D charger-input suspend hold/restore
//! - `mod.rs`      Bridge (shared state + one Mutex) + recover/release/persist
//!
//! Threading: one `Bridge` shared by the main loop (user_saver read), the
//! bridge sync thread (sync) and the worker (release_all on service-off
//! restore). A single Mutex serializes all state + IO.
//!
//! Restore exclusivity: `release_all` blocks further syncs (`blocked=true`)
//! so a queued foreground context cannot re-engage a saver/input-suspend hold after
//! the service-off restore has started. A failed restore calls `resume()`.

mod affinity;
mod holds;
mod input_suspend;
mod lifecycle;
mod store;
mod sync;

#[cfg(test)]
mod tests;
mod thermal;

pub use holds::HoldsInfo;
pub use holds::ReleaseReport;
pub use sync::SyncCtx;
pub use thermal::cleanup_orphan as cleanup_thermal;
pub use thermal::files_fix as thermal_files_fix;
pub use thermal::files_scan as thermal_files_scan;

/// Read-only thermal takeover inspection for `doctor`.
pub fn thermal_inspect() -> serde_json::Value {
    thermal::inspect()
}

/// Releases every persisted hold without a running daemon.
///
/// Used by the CLI (`clean`, and as the final step of `restore`) and by the
/// app when the service was stopped after a crash. It constructs a normal
/// [`Bridge`] from `holds.json`, so the same state machine and failure rules
/// apply as on the daemon path.
pub fn clean_holds(state_dir: &Path) -> ReleaseReport {
    let publisher = Publisher::new();
    let bridge = Bridge::recover(state_dir, publisher, None);
    let report = bridge.release_all();
    if report.ok() {
        bridge.resume();
    }
    report
}

use super::proto::Publisher;
use crate::daemon::settings;
use crate::daemon::settings::LiveFlags;
use holds::{Holds, HoldsFile};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI8, Ordering};
use std::sync::{Arc, Mutex};
/// Shared bridge (state + IO), one Mutex serializes everything.
pub struct Bridge {
    holds_path: PathBuf,
    affinity_path: PathBuf,
    publisher: Arc<Publisher>,
    state: Mutex<State>,
    /// Lock-free attribution mirror: -1 = no hold (arbiter sees live),
    /// 0/1 = the captured user value while a saver hold is active. Kept in
    /// sync with `state` so the decision loop never blocks on the bridge Mutex.
    saver_override: AtomicI8,
    /// Live saver cache; updated write-through after every settings put so
    /// the arbiter never mistakes our own just-written value for user intent.
    flags: Option<Arc<LiveFlags>>,
    /// False during service-off restore: no sync may re-engage a hold.
    blocked: AtomicBool,
}

pub(crate) struct State {
    holds: Holds,
    /// Thread placement (opt-in Game-only): captured affinity per tid.
    affinity: affinity::AffinityState,
    /// Rebuild `/data` map/configs on the first thermal sync after a daemon
    /// restart so ROM updates and the 20–23 -> 60–63 migration are picked up.
    thermal_refresh_pending: bool,
    /// Backoff after a failed `mi_thermald` select/install (avoid a 4 s
    /// blocking retry + log spam on every sync while the daemon is broken).
    thermal_retry_at: Option<std::time::Instant>,
    thermal_failures: u8,
}

impl Bridge {
    /// User-intent saver value for the arbiter: lock-free read (the refresh
    /// happens after every holds mutation).
    pub fn user_saver(&self, live: bool) -> bool {
        match self.saver_override.load(Ordering::Relaxed) {
            0 => false,
            1 => true,
            _ => live,
        }
    }

    /// Keep the polled cache in step with our own write.
    fn set_live_saver(&self, value: bool) {
        if let Some(flags) = &self.flags {
            flags.set_saver(value);
        }
    }

    /// Read-only hold state for `diag` (brief lock; never any IO).
    pub fn holds_info(&self) -> HoldsInfo {
        let st = self.lock();
        HoldsInfo {
            saver_held: st.holds.saver_held,
            saver_saved: st.holds.saver_saved,
            input_suspend_held: st.holds.input_suspend_held,
            input_suspend_saved: st.holds.input_suspend_saved.clone(),
            thermal_held: st.holds.thermal_held,
            thermal_saved_sconfig: st.holds.thermal_saved_sconfig,
            thermal_profile: st.holds.thermal_profile.clone(),
            thermal_yield: st.holds.thermal_yield,
            affinity_held: st.affinity.active(),
            affinity_app: st.affinity.target.clone(),
            affinity_threads: st.affinity.threads.len(),
        }
    }

    fn refresh_attribution(&self) {
        let st = self.lock();
        // tri-state mirror: -1 no hold; 0/1 = the captured user value.
        let v: i8 = if st.holds.saver_held {
            if st.holds.user_saver(false) {
                1
            } else {
                0
            }
        } else {
            -1
        };
        self.saver_override.store(v, Ordering::Relaxed);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn log_event(&self, msg: String) {
        self.publisher.log(&format!("bridge: {msg}"));
    }
}
