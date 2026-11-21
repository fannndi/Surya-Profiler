//! One thermal sync: install, yield, select, repair.

use super::release::set_sconfig_verified;
use super::*;
use std::time::{Duration, Instant};

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

        let mut changed = false;

        // Refresh map/configs once after a daemon restart so a ROM update or
        // an upgrade from the 20–23 era is picked up even when holds.json
        // says thermal_held=true.
        if st.thermal_refresh_pending {
            st.thermal_refresh_pending = false;
            if st.holds.thermal_held || marker_path().exists() {
                match install_all() {
                    Ok(src) => self.log_event(format!("thermal: map refreshed ({src})")),
                    Err(e) => self.log_event(format!("thermal: map refresh failed: {e}")),
                }
                changed = true;
            }
        }

        let live = read_i32(&sconfig_path());

        // MIUI performance mode is an explicit user override. Release our
        // custom index (or an unknown/default value) to MIUI's performance
        // scenario before the normal yield check.
        let perf = miui_performance_mode();
        if perf {
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
        if let Some(deadline) = st.thermal_retry_at {
            if Instant::now() < deadline {
                return changed;
            }
            st.thermal_retry_at = None;
        }

        if !st.holds.thermal_held {
            let marker_before = marker_path().exists();
            match install_all() {
                Ok(src) => {
                    if src == "fallback" {
                        self.log_event("thermal: using fallback stock map".into());
                    }
                }
                Err(e) => {
                    self.thermal_failed(st, format!("install failed: {e}"));
                    let _ = remove_owned();
                    return changed;
                }
            }
            st.holds.thermal_held = true;
            // A leftover custom/legacy index is not a user value; the safe
            // restore point is stock normal. MIUI scenario values are kept
            // only when they have a vendor map entry.
            let saved = match live {
                Some(v) if is_custom_index(v) || (marker_before && is_legacy_index(v)) => 0,
                Some(v) if thermal_scenario::has_vendor_map_entry(v) => v,
                _ => 0,
            };
            st.holds.thermal_saved_sconfig = Some(saved);
            st.holds.thermal_yield = None;
            changed = true;

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
                    self.thermal_failed(st, format!("select failed: {e}"));
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
                    self.log_event(format!(
                        "thermal: MIUI took sconfig {v} ({}); yielding",
                        thermal_scenario::label(v)
                    ));
                    st.holds.thermal_yield = Some(v);
                }
                SelectOutcome::Failed(e) => {
                    self.thermal_failed(st, format!("select failed: {e}"));
                }
            }
        }
        changed
    }

    fn thermal_failed(&self, st: &mut State, msg: String) {
        st.thermal_failures = st.thermal_failures.saturating_add(1).min(6);
        let shift = (st.thermal_failures - 1).min(4) as u32;
        let secs = 15u64.saturating_mul(1u64 << shift).min(300);
        st.thermal_retry_at = Some(Instant::now() + Duration::from_secs(secs));
        self.log_event(format!("thermal: {msg}; retry in {secs}s"));
    }

    fn thermal_recovered(&self, st: &mut State) {
        st.thermal_failures = 0;
        st.thermal_retry_at = None;
    }
}
