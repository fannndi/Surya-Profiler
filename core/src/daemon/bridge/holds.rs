//! Hold/restore state machine + persisted restore points.
//!
//! Three surfaces can be held: thermal takeover, MIUI saver and R&D input suspend.
//! - MIUI Battery Saver (`global low_power`) while a powersave-mapped app is
//!   in front;
//! - the R&D charger-input suspend (`input_suspend=1`);
//! - the per-profile thermal takeover.
//!
//! The first hold captures the live value as the restore point; the release
//! writes it back. While a saver hold is active the arbiter must see the
//! USER's value, not our transient write (attribution).

use serde::{Deserialize, Serialize};

/// Outcome of releasing every bridge hold for Service OFF / clean exit.
///
/// Every surface must be restored successfully before the service may stop.
/// A `false` field means the live device may still carry our value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseReport {
    pub saver_ok: bool,
    pub input_suspend_ok: bool,
    pub thermal_ok: bool,
    /// Thread placement (opt-in) captured affinity restore.
    #[serde(default)]
    pub affinity_ok: bool,
}

impl Default for ReleaseReport {
    fn default() -> Self {
        ReleaseReport {
            saver_ok: true,
            input_suspend_ok: true,
            thermal_ok: true,
            affinity_ok: true,
        }
    }
}

impl ReleaseReport {
    pub fn ok(&self) -> bool {
        self.saver_ok && self.input_suspend_ok && self.thermal_ok && self.affinity_ok
    }

    /// Machine-readable failure names for logs/diagnostics.
    pub fn failures(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if !self.saver_ok {
            out.push("saver");
        }
        if !self.input_suspend_ok {
            out.push("input_suspend");
        }
        if !self.thermal_ok {
            out.push("thermal");
        }
        if !self.affinity_ok {
            out.push("affinity");
        }
        out
    }
}

#[derive(Debug)]
pub enum SaverAction {
    None,
    TurnOn,
    Keep,
    Restore,
}

/// R&D input-suspend action (`input_suspend` node).
#[derive(Debug)]
pub enum InputSuspendAction {
    None,
    Keep,
    /// Suspend input (write 1): the device runs on battery.
    Engage,
    /// Release: write the captured value back (None = assume "0").
    Release(Option<String>),
}

/// Hold/restore state machine (pure, unit tested).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Holds {
    pub saver_held: bool,
    pub saver_saved: bool,
    pub input_suspend_held: bool,
    /// The node value before the R&D suspend (None = assume "0").
    pub input_suspend_saved: Option<String>,
    /// True while our custom thermal map/configs are installed and owned.
    pub thermal_held: bool,
    /// `sconfig` value before the takeover (usually 0).
    pub thermal_saved_sconfig: Option<i32>,
    /// Last custom profile selected (`balance`/`game`/`powersave`/`sleep`).
    pub thermal_profile: Option<String>,
    /// A MIUI scenario we are yielding to (e.g. 12 = camera).
    pub thermal_yield: Option<i32>,
}

impl Holds {
    /// True when any surface is currently held by the bridge.
    pub fn any_held(&self) -> bool {
        self.saver_held || self.input_suspend_held || self.thermal_held
    }

    /// `live` = current battery-saver state; `want` = bridge wants ON.
    pub fn request_saver(&self, live: bool, want: bool) -> (Holds, SaverAction) {
        match (want, self.saver_held) {
            (true, false) => (
                Holds {
                    saver_held: true,
                    saver_saved: live,
                    ..self.clone()
                },
                if live {
                    SaverAction::Keep
                } else {
                    SaverAction::TurnOn
                },
            ),
            // While held, re-assert our value if something (the user, MIUI)
            // turned it off; the captured pre-hold value remains the restore
            // point. This is what "saver follow" means for a mapped app.
            (true, true) => {
                if live {
                    (self.clone(), SaverAction::Keep)
                } else {
                    (self.clone(), SaverAction::TurnOn)
                }
            }
            (false, true) => (
                Holds {
                    saver_held: false,
                    ..self.clone()
                },
                SaverAction::Restore,
            ),
            (false, false) => (self.clone(), SaverAction::None),
        }
    }

    /// The saver value the arbiter should treat as USER intent: while we
    /// hold the flag for a mapped app, the captured value stands in.
    pub fn user_saver(&self, live: bool) -> bool {
        if self.saver_held {
            self.saver_saved
        } else {
            live
        }
    }

    /// `live` = current `input_suspend`; `want` = the R&D switch wants the
    /// input suspended. Capture on the first engage, write back on release
    /// (a missing capture assumes "0" — stock).
    pub fn request_input_suspend(
        &self,
        live: Option<String>,
        want: bool,
    ) -> (Holds, InputSuspendAction) {
        match (want, self.input_suspend_held) {
            (true, false) => (
                Holds {
                    input_suspend_held: true,
                    input_suspend_saved: live.clone(),
                    ..self.clone()
                },
                if live.as_deref() == Some("1") {
                    InputSuspendAction::Keep
                } else {
                    InputSuspendAction::Engage
                },
            ),
            (true, true) => {
                if live.as_deref() == Some("1") {
                    (self.clone(), InputSuspendAction::Keep)
                } else {
                    (self.clone(), InputSuspendAction::Engage)
                }
            }
            (false, true) => (
                Holds {
                    input_suspend_held: false,
                    input_suspend_saved: None,
                    ..self.clone()
                },
                InputSuspendAction::Release(self.input_suspend_saved.clone()),
            ),
            (false, false) => (self.clone(), InputSuspendAction::None),
        }
    }
}

/// Persisted holds (crash-safe restore points), `holds.json` in the state dir.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HoldsFile {
    #[serde(default)]
    pub saver_held: bool,
    #[serde(default)]
    pub saver_saved: bool,
    #[serde(default, alias = "bypass_held")]
    pub input_suspend_held: bool,
    #[serde(default, alias = "bypass_saved")]
    pub input_suspend_saved: Option<String>,
    #[serde(default)]
    pub thermal_held: bool,
    #[serde(default)]
    pub thermal_saved_sconfig: Option<i32>,
    #[serde(default)]
    pub thermal_profile: Option<String>,
    #[serde(default)]
    pub thermal_yield: Option<i32>,
}

/// Read-only hold view for diagnostics (`diag` command); never mutates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HoldsInfo {
    pub saver_held: bool,
    pub saver_saved: bool,
    pub input_suspend_held: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_suspend_saved: Option<String>,
    pub thermal_held: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thermal_saved_sconfig: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thermal_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thermal_yield: Option<i32>,
    /// Opt-in per-app thread placement (Game-mapped apps).
    #[serde(default)]
    pub affinity_held: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affinity_app: Option<String>,
    #[serde(default)]
    pub affinity_threads: usize,
}

#[cfg(test)]
mod tests;
