//! Global display-scale hold (`display.json`).
//!
//! `wm size` + `wm density` are runtime overrides that persist beyond the
//! daemon. We capture the user's exact originals (including a density such as
//! 400) and restore them on Off / Service OFF / daemon exit.

use super::presets;
use super::wm::{Host, WmSnapshot};
use crate::logging::{self, Level};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct DisplayState {
    /// Applied preset id (`"hd"`), `None` = native.
    #[serde(default)]
    pub applied: Option<String>,
    /// Exact original overrides captured before the first apply.
    #[serde(default)]
    pub original: Option<WmSnapshot>,
    /// Transient: the live wm values were verified after the last start.
    #[serde(skip)]
    verified: bool,
}

impl DisplayState {
    pub(crate) fn active(&self) -> bool {
        self.applied.is_some() || self.original.is_some()
    }
}

/// Reconciles the desired preset; returns true when persisted state changed.
pub(crate) fn sync(st: &mut DisplayState, desired_raw: &str) -> bool {
    let desired = match presets::canonical(desired_raw) {
        Some(v) => v,
        None => {
            warn(format!(
                "display_scale '{desired_raw}' is unknown; keeping native"
            ));
            "off"
        }
    };
    if st.applied.as_deref() == Some(desired) {
        if desired != "off" && !st.verified {
            let host = Host::detect();
            if let Some(p) = presets::find(desired) {
                let size = format!("{}x{}", p.width, p.height);
                let want_density = p.density.to_string();
                let mismatch = host.read().is_none_or(|live| {
                    live.size.as_deref() != Some(size.as_str())
                        || live.density.as_deref() != Some(want_density.as_str())
                });
                if !mismatch {
                    st.verified = true;
                } else {
                    match host.apply(&size, p.density) {
                        Ok(()) => {
                            st.verified = true;
                            info(format!("display: re-asserted {size} @ {}dpi", p.density));
                            return true;
                        }
                        Err(e) => warn(format!("display re-assert failed: {e}")),
                    }
                }
            }
        }
        return false;
    }
    if desired == "off" {
        return match release(st) {
            Ok(changed) => changed,
            Err(e) => {
                warn(format!("display restore failed: {e}"));
                false
            }
        };
    }

    let host = Host::detect();
    if st.original.is_none() {
        let Some(original) = host.read() else {
            warn("wm size/density capture failed; not applying display preset".into());
            return false;
        };
        st.original = Some(original);
    }
    let Some(p) = presets::find(desired) else {
        return false;
    };
    let size = format!("{}x{}", p.width, p.height);
    match host.apply(&size, p.density) {
        Ok(()) => {
            info(format!("display: {size} @ {}dpi ({})", p.density, p.id));
            st.applied = Some(desired.to_string());
            st.verified = true;
            true
        }
        Err(e) => {
            warn(format!("display apply {size} @ {} failed: {e}", p.density));
            // A partial write is possible; put the user's originals back.
            if let Some(original) = st.original.clone() {
                if host.restore(&original).is_ok() {
                    st.original = None;
                    st.applied = None;
                }
            }
            false
        }
    }
}

/// Restores the captured originals; a failure keeps the hold for retry.
pub(crate) fn release(st: &mut DisplayState) -> Result<bool, String> {
    if !st.active() {
        return Ok(false);
    }
    let host = Host::detect();
    let original = st.original.clone().unwrap_or_default();
    host.restore(&original)?;
    info(format!("display: restored {}", describe(&original)));
    st.applied = None;
    st.original = None;
    st.verified = false;
    Ok(true)
}

fn describe(s: &WmSnapshot) -> String {
    let size = s.size.as_deref().unwrap_or("physical");
    let density = s.density.as_deref().unwrap_or("physical");
    format!("size {size}, density {density}")
}

fn info(msg: String) {
    logging::write(Level::Info, "display", &msg);
}

fn warn(msg: String) {
    logging::write(Level::Warn, "display", &msg);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_native() {
        let st = DisplayState::default();
        assert!(!st.active());
        assert!(presets::find("hd").is_some());
        assert_eq!(presets::PRESETS.len(), 7);
    }
}
