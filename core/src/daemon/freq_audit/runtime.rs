//! Runtime glue for the freq floor audit (impl [`Runtime`](super::Runtime)).
//!
//! Called from `supervise` every 3 s tick; the audit itself rate-limits to
//! its sampling interval. Screen-on drift is logged once per episode and
//! never fought (MIUI's overlay releases on its own); screen-off drift is
//! enforced through `evaluate("freq_audit")` after the grace.

use super::super::Runtime;
use super::{desired_floors, live_floors, FloorAction, Observe};
use std::time::Instant;

impl Runtime {
    /// Called from `supervise` every tick; the audit rate-limits itself.
    pub(in crate::daemon) fn freq_audit_tick(&mut self) {
        let cfg = self.config.get();
        if !cfg.freq_floor_audit || !cfg.enabled || self.restoring || !self.reconciled {
            return;
        }
        let now = Instant::now();
        if !self.floor_audit.due(now) {
            return;
        }
        // MIUI performance mode legitimately raises the floor — never fight it.
        if self.env.thermal_global_mode == Some(1)
            || self.env.thermal_perf_config.is_some_and(|v| v > 0)
        {
            if self.floor_audit.mark_yield() {
                self.log("floor audit: MIUI performance mode owns the CPU floor — yielding");
            }
            return;
        }
        let Some(active) = self.active.clone() else {
            return;
        };
        let Some(desired) = desired_floors(&self.state_dir, &active) else {
            return;
        };
        let live = live_floors();
        let high = live.little_khz > desired.little_khz || live.big_khz > desired.big_khz;

        // Screen-on: MIUI's floor is a bounded overlay (wake ~10 s, launch
        // ~40 s, then it releases). Log once per episode, never fight.
        if self.screen_on {
            if high {
                if self.floor_audit.note_holder() {
                    self.log(&format!(
                        "floor audit: MIUI holds the screen-on CPU floor {}/{} kHz (profile {active}, fg {:?}) — release expected; not fighting it",
                        live.little_khz, live.big_khz, self.last_fg,
                    ));
                }
            } else if self.floor_audit.holder_noted() {
                self.log("floor audit: screen-on floor released back to the profile value");
                self.floor_audit.reset();
            }
            return;
        }

        // Screen-off: no interaction is possible, so a high floor here is
        // genuinely wrong (stuck hint / foreign tuner). Enforce ours.
        if high {
            self.log(&format!(
                "floor audit: live floor {}/{} kHz > profile floor {}/{} kHz (profile {active}, sconfig {:?})",
                live.little_khz, live.big_khz, desired.little_khz, desired.big_khz,
                self.env.thermal_sconfig,
            ));
        }
        match self.floor_audit.observe(
            Observe {
                high,
                allowed: true,
            },
            now,
        ) {
            FloorAction::None => {}
            FloorAction::Restored => {
                self.log("floor audit: floor back at the profile value");
            }
            FloorAction::Reassert => {
                if self.floor_audit.attempts() > 1 {
                    self.log(
                        "floor audit: floor still high after re-assert — external floor holder suspected",
                    );
                }
                self.log(&format!("floor audit: re-asserting the {active} floor"));
                self.evaluate("freq_audit", None);
            }
        }
    }
}
