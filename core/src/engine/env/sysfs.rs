//! Root/zone discovery and read helpers.

use super::*;

pub fn default_root() -> PathBuf {
    if crate::engine::probe::allow_env_overrides() {
        if let Some(v) = std::env::var_os(SYSFS_ROOT_ENV) {
            return PathBuf::from(v);
        }
    }
    PathBuf::from("/")
}

/// Cooling-device state files resolved once at startup.
pub(crate) struct CoolingPaths {
    pub(crate) cur: PathBuf,
    pub(crate) max: PathBuf,
}

pub(crate) fn discover_zones(root: &Path) -> Vec<(String, PathBuf)> {
    let base = root.join("sys/class/thermal");
    let mut zones = Vec::new();
    let Ok(rd) = fs::read_dir(&base) else {
        return zones;
    };
    for e in rd.flatten() {
        let p = e.path();
        let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.starts_with("thermal_zone") {
            continue;
        }
        if let Some(t) = read_trim(&p.join("type")) {
            zones.push((t, p.join("temp")));
        }
    }
    zones
}

pub(crate) fn read_trim(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

// --- pure parsers (unit tested without a filesystem) -------------------------
