//! Per-app downscale hold state + capture/restore (`downscale.json`).
//!
//! Persistent while Service ON (the platform override survives app restarts),
//! restored on config Off / package removal / Service OFF / daemon exit.
//! `last_checked` is transient and only forces a re-verify when a configured
//! app becomes foreground again.

use super::platform_compat::{self, Host};
use super::ratios;
use crate::logging::{self, Level};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One package we downscaled, including the value to restore.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AppliedApp {
    /// Canonical factor we wrote (`0.65`).
    pub applied: String,
    /// Previous override: `None` = native/off, `Some("0.8")` = MIUI/user.
    pub prior: Option<String>,
}

/// Persisted state (`<state_dir>/downscale.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct DownscaleState {
    #[serde(default)]
    pub apps: BTreeMap<String, AppliedApp>,
    #[serde(skip)]
    supported: Option<bool>,
    #[serde(skip)]
    last_checked: Option<String>,
}

impl DownscaleState {
    pub(crate) fn active(&self) -> bool {
        !self.apps.is_empty()
    }
}

/// Reconciles the desired map against the live holds.
/// Returns true when the persisted state changed.
pub(crate) fn sync(
    st: &mut DownscaleState,
    desired: &BTreeMap<String, String>,
    foreground: Option<&str>,
) -> bool {
    let host = Host::detect();
    let supported = *st.supported.get_or_insert_with(|| host.supported());
    if !supported {
        if st.active() {
            warn("cmd game downscale is unavailable; keeping existing holds".into());
        }
        return false;
    }

    let mut wanted = BTreeMap::new();
    for (pkg, raw) in desired {
        if !allowed_pkg(pkg) {
            warn(format!("{pkg}: protected package; render scale ignored"));
            continue;
        }
        match ratios::canonical(raw) {
            Some(ratio) => {
                wanted.insert(pkg.clone(), ratio.to_string());
            }
            None => warn(format!("{pkg}: invalid downscale ratio '{raw}'")),
        }
    }

    let mut changed = false;

    // Restore apps that are no longer configured.
    let removed: Vec<String> = st
        .apps
        .keys()
        .filter(|pkg| !wanted.contains_key(*pkg))
        .cloned()
        .collect();
    for pkg in removed {
        if restore_one(&host, &pkg, st.apps.get(&pkg).cloned()) {
            st.apps.remove(&pkg);
            changed = true;
        }
    }

    // Foreground changes trigger one verification/re-assert per app visit.
    let check = st.last_checked.as_deref() != foreground;
    st.last_checked = foreground.map(str::to_string);

    for (pkg, ratio) in &wanted {
        if let Some(rec) = st.apps.get(pkg).cloned() {
            if rec.applied == *ratio {
                if check && foreground == Some(pkg.as_str()) {
                    match host.current(pkg) {
                        Some(Some(live)) if live == *ratio => {}
                        Some(live) => {
                            info(format!(
                                "{pkg}: MIUI changed downscale to {}; re-asserting {ratio}",
                                live.map(|v| ratios::label(&v))
                                    .unwrap_or_else(|| "native".into())
                            ));
                            if let Err(e) = host.apply(pkg, ratio) {
                                warn(format!("{pkg}: re-assert failed: {e}"));
                            }
                        }
                        None => warn(format!("{pkg}: could not verify downscale")),
                    }
                }
                continue;
            }
            match host.apply(pkg, ratio) {
                Ok(()) => {
                    let old = st
                        .apps
                        .get(pkg)
                        .map(|r| r.applied.clone())
                        .unwrap_or_default();
                    if let Some(rec) = st.apps.get_mut(pkg) {
                        rec.applied = ratio.clone();
                    }
                    info(format!(
                        "{pkg}: downscale {} -> {}",
                        ratios::label(&old),
                        ratios::label(ratio)
                    ));
                    changed = true;
                }
                Err(e) => warn(format!("{pkg}: downscale change failed: {e}")),
            }
            continue;
        }

        let Some(prior) = host.current(pkg) else {
            warn(format!(
                "{pkg}: prior downscale capture failed; not applying"
            ));
            continue;
        };
        match host.apply(pkg, ratio) {
            Ok(()) => {
                info(format!(
                    "{pkg}: downscale {} (prior {})",
                    ratios::label(ratio),
                    platform_compat::prior_label(&prior)
                ));
                st.apps.insert(
                    pkg.clone(),
                    AppliedApp {
                        applied: ratio.clone(),
                        prior,
                    },
                );
                changed = true;
            }
            Err(e) => warn(format!("{pkg}: downscale apply failed: {e}")),
        }
    }

    if st.apps.is_empty() {
        st.last_checked = None;
    }
    changed
}

/// Restores every held package; a failed restore keeps the record for retry.
pub(crate) fn release(st: &mut DownscaleState) -> Result<bool, String> {
    if st.apps.is_empty() {
        return Ok(false);
    }
    let host = Host::detect();
    let mut keep = BTreeMap::new();
    let mut first_err = None;
    let mut changed = false;
    for (pkg, rec) in std::mem::take(&mut st.apps) {
        if restore_one(&host, &pkg, Some(rec.clone())) {
            changed = true;
        } else {
            if first_err.is_none() {
                first_err = Some(format!("could not restore {pkg}"));
            }
            keep.insert(pkg, rec);
        }
    }
    st.apps = keep;
    st.last_checked = None;
    match first_err {
        Some(e) => Err(e),
        None => Ok(changed),
    }
}

/// Never touch SystemUI/launcher/self; the compat change can restart them.
fn allowed_pkg(pkg: &str) -> bool {
    !matches!(
        pkg,
        "android" | "com.android.systemui" | "com.miui.home" | "com.suryaprofiler"
    )
}

fn restore_one(host: &Host, pkg: &str, rec: Option<AppliedApp>) -> bool {
    let Some(rec) = rec else {
        return true;
    };
    let result = match rec.prior.as_deref() {
        Some(prior) => host.apply(pkg, prior),
        None => host.disable(pkg),
    };
    match result {
        Ok(()) => {
            info(format!(
                "{pkg}: downscale restored to {}",
                platform_compat::prior_label(&rec.prior)
            ));
            true
        }
        Err(e) => {
            warn(format!("{pkg}: downscale restore failed: {e}"));
            false
        }
    }
}

fn info(msg: String) {
    logging::write(Level::Info, "downscale", &msg);
}

fn warn(msg: String) {
    logging::write(Level::Warn, "downscale", &msg);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protects_system_ui_launcher_and_self() {
        for pkg in [
            "android",
            "com.android.systemui",
            "com.miui.home",
            "com.suryaprofiler",
        ] {
            assert!(!allowed_pkg(pkg), "{pkg}");
        }
        assert!(allowed_pkg("com.netmarble.skiagb"));
        assert!(allowed_pkg("com.ss.android.ugc.trill"));
    }
}
