//! Restore gate + sync/release orchestration.

use super::*;

impl Bridge {
    /// Starts the restore gate immediately...
    pub(crate) fn begin_restore(&self) {
        self.blocked.store(true, Ordering::Release);
    }

    /// True when `holds.json` has any active hold.
    pub(crate) fn has_holds(&self) -> bool {
        self.lock().holds.any_held()
    }

    /// One full sync: MIUI saver follow, R&D input suspend, thermal profile.
    /// Serialized by the internal Mutex; safe to call after every decision.
    ///
    /// The restore gate is checked *inside* the lock. `release_all` takes the
    /// same lock before it writes anything back, so a sync either completes
    /// before the release (and is then undone by it) or observes the gate and
    /// returns without touching the device.
    pub(crate) fn sync(&self, ctx: &SyncCtx) {
        let (saver_changed, input_suspend_changed, thermal_changed) = {
            let mut st = self.lock();
            if self.blocked.load(Ordering::Acquire) || !ctx.enabled {
                return;
            }
            let saver_changed = self.sync_saver(&mut st, ctx);
            let input_suspend_changed = self.sync_input_suspend(&mut st, ctx);
            let thermal_changed = self.sync_thermal(&mut st, ctx);
            (saver_changed, input_suspend_changed, thermal_changed)
        };
        if saver_changed || input_suspend_changed || thermal_changed {
            self.refresh_attribution();
            let st = self.lock();
            self.persist(&st);
        }
    }

    /// Writes the user's captured values back (service-off restore / exit).
    ///
    /// Returns a per-surface report: Service OFF is only allowed when every
    /// surface succeeded. The gate is closed while holding the state lock, so
    /// no sync can slip between the release and the blocked flag.
    pub(crate) fn release_all(&self) -> ReleaseReport {
        let mut report = ReleaseReport::default();
        let mut changed = false;
        {
            let mut st = self.lock();
            self.blocked.store(true, Ordering::Release);

            if st.holds.saver_held {
                let saved = st.holds.saver_saved;
                let value = if saved { "1" } else { "0" };
                match settings::put_verified("global", settings::SAVER_KEY, value) {
                    Ok(()) => {
                        self.log_event("MIUI saver restored".into());
                        self.set_live_saver(saved);
                        st.holds.saver_held = false;
                        changed = true;
                    }
                    Err(e) => {
                        report.saver_ok = false;
                        self.log_event(format!("MIUI saver restore failed: {e}"));
                    }
                }
            }

            if st.holds.input_suspend_held {
                let value = st
                    .holds
                    .input_suspend_saved
                    .clone()
                    .unwrap_or_else(|| "0".into());
                match super::input_suspend::write_verified(&value) {
                    Ok(()) => {
                        self.log_event(format!("R&D input suspend OFF (node {value})"));
                        st.holds.input_suspend_held = false;
                        st.holds.input_suspend_saved = None;
                        changed = true;
                    }
                    Err(e) => {
                        report.input_suspend_ok = false;
                        self.log_event(format!("R&D input suspend release failed: {e}"));
                    }
                }
            }

            let thermal_needed = st.holds.thermal_held || thermal::marker_exists();
            let thermal_released = self.release_thermal_locked(&mut st);
            report.thermal_ok = !thermal_needed || thermal_released;
            if thermal_released {
                changed = true;
            }
        }
        if changed {
            self.refresh_attribution();
            let st = self.lock();
            self.persist(&st);
        }
        if report.ok() {
            self.log_event("all holds released".into());
        } else {
            self.log_event(format!("release incomplete: {:?}", report.failures()));
        }
        report
    }

    /// Re-enable syncs after a failed restore (the service stays alive).
    pub(crate) fn resume(&self) {
        self.blocked.store(false, Ordering::Release);
    }
}
