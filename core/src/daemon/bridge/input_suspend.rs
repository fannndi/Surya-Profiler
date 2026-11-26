//! R&D charger-input suspend: `input_suspend=1` votes 0 mA on the charger
//! input (USER_VOTER), so the device runs on battery while the switch is on.
//!
//! This is deliberately a single global switch, not a per-app rule and not a
//! charge guard: no floor, no hysteresis, no automatic policy. The bridge
//! captures the previous node value and releases it when the switch is turned
//! off, the service stops, or the daemon exits. Every write passes the
//! engine's catalog guard (`ALLOWED_EXACT`, Baseline).

use super::holds::InputSuspendAction;
use super::{Bridge, State, SyncCtx};
use crate::engine::apply::guarded_write;
use crate::engine::env::default_root;
use std::fs;

#[cfg(test)]
mod tests;

/// Catalog path for the single audited charge node. Writes always use this
/// literal (the catalog guard checks membership); reads resolve it through
/// the simulation root in [`node`].
pub(super) const INPUT_SUSPEND_PATH: &str = "/sys/class/power_supply/battery/input_suspend";

fn node() -> String {
    default_root()
        .join(INPUT_SUSPEND_PATH.trim_start_matches('/'))
        .display()
        .to_string()
}

fn read_live() -> Option<String> {
    fs::read_to_string(node())
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub(super) fn write_verified(value: &str) -> Result<(), String> {
    guarded_write(INPUT_SUSPEND_PATH, value)?;
    for _ in 0..4 {
        if read_live().as_deref() == Some(value) {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let got = read_live();
    Err(format!(
        "input_suspend read-back is {got:?}, wanted {value:?}"
    ))
}

/// Stock value to release for an orphan, or `None` when the node is already
/// safe or a persisted hold owns it.
pub(super) fn orphan_release_value(held: bool, live: Option<&str>) -> Option<&'static str> {
    (!held && live == Some("1")).then_some("0")
}

impl Bridge {
    /// Crash recovery: `input_suspend` is an app-only R&D node and its stock
    /// value is 0. If a previous run wrote 1 but the hold was lost, release it
    /// at startup so the phone does not stay on battery while plugged in.
    ///
    /// The main loop must call this unconditionally at startup: the R&D switch
    /// being enabled does not make an unowned live 1 safe. A persisted hold is
    /// left untouched for the normal release path.
    pub(crate) fn reconcile_orphan_input_suspend(&self) {
        let st = self.lock();
        let live = read_live();
        let Some(value) = orphan_release_value(st.holds.input_suspend_held, live.as_deref()) else {
            return;
        };
        match write_verified(value) {
            Ok(()) => self.log_event("R&D input suspend orphan released".into()),
            Err(e) => self.log_event(format!("R&D input suspend orphan release failed: {e}")),
        }
    }

    /// One input-suspend sync; caller holds the bridge lock (like the saver sync).
    pub(super) fn sync_input_suspend(&self, st: &mut State, ctx: &SyncCtx) -> bool {
        self.sync_input_suspend_with_live(st, ctx, read_live())
    }

    fn sync_input_suspend_with_live(
        &self,
        st: &mut State,
        ctx: &SyncCtx,
        live: Option<String>,
    ) -> bool {
        let (next, action) = st.holds.request_input_suspend(live, ctx.rnd_input_suspend);
        match action {
            InputSuspendAction::Engage => {
                // Write-ahead the captured restore point before touching the
                // charger node; a crash cannot leave input_suspend=1 unowned.
                let previous = st.holds.clone();
                st.holds = next;
                if self.persist(st).is_err() {
                    st.holds = previous;
                    return false;
                }
                match write_verified("1") {
                    Ok(()) => {
                        self.log_event("R&D input suspend ON".into());
                        true
                    }
                    Err(e) => {
                        self.log_event(format!("R&D input suspend failed: {e}"));
                        false
                    }
                }
            }
            InputSuspendAction::Release(saved) => {
                let value = saved.unwrap_or_else(|| "0".into());
                match write_verified(&value) {
                    Ok(()) => {
                        self.log_event(format!("R&D input suspend OFF (node {value})"));
                        st.holds = next;
                        true
                    }
                    Err(e) => {
                        self.log_event(format!("R&D input suspend release failed: {e}"));
                        false
                    }
                }
            }
            InputSuspendAction::Keep => {
                let changed = next != st.holds;
                st.holds = next;
                changed
            }
            InputSuspendAction::None => false,
        }
    }
}
