//! Daemon view of `config.json`.
//!
//! Ownership: the APP is the single writer (it also serves the UI when the
//! service is off); the daemon reads — at startup, on an explicit
//! `config_changed` command, and on mtime changes as a fallback.
//!
//! The file is written atomically by the app (tmp + rename) so a torn read
//! is impossible; still, a parse failure keeps the previous config and is
//! reported, never fatal.
//!
//! Schema 2 is intentionally tiny: service flags, the universal base, the
//! per-app mapping, the automatic thermal takeover and the opt-in R&D
//! input-suspend switch. Everything the app does not need is ignored (old
//! schema-1 keys disappear on the first app write).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub const SCHEMA: u32 = 2;

fn default_true() -> bool {
    true
}

fn default_base() -> String {
    "balance".into()
}

/// Profile ids the engine ships. `sleep` is automatic; the rest are the
/// three user-facing Device Profiles.
pub const USER_PROFILES: &[&str] = &["powersave", "balance", "game"];

/// User intent mirrored from the app. Unknown fields are ignored (the app
/// may add UI-only keys); missing fields fall back to defaults, so a partial
/// file still yields a sane daemon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DaemonConfig {
    #[serde(default = "default_schema")]
    pub schema: u32,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub dynamic: bool,
    /// Take over MIUI thermal in lockstep with the active Device Profile.
    /// Default on; the vendor map remains the fallback.
    #[serde(default = "default_true")]
    pub custom_thermal: bool,
    #[serde(default = "default_base")]
    pub base_profile: String,
    /// Apps Profile: package -> Device Profile id. Only the three user
    /// profiles are valid (validated by `effective_app_map`).
    #[serde(default)]
    pub app_map: BTreeMap<String, String>,
    /// R&D only: suspend charger input (`input_suspend=1`) while the
    /// service is on. Captured/restored through the bridge holds.
    #[serde(default)]
    pub rnd_input_suspend: bool,
    /// Freq floor audit: re-assert the active profile's CPU floor when an
    /// external writer keeps `scaling_min_freq` high with no activity
    /// (default on; yields to MIUI performance mode, backs off when a
    /// re-assert does not stick).
    #[serde(default = "default_true")]
    pub freq_floor_audit: bool,
    /// Daemon log level (`error|warn|info|debug`); None = env/default.
    #[serde(default)]
    pub log_level: Option<String>,
}

fn default_schema() -> u32 {
    SCHEMA
}

impl Default for DaemonConfig {
    fn default() -> Self {
        DaemonConfig {
            schema: SCHEMA,
            enabled: true,
            dynamic: true,
            custom_thermal: true,
            base_profile: default_base(),
            app_map: BTreeMap::new(),
            rnd_input_suspend: false,
            freq_floor_audit: true,
            log_level: None,
        }
    }
}

impl DaemonConfig {
    /// Fail-closed shape for an unreadable/unsupported config. The daemon may
    /// still run (so the app can repair it), but it must not tune or take over
    /// thermal based on defaults it invented.
    fn fail_closed() -> Self {
        DaemonConfig {
            enabled: false,
            dynamic: false,
            custom_thermal: false,
            freq_floor_audit: false,
            ..DaemonConfig::default()
        }
    }
}

/// Cached config file with change detection.
pub struct ConfigFile {
    path: PathBuf,
    cached: DaemonConfig,
    stamp: Option<SystemTime>,
    len: u64,
}

impl ConfigFile {
    /// Loads the file; on any failure returns defaults + the error reason.
    pub fn load(path: &Path) -> (Self, Option<String>) {
        match fs::read_to_string(path) {
            Ok(raw) => match serde_json::from_str::<DaemonConfig>(&raw) {
                Ok(cfg) if cfg.schema <= SCHEMA => (
                    ConfigFile {
                        path: path.to_path_buf(),
                        cached: cfg,
                        stamp: mtime(path),
                        len: raw.len() as u64,
                    },
                    None,
                ),
                Ok(cfg) => (
                    ConfigFile {
                        path: path.to_path_buf(),
                        cached: DaemonConfig::fail_closed(),
                        stamp: mtime(path),
                        len: 0,
                    },
                    Some(format!(
                        "config schema {} is newer than {SCHEMA}",
                        cfg.schema
                    )),
                ),
                Err(e) => (
                    ConfigFile {
                        path: path.to_path_buf(),
                        cached: DaemonConfig::fail_closed(),
                        stamp: mtime(path),
                        len: 0,
                    },
                    Some(format!("config parse: {e}")),
                ),
            },
            Err(e) => (
                ConfigFile {
                    path: path.to_path_buf(),
                    cached: DaemonConfig::fail_closed(),
                    stamp: None,
                    len: 0,
                },
                Some(format!("config read: {e}")),
            ),
        }
    }

    pub fn get(&self) -> &DaemonConfig {
        &self.cached
    }

    /// True (and reload) when the file's mtime/size changed since last read.
    pub fn reload_if_changed(&mut self) -> bool {
        let stamp = mtime(&self.path);
        if stamp == self.stamp && stamp.is_none() {
            return false; // file still absent
        }
        let len = fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0);
        if stamp == self.stamp && len == self.len {
            return false;
        }
        self.reload()
    }

    /// Unconditional reload — used by the explicit `config_changed` hint so
    /// a write can never be missed by mtime granularity. A parse failure
    /// keeps the previous good config (reported, never fatal).
    pub fn reload(&mut self) -> bool {
        let Ok(raw) = fs::read_to_string(&self.path) else {
            return false; // transient (mid-rename) — retry next tick
        };
        match serde_json::from_str::<DaemonConfig>(&raw) {
            Ok(cfg) if cfg.schema <= SCHEMA => {
                self.cached = cfg;
                self.stamp = mtime(&self.path);
                self.len = raw.len() as u64;
                true
            }
            // Keep the last known-good config on a parse error or a newer
            // schema; never downgrade a future file into today's defaults.
            Ok(_) | Err(_) => false,
        }
    }
}

fn mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

impl DaemonConfig {
    /// Validated app map: unknown profile ids are dropped so a typo (or a
    /// stale `boost` mapping) can never reach the engine.
    pub fn effective_app_map(&self) -> BTreeMap<String, String> {
        self.app_map
            .iter()
            .filter(|(_, p)| USER_PROFILES.contains(&p.as_str()))
            .map(|(pkg, p)| (pkg.clone(), p.clone()))
            .collect()
    }

    /// Base profile id validated against the shipped set; unknown values fall
    /// back to `balance` so a typo in config.json can never strand the daemon.
    pub fn effective_base_profile(&self) -> String {
        if USER_PROFILES.contains(&self.base_profile.as_str()) {
            self.base_profile.clone()
        } else {
            "balance".to_string()
        }
    }
}

#[cfg(test)]
mod tests;
