//! `cmd game downscale` + `dumpsys platform_compat` plumbing.
//!
//! The AOSP GameManagerService shell command toggles `PlatformCompat`
//! change IDs (`DOWNSCALED` + one `DOWNSCALE_xx`). The app buffer is scaled
//! uniformly while its layout/density stay unchanged; this module captures
//! the previous override so Service OFF can restore it.

use super::ratios::{self, GATE_ID, SCALE_IDS};
use crate::platform;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub(crate) struct Host {
    pub cmd: PathBuf,
    pub dumpsys: PathBuf,
}

impl Host {
    pub(crate) fn detect() -> Host {
        Host {
            cmd: env_bin("SURYAPROFILER_CMD_BIN", "/system/bin/cmd"),
            dumpsys: env_bin("SURYAPROFILER_DUMPSYS_BIN", "/system/bin/dumpsys"),
        }
    }

    /// Feature probe: the MIUI `game` service must advertise `downscale`.
    /// `cmd game help` exits 255 on AOSP even on success, so only its text
    /// and timeout matter.
    pub(crate) fn supported(&self) -> bool {
        let Some(out) = run(&self.cmd, &["game", "help"], Duration::from_secs(2)) else {
            return false;
        };
        !out.timed_out && out.stdout_lossy().contains("downscale")
    }

    /// Current downscale state for `pkg`: `Some(None)` = off,
    /// `Some(Some("0.7"))` = active ratio, `None` = capture failed.
    pub(crate) fn current(&self, pkg: &str) -> Option<Option<String>> {
        let out = run(&self.dumpsys, &["platform_compat"], Duration::from_secs(3))?;
        if !out.success() {
            return None;
        }
        parse_state(&out.stdout_lossy(), pkg)
    }

    pub(crate) fn apply(&self, pkg: &str, ratio: &str) -> Result<(), String> {
        let Some(out) = run(
            &self.cmd,
            &["game", "downscale", ratio, pkg],
            Duration::from_secs(2),
        ) else {
            return Err("cmd game downscale spawn failed".into());
        };
        if !out.success() {
            return Err(format!(
                "cmd game downscale {ratio} {pkg}: {}",
                out.stderr_lossy().trim()
            ));
        }
        for _ in 0..3 {
            if self.current(pkg) == Some(Some(ratio.to_string())) {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(60));
        }
        Err(format!("downscale read-back for {pkg} is not {ratio}"))
    }

    pub(crate) fn disable(&self, pkg: &str) -> Result<(), String> {
        let Some(out) = run(
            &self.cmd,
            &["game", "downscale", "disable", pkg],
            Duration::from_secs(2),
        ) else {
            return Err("cmd game downscale disable spawn failed".into());
        };
        if !out.success() {
            return Err(format!(
                "cmd game downscale disable {pkg}: {}",
                out.stderr_lossy().trim()
            ));
        }
        for _ in 0..3 {
            if self.current(pkg) == Some(None) {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(60));
        }
        Err(format!("downscale disable read-back failed for {pkg}"))
    }
}

fn env_bin(var: &str, default: &str) -> PathBuf {
    std::env::var_os(var)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default))
}

fn run(bin: &Path, args: &[&str], timeout: Duration) -> Option<platform::Output> {
    platform::run(bin.to_str()?, args, timeout)
}

/// Parses one package's state from a `dumpsys platform_compat` dump.
///
/// Returns `None` when the `DOWNSCALED` gatekeeper line is absent (unknown
/// format), `Some(None)` for off, `Some(Some(ratio))` for an active scale.
pub(crate) fn parse_state(dump: &str, pkg: &str) -> Option<Option<String>> {
    let gate = find_line(dump, GATE_ID)?;
    if !value_for(gate, pkg) {
        return Some(None);
    }
    let mut best: Option<(&str, f64)> = None;
    for (id, ratio) in SCALE_IDS {
        let Some(line) = find_line(dump, *id) else {
            continue;
        };
        if !value_for(line, pkg) {
            continue;
        }
        let Ok(v) = ratio.parse::<f64>() else {
            continue;
        };
        if best.is_none_or(|(_, old)| v > old) {
            best = Some((ratio, v));
        }
    }
    Some(best.map(|(r, _)| r.to_string()))
}

fn find_line(dump: &str, id: i64) -> Option<&str> {
    let needle = format!("ChangeId({id};");
    dump.lines().find(|l| l.trim_start().starts_with(&needle))
}

/// Extracts `pkg=true/false` from `rawOverrides` (preferred) or
/// `packageOverrides`; absent means the change default (false for these IDs).
fn value_for(line: &str, pkg: &str) -> bool {
    for key in ["rawOverrides={", "packageOverrides={"] {
        let Some(start) = line.find(key) else {
            continue;
        };
        let body_start = start + key.len();
        let Some(end_rel) = line[body_start..].find('}') else {
            continue;
        };
        let body = &line[body_start..body_start + end_rel];
        for pair in body.split(',') {
            let Some((name, value)) = pair.split_once('=') else {
                continue;
            };
            if name.trim() == pkg {
                return value.trim() == "true";
            }
        }
    }
    false
}

/// Canonical ratio for logs; `None` for off/unknown.
pub(crate) fn prior_label(prior: &Option<String>) -> String {
    prior
        .as_deref()
        .map(ratios::label)
        .unwrap_or_else(|| "native".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DUMP: &str = "\
ChangeId(168419799; name=DOWNSCALED; disabled; packageOverrides={com.a=true, com.b=false}; rawOverrides={com.a=true, com.b=false}; overridable)
ChangeId(176926829; name=DOWNSCALE_70; disabled; packageOverrides={com.a=true}; rawOverrides={com.a=true}; overridable)
ChangeId(176926753; name=DOWNSCALE_80; disabled; packageOverrides={com.a=false}; rawOverrides={com.a=false}; overridable)
";

    #[test]
    fn reads_off_and_active_states() {
        assert_eq!(parse_state(DUMP, "com.a"), Some(Some("0.7".into())));
        assert_eq!(parse_state(DUMP, "com.b"), Some(None));
        assert_eq!(parse_state(DUMP, "com.c"), Some(None));
        assert_eq!(parse_state("garbage", "com.a"), None);
    }

    #[test]
    fn prefers_the_highest_active_scale() {
        let dump = "\
ChangeId(168419799; name=DOWNSCALED; rawOverrides={com.x=true})
ChangeId(176926829; name=DOWNSCALE_70; rawOverrides={com.x=true})
ChangeId(176926753; name=DOWNSCALE_80; rawOverrides={com.x=true})
";
        assert_eq!(parse_state(dump, "com.x"), Some(Some("0.8".into())));
    }

    #[test]
    fn raw_override_wins_over_package_override() {
        let dump = "\
ChangeId(168419799; name=DOWNSCALED; packageOverrides={com.x=true}; rawOverrides={com.x=false})
";
        assert_eq!(parse_state(dump, "com.x"), Some(None));
    }
}
