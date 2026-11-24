//! `wm size` / `wm density` capture, apply and exact restore.

use crate::platform;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Original runtime overrides. `None` means "no override" (reset on restore);
/// `Some` restores the user's exact value (e.g. density 400).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct WmSnapshot {
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default)]
    pub density: Option<String>,
}

pub(crate) struct Host {
    pub wm: PathBuf,
}

impl Host {
    pub(crate) fn detect() -> Host {
        Host {
            wm: std::env::var_os("SURYAPROFILER_WM_BIN")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/system/bin/wm")),
        }
    }

    pub(crate) fn read(&self) -> Option<WmSnapshot> {
        let size = self.run(&["size"])?;
        let density = self.run(&["density"])?;
        Some(WmSnapshot {
            size: override_value(&size, "size"),
            density: override_value(&density, "density"),
        })
    }

    pub(crate) fn apply(&self, size: &str, density: u32) -> Result<(), String> {
        validate_size(size)?;
        if !(160..=640).contains(&density) {
            return Err(format!("density {density} outside 160..640"));
        }
        self.write(&["size", size])?;
        self.write(&["density", &density.to_string()])?;
        let live = self.read().ok_or("wm read-back failed")?;
        let want_density = density.to_string();
        if live.size.as_deref() != Some(size)
            || live.density.as_deref() != Some(want_density.as_str())
        {
            return Err(format!(
                "wm read-back is {live:?}, wanted {size} @ {density}"
            ));
        }
        Ok(())
    }

    pub(crate) fn restore(&self, snap: &WmSnapshot) -> Result<(), String> {
        match snap.size.as_deref() {
            Some(size) => self.write(&["size", size])?,
            None => self.write(&["size", "reset"])?,
        }
        match snap.density.as_deref() {
            Some(d) => self.write(&["density", d])?,
            None => self.write(&["density", "reset"])?,
        }
        let live = self.read().ok_or("wm read-back failed")?;
        if live != *snap {
            return Err(format!("wm restore read-back is {live:?}, wanted {snap:?}"));
        }
        Ok(())
    }

    fn run(&self, args: &[&str]) -> Option<String> {
        let bin = self.wm.to_str()?;
        let out = platform::run(bin, args, Duration::from_secs(3))?;
        out.success().then(|| out.stdout_lossy())
    }

    fn write(&self, args: &[&str]) -> Result<(), String> {
        let bin = self.wm.to_str().ok_or("invalid wm path")?;
        let out = platform::run(bin, args, Duration::from_secs(5))
            .ok_or_else(|| format!("wm {} spawn failed", args.join(" ")))?;
        if out.success() {
            Ok(())
        } else {
            Err(format!(
                "wm {} failed: {}",
                args.join(" "),
                out.stderr_lossy().trim()
            ))
        }
    }
}

fn override_value(text: &str, kind: &str) -> Option<String> {
    let prefix = format!("Override {kind}: ");
    text.lines()
        .find_map(|l| l.trim().strip_prefix(&prefix).map(|v| v.trim().to_string()))
}

fn validate_size(size: &str) -> Result<(), String> {
    let (w, h) = size
        .split_once('x')
        .ok_or_else(|| format!("bad size {size}"))?;
    let w: u32 = w.parse().map_err(|_| format!("bad width in {size}"))?;
    let h: u32 = h.parse().map_err(|_| format!("bad height in {size}"))?;
    if w < 320 || h < 320 || w * 20 != h * 9 {
        return Err(format!("{size} is not an exact 20:9 size"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_override_values() {
        let size = "Physical size: 1080x2400\nOverride size: 720x1600\n";
        let density = "Physical density: 440\nOverride density: 400\n";
        assert_eq!(override_value(size, "size").as_deref(), Some("720x1600"));
        assert_eq!(override_value(density, "density").as_deref(), Some("400"));
        assert_eq!(override_value("Physical size: 1080x2400\n", "size"), None);
    }

    #[test]
    fn validates_exact_20_9_sizes() {
        for s in ["1080x2400", "864x1920", "810x1800", "720x1600", "540x1200"] {
            assert!(validate_size(s).is_ok(), "{s}");
        }
        assert!(validate_size("720x1280").is_err());
        assert!(validate_size("1920x1080").is_err());
        assert!(validate_size("abc").is_err());
    }
}
