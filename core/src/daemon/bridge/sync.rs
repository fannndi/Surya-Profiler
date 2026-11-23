//! Sync context, the pure saver gate and the MIUI saver IO.
//!
//! Inherent impl for [`Bridge`](super::Bridge) (defined in `mod.rs`); the
//! state machine lives in `holds.rs`. Every sync runs under the bridge
//! Mutex held by the caller.

use super::holds::SaverAction;
use super::{Bridge, State};
use crate::daemon::settings;
use std::collections::BTreeMap;

/// Everything the bridge sync needs about the current world.
#[derive(Debug, Clone, PartialEq)]
pub struct SyncCtx {
    /// Service enabled. A disabled service must never re-engage a hold
    /// (Service OFF recovery / stale supervisor tick).
    pub enabled: bool,
    pub last_real: Option<String>,
    pub screen_on: bool,
    pub locked: bool,
    pub dynamic: bool,
    /// Validated app map (only the three user profiles).
    pub app_map: BTreeMap<String, String>,
    /// Split screen active: dual-app mode disables the per-app saver follow.
    pub multi_window: bool,
    /// Last polled MIUI saver flag (LiveFlags cache; avoids a subprocess per sync).
    pub live_saver: bool,
    /// Profile the app/daemon is applying (thermal takeover follows it).
    pub profile: Option<String>,
    /// User switch: take over MIUI thermal per profile (default on).
    pub custom_thermal: bool,
    /// R&D-only global charger-input suspend.
    pub rnd_input_suspend: bool,
}

impl Bridge {
    pub(super) fn sync_saver(&self, st: &mut State, ctx: &SyncCtx) -> bool {
        let want = saver_want(ctx);
        // On the first engage, read the live value synchronously; the polled
        // cache can be up to one cadence stale and would capture the wrong
        // restore point (e.g. re-enable the user's saver on Service OFF).
        let live = if want && !st.holds.saver_held {
            settings::read_bool("global", settings::SAVER_KEY).unwrap_or(ctx.live_saver)
        } else {
            ctx.live_saver
        };
        let (next, action) = st.holds.request_saver(live, want);
        let changed = next != st.holds;
        match action {
            SaverAction::TurnOn => {
                // Write-ahead: a crash after the device write but before the
                // hold was persisted would otherwise leave low_power=1 with no
                // restore point. Persist first, then write; a failed write is
                // retried by the next sync (the hold stays engaged).
                st.holds = next;
                self.persist(st);
                match settings::put("global", settings::SAVER_KEY, "1") {
                    Ok(()) => {
                        self.log_event(format!(
                            "MIUI saver ON ({})",
                            ctx.last_real.as_deref().unwrap_or("?")
                        ));
                        self.set_live_saver(true);
                        true
                    }
                    Err(e) => {
                        self.log_event(format!("MIUI saver write failed: {e}"));
                        false
                    }
                }
            }
            SaverAction::Restore => {
                let value = if next.saver_saved { "1" } else { "0" };
                match settings::put("global", settings::SAVER_KEY, value) {
                    Ok(()) => {
                        self.log_event("MIUI saver restored".into());
                        self.set_live_saver(next.saver_saved);
                        st.holds = next;
                        true
                    }
                    Err(e) => {
                        self.log_event(format!("MIUI saver restore failed: {e}"));
                        false
                    }
                }
            }
            SaverAction::Keep => {
                st.holds = next;
                changed
            }
            SaverAction::None => false,
        }
    }
}

// --- pure gate (unit tested; the sync function above only wires IO) ---------

/// The profile the app map assigns to the current foreground, if any.
fn mapped_profile(ctx: &SyncCtx) -> Option<&str> {
    ctx.last_real
        .as_deref()
        .and_then(|p| ctx.app_map.get(p))
        .map(|s| s.as_str())
}

/// Saver follow wants the saver ON: Dynamic ON + a powersave-mapped app in
/// front + screen visible & unlocked.
fn saver_want(ctx: &SyncCtx) -> bool {
    ctx.dynamic
        && !ctx.multi_window
        && mapped_profile(ctx) == Some("powersave")
        && ctx.screen_on
        && !ctx.locked
}

#[cfg(test)]
mod tests;
