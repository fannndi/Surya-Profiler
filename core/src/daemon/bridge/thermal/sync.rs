//! One thermal sync: install, yield, select, repair.

use super::release::set_sconfig_verified;
use super::*;
use std::time::Instant;

#[cfg(test)]
mod tests;

/// MIUI can write a yield scenario between our select write and the next
/// sync. Re-read immediately so the camera/phone override is not lost.
fn post_select_yield(target: i32) -> Option<i32> {
    let live = read_i32(&sconfig_path())?;
    (live != target && thermal_scenario::is_yield(live)).then_some(live)
}

impl Bridge {
    /// One thermal sync. Caller holds the bridge lock.
    pub(crate) fn sync_thermal(&self, st: &mut State, ctx: &SyncCtx) -> bool {
        let desired = if ctx.custom_thermal {
            ctx.profile.as_deref().and_then(ThermalProfile::from_id)
        } else {
            None
        };
        let Some(desired) = desired else {
            return self.release_thermal_locked(st);
        };
        if st.holds_recovery_blocked {
            // Recovery already logged why. Release remains available, but a
            // missing restore point must not be replaced by a guessed capture.
            return false;
        }

        let mut changed = false;

        // Refresh map/configs once after a daemon restart so a ROM update or
        // an upgrade from the 20–23 era is picked up even when holds.json
        // says thermal_held=true.
        if st.thermal_refresh_pending {
            if st.holds.thermal_held || marker_path().exists() {
                if !self.persist_thermal_before_write(st) {
                    return changed;
                }
                match install_all() {
                    Ok(src) => self.log_event(format!("thermal: map refreshed ({src})")),
                    Err(e) => self.log_event(format!("thermal: map refresh failed: {e}")),
                }
            }
            st.thermal_refresh_pending = false;
            changed = true;
        }

        let live = read_i32(&sconfig_path());

        // MIUI performance mode is an explicit user override. Release our
        // custom index (or an unknown/default value) to MIUI's performance
        // scenario before the normal yield check.
        let perf = miui_performance_mode();
        if perf {
            if (st.holds.thermal_held || marker_path().exists())
                && !self.persist_thermal_before_write(st)
            {
                return changed;
            }
            let forced = match force_perf_stock(live) {
                Ok(v) => v,
                Err(e) => {
                    self.log_event(format!("thermal: performance-mode release failed: {e}"));
                    return changed;
                }
            };
            let y = forced.or(Some(1));
            if st.holds.thermal_yield != y {
                self.log_event("thermal: yielding to MIUI performance mode".into());
                st.holds.thermal_yield = y;
                changed = true;
            }
            return changed;
        }

        let yield_now = live.map(thermal_scenario::is_yield).unwrap_or(false);

        // Yield/performance checks must run even while selection is backing
        // off; otherwise a camera/perf scenario can be ignored for minutes.
        if yield_now {
            let y = yield_value(live, perf);
            if st.holds.thermal_yield != y {
                self.log_event(format!(
                    "thermal: yielding to MIUI {}",
                    yield_reason(live, perf)
                ));
                st.holds.thermal_yield = y;
                changed = true;
            }
            return changed;
        }
        if st.thermal_retry.blocks(desired.id(), Instant::now()) {
            return changed;
        }

        if !st.holds.thermal_held {
            let marker_before = marker_path().exists();
            // A leftover custom/legacy index is not a user value; the safe
            // restore point is stock normal. Every decoded MIUI scenario is
            // preserved, including offsets with no literal vendor-map entry.
            let saved = initial_saved_sconfig(live, marker_before);
            let previous = st.holds.clone();
            st.holds.thermal_held = true;
            st.holds.thermal_saved_sconfig = Some(saved);
            st.holds.thermal_yield = None;
            if !self.persist_thermal_before_write(st) {
                st.holds = previous;
                return changed;
            }
            changed = true;

            match install_all() {
                Ok(src) => {
                    if src == "fallback" {
                        self.log_event("thermal: using fallback stock map".into());
                    }
                }
                Err(e) => {
                    self.thermal_failed(st, desired.id(), format!("install failed: {e}"));
                    let _ = remove_owned();
                    return changed;
                }
            }

            match select_profile(desired) {
                SelectOutcome::Ok => {
                    self.thermal_recovered(st);
                    self.log_event(format!(
                        "thermal: {} active (sconfig {})",
                        desired.id(),
                        desired.sconfig()
                    ));
                    st.holds.thermal_profile = Some(desired.id().into());
                    if let Some(v) = post_select_yield(desired.sconfig()) {
                        self.log_event(format!(
                            "thermal: MIUI preempted after select {v} ({}); yielding",
                            thermal_scenario::label(v)
                        ));
                        st.holds.thermal_yield = Some(v);
                    }
                }
                SelectOutcome::Preempted(v) => {
                    self.thermal_recovered(st);
                    self.log_event(format!(
                        "thermal: MIUI took sconfig {v} ({}); yielding",
                        thermal_scenario::label(v)
                    ));
                    st.holds.thermal_yield = Some(v);
                }
                SelectOutcome::Failed(e) => {
                    self.repair_after_select_failure(&e);
                    self.thermal_failed(st, desired.id(), format!("select failed: {e}"));
                    let saved = st.holds.thermal_saved_sconfig.unwrap_or(0);
                    // Never remove the map while sconfig may still point at
                    // one of our indices; mi_thermald keeps the last config
                    // for a missing entry. Keep the takeover for a retry.
                    match set_sconfig_verified(saved) {
                        Ok(()) => {
                            let _ = remove_owned();
                            st.holds.thermal_held = false;
                            st.holds.thermal_saved_sconfig = None;
                            st.holds.thermal_profile = None;
                        }
                        Err(restore) => {
                            self.log_event(format!(
                                "thermal: sconfig fallback failed: {restore}; keeping map"
                            ));
                        }
                    }
                }
            }
            return changed;
        }

        // Already installed: repair missing files, then keep the selected
        // profile in sync.
        if !marker_path().exists() {
            if !self.persist_thermal_before_write(st) {
                return changed;
            }
            match install_all() {
                Ok(src) => {
                    self.log_event(format!("thermal: reinstalled ({src})"));
                    changed = true;
                }
                Err(e) => {
                    self.log_event(format!("thermal: reinstall failed: {e}"));
                    return changed;
                }
            }
        }

        if st.holds.thermal_yield.take().is_some() {
            changed = true;
        }

        let target = desired.sconfig();
        let profile_changed = st.holds.thermal_profile.as_deref() != Some(desired.id());
        if live != Some(target) || profile_changed {
            if !self.persist_thermal_before_write(st) {
                return changed;
            }
            if let Some(v) = live {
                if !is_custom_index(v) && !thermal_scenario::is_default(v) {
                    self.log_event(format!(
                        "thermal: keeping custom over MIUI {} (sconfig {v})",
                        thermal_scenario::label(v)
                    ));
                }
            }
            match select_profile(desired) {
                SelectOutcome::Ok => {
                    self.thermal_recovered(st);
                    self.log_event(format!(
                        "thermal: {} active (sconfig {})",
                        desired.id(),
                        target
                    ));
                    st.holds.thermal_profile = Some(desired.id().into());
                    changed = true;
                    if let Some(v) = post_select_yield(target) {
                        self.log_event(format!(
                            "thermal: MIUI preempted after select {v} ({}); yielding",
                            thermal_scenario::label(v)
                        ));
                        st.holds.thermal_yield = Some(v);
                    }
                }
                SelectOutcome::Preempted(v) => {
                    self.thermal_recovered(st);
                    self.log_event(format!(
                        "thermal: MIUI took sconfig {v} ({}); yielding",
                        thermal_scenario::label(v)
                    ));
                    st.holds.thermal_yield = Some(v);
                }
                SelectOutcome::Failed(e) => {
                    self.repair_after_select_failure(&e);
                    self.thermal_failed(st, desired.id(), format!("select failed: {e}"));
                }
            }
        }
        changed
    }

    /// Persist the restore point before any thermal device write. A held
    /// takeover without a captured `sconfig` is not safe to reassert.
    fn persist_thermal_before_write(&self, st: &State) -> bool {
        if st.holds.thermal_held && st.holds.thermal_saved_sconfig.is_none() {
            self.log_event(
                "thermal hold persistence refused: sconfig restore point is missing".into(),
            );
            return false;
        }
        self.persist(st).is_ok()
    }

    /// A failed select is often a corrupted owned config (truncated/garbled
    /// while the marker survived). Rewrite the four configs + map once; the
    /// directory inotify then reloads the current `sconfig` without another
    /// select. The normal backoff still decides when to retry.
    fn repair_after_select_failure(&self, err: &str) {
        if !marker_path().exists() {
            return;
        }
        match install_all() {
            Ok(src) => self.log_event(format!(
                "thermal: select failed ({err}); configs reinstalled ({src})"
            )),
            Err(e) => self.log_event(format!(
                "thermal: select failed ({err}); reinstall failed: {e}"
            )),
        }
    }

    fn thermal_failed(&self, st: &mut State, desired: &str, msg: String) {
        let retry_in = st.thermal_retry.failed(desired, Instant::now());
        self.log_event(format!("thermal: {msg}; retry in {}s", retry_in.as_secs()));
    }

    fn thermal_recovered(&self, st: &mut State) {
        st.thermal_retry.recovered();
    }
}
