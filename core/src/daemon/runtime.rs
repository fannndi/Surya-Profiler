//! The event side of the main loop: state emission + apply/restore events.
//!
//! Inherent impl for [`Runtime`](super::Runtime) (defined in `mod.rs`).
//! The impl blocks are split for file size: this one holds logging +
//! snapshot emission + applied/restored handling; see `commands.rs`
//! (dispatch) and `evaluate.rs` (decision + supervisor).

use super::power::{PowerSample, PowerTick};
use super::worker;
use super::worker::RestoredEvent;
use super::Runtime;
use super::{Event, Snapshot};
use crate::engine::env::EnvSnapshot;

impl Runtime {
    pub(super) fn log(&self, msg: &str) {
        self.publisher.log(msg);
    }

    pub(super) fn emit_state(&mut self, force: bool) {
        let snap = Snapshot {
            screen_on: self.screen_on,
            locked: self.locked,
            multi_window: self.multi_window,
            second_window: self.second_window.clone(),
            foreground: self.last_fg.clone(),
            active: self.active.clone(),
            reason: self.reason.clone(),
            src_pkg: self.src_pkg.clone(),
        };
        if force || snap != self.last_state {
            self.last_state = snap.clone();
            self.publisher.emit(&Event::State { state: snap });
        }
    }

    pub(super) fn emit_decision(
        &self,
        trigger: &str,
        action: &str,
        profile: Option<String>,
        reason: Option<String>,
    ) {
        self.publisher.emit(&Event::Decision {
            trigger: trigger.into(),
            action: action.into(),
            profile,
            reason,
        });
    }

    /// Fresh environment sample: keep it for diagnostics; forward to the app
    /// only when something changed (twice-a-minute silence otherwise).
    pub(super) fn on_env(&mut self, snap: EnvSnapshot) {
        if snap == self.env {
            return;
        }
        self.env = snap;
        self.publisher.emit(&Event::Env {
            env: Box::new(self.env.clone()),
        });
    }

    /// Fast power tick: feed the ring and forward the sample when the Power
    /// screen is watching. A missing current/voltage node still updates the
    /// live sample (status/temperature) but creates no history bucket.
    pub(super) fn on_power(&mut self, tick: PowerTick) {
        // Never attribute a locked/off panel to the last app: the lock screen
        // is not that app's foreground time.
        let pkg = if self.screen_on && !self.locked {
            self.last_real.clone()
        } else {
            None
        };
        let sample = PowerSample::from_reading(
            crate::daemon::power::now_ms(),
            tick.reading,
            self.screen_on,
            pkg,
            self.active.clone(),
        );
        let bucket = self.power.record(&sample, tick.interval_ms);
        if self.power_watch {
            self.publisher.emit(&Event::PowerLive {
                sample: Box::new(sample),
                bucket: bucket.map(Box::new),
            });
        }
    }

    pub(super) fn on_applied(&mut self, ev: worker::AppliedEvent) {
        if ev.ok {
            if ev.wrote == 0 {
                // Periodic confirmations are silent; a real switch writes.
            } else if ev.watchdog {
                self.log(&format!(
                    "auto-revive: healed {} keys for {}",
                    ev.wrote, ev.profile
                ));
                self.stats.record_heal(ev.wrote);
            } else {
                self.log(&format!(
                    "apply {}: done in {}ms (settle {}ms)",
                    ev.profile, ev.ms, ev.settle_ms
                ));
            }
            // a real switch (not an "in place" confirmation) is history
            if self.active.as_deref() != Some(ev.profile.as_str()) {
                self.stats
                    .record(self.active.as_deref(), &ev.profile, &ev.reason, &self.env);
            }
            self.active = Some(ev.profile.clone());
            self.reason = Some(ev.reason.clone());
            self.src_pkg = ev.src_pkg.clone();
            // Only a confirmed apply counts as reconciled; a failed forced
            // apply must stay eligible for the next startup/periodic re-plan.
            self.reconciled = true;
        } else {
            self.log(&format!(
                "apply {} failed after retry ({})",
                ev.profile, ev.failed
            ));
            self.reason = Some(format!("apply failed ({})", ev.failed));
            // Re-arm the forced re-plan so the next 15 s tick retries instead
            // of accepting the stale profile as "in place".
            self.reconciled = false;
        }
        self.publisher.emit(&Event::Applied {
            profile: ev.profile,
            reason: ev.reason,
            src_pkg: ev.src_pkg,
            ok: ev.ok,
            applied: ev.applied,
            wrote: ev.wrote,
            verified: ev.verified,
            failed: ev.failed,
            ms: ev.ms,
            settle_ms: ev.settle_ms,
        });
        // Thermal follows the profile that actually applied (not just the
        // decision); a failed apply leaves the previous target in place.
        let _ = self.sync_tx.send(self.bridge_ctx(self.active.clone()));
        self.emit_state(false);
    }

    /// Service-off restore finished. A successful restore leaves the daemon
    /// in `restoring` state (no further applies/syncs) until the app stops
    /// the service and stdin closes. A failure resumes normal operation so
    /// the user can retry from a live service.
    pub(super) fn on_restored(&mut self, ev: RestoredEvent) {
        let ok = ev.ok;
        if ok {
            if let Some(prev) = self.active.clone() {
                self.stats
                    .record(Some(&prev), "stock", "restore", &self.env);
            }
            self.active = None;
            self.reason = None;
            self.src_pkg = None;
            self.restoring = true;
            self.log(&format!(
                "restore: ok=true wrote={} verified={} failed={} holds={:?}",
                ev.wrote, ev.verified, ev.failed, ev.release
            ));
        } else {
            self.restoring = false;
            self.bridge.resume();
            self.log(&format!(
                "restore: ok=false wrote={} verified={} failed={} holds={:?} failed_surfaces={:?}",
                ev.wrote,
                ev.verified,
                ev.failed,
                ev.release,
                ev.release.failures()
            ));
        }
        self.publisher.emit_reliable(&Event::Restored {
            ok,
            wrote: ev.wrote,
            verified: ev.verified,
            failed: ev.failed,
            release: ev.release,
        });
        self.emit_state(false);
    }
}
