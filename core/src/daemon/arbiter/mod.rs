//! Pure decision table.
//!
//! Model: the last manually chosen profile is the universal BASE; apps with
//! a per-app mapping override it while they are in front; screen-off always
//! applies the sleep profile; a visible split screen uses the middle ground.
//! Pure data in, one decision out — no IO.

/// Screen-off profile.
pub const SLEEP_PROFILE: &str = "sleep";
/// MIUI/Android battery saver forces the frugal base.
pub const SAVER_PROFILE: &str = "powersave";
/// Dual visible apps (split/floating) are always tuned Balanced.
pub const MULTI_WINDOW_PROFILE: &str = "balance";

#[derive(Debug, Clone, PartialEq)]
pub struct ArbiterInput {
    pub service_enabled: bool,
    pub screen_on: bool,
    pub keyguard_locked: bool,
    pub foreground_pkg: Option<String>,
    pub app_map: std::collections::BTreeMap<String, String>,
    pub base_profile: String,
    pub sleep_profile: String,
    /// MIUI battery saver / Android battery saver — forces the frugal base.
    pub saver_on: bool,
    /// Split screen / floating window active — dual-app concurrency.
    pub multi_window: bool,
    /// Dynamic Profile: ON = mapped app overrides the base; OFF = base wins.
    pub dynamic_profile: bool,
}

impl Default for ArbiterInput {
    fn default() -> Self {
        ArbiterInput {
            service_enabled: true,
            screen_on: true,
            keyguard_locked: false,
            foreground_pkg: None,
            app_map: Default::default(),
            base_profile: "balance".into(),
            sleep_profile: SLEEP_PROFILE.into(),
            saver_on: false,
            multi_window: false,
            dynamic_profile: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// Leave whatever is active in place.
    None,
    /// Apply `profile`; `reason` is a stable machine label.
    Apply { profile: String, reason: String },
}

/// Packages that must not trigger a switch: system chrome and dialogs
/// appear "in front" of the real foreground app. IMEs match by substring.
/// The launcher and Surya Profiler itself are NOT transient: they are normal
/// full-screen apps. Leaving a mapped app to either one must return to the
/// universal base (Surya Profiler is never offered in the Apps Profile list).
const TRANSIENT_PACKAGES: &[&str] = &[
    "com.android.systemui",
    "com.android.permissioncontroller",
    "com.google.android.permissioncontroller",
    "com.lbe.security.miui", // MIUI permission dialogs
    "android",
    "com.baidu.input_mi", // MIUI-common IME without "inputmethod" in the name
];

pub fn is_transient(pkg: Option<&str>) -> bool {
    match pkg {
        None => true,
        Some(p) => {
            TRANSIENT_PACKAGES.contains(&p) || p.to_ascii_lowercase().contains("inputmethod")
        }
    }
}

/// Foreground package a decision should use.
///
/// `last_fg` is the raw current foreground (including overlays); `last_real`
/// is the last non-transient app underneath. A real foreground always wins,
/// even if it is our own unmapped app. Only a genuine overlay (SystemUI, IME,
/// permission dialog) falls back to the app underneath — otherwise YouTube's
/// mapping would stay active while Surya Profiler is visibly in front.
pub fn decision_foreground(last_fg: Option<&str>, last_real: Option<&str>) -> Option<String> {
    match last_fg {
        Some(pkg) if !is_transient(Some(pkg)) => Some(pkg.to_string()),
        _ => last_real.map(str::to_string),
    }
}

pub fn decide(i: &ArbiterInput) -> Decision {
    if !i.service_enabled {
        return Decision::None;
    }

    if !i.screen_on {
        return Decision::Apply {
            profile: i.sleep_profile.clone(),
            reason: "screen off".into(),
        };
    }

    if i.keyguard_locked {
        return Decision::None;
    }

    // dual-app concurrency overrides mapping AND saver: two visible apps
    // need the middle ground (user verdict 2026-10-08)
    if i.multi_window {
        return Decision::Apply {
            profile: MULTI_WINDOW_PROFILE.into(),
            reason: "multi-window".into(),
        };
    }

    let pkg = i.foreground_pkg.as_deref();
    if is_transient(pkg) {
        return Decision::None;
    }

    // Dynamic Profile OFF: the universal base wins no matter what the app
    // map says — mapping is only a hint for the dynamic mode.
    let mapped = if i.dynamic_profile {
        pkg.and_then(|p| i.app_map.get(p)).cloned()
    } else {
        None
    };
    // MIUI battery saver: the unmapped universe is forced to the frugal
    // base — mapped apps still win (the user may game under saver).
    let effective_base = if i.saver_on {
        SAVER_PROFILE
    } else {
        i.base_profile.as_str()
    };
    match mapped {
        Some(profile) => Decision::Apply {
            profile,
            reason: "app".into(),
        },
        None => Decision::Apply {
            profile: effective_base.to_string(),
            reason: "base".into(),
        },
    }
}

#[cfg(test)]
mod tests;
