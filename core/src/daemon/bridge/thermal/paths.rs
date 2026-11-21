//! Filesystem locations used by the thermal takeover.

use super::*;

pub(crate) fn config_dir() -> PathBuf {
    default_root().join("data/vendor/thermal/config")
}

pub(crate) fn sconfig_path() -> PathBuf {
    default_root().join("sys/class/thermal/thermal_message/sconfig")
}

pub(crate) fn decrypt_path() -> PathBuf {
    default_root().join("data/vendor/thermal/decrypt.txt")
}

pub(crate) fn marker_path() -> PathBuf {
    config_dir().join(MARKER_NAME)
}

pub(crate) fn marker_exists() -> bool {
    marker_path().exists()
}

pub(crate) fn map_path() -> PathBuf {
    config_dir().join("thermal-map.conf")
}

pub(crate) fn config_path(profile: ThermalProfile) -> PathBuf {
    config_dir().join(profile.config_name())
}

pub(crate) fn global_mode_path() -> PathBuf {
    default_root().join("data/vendor/thermal/thermal-global-mode")
}

pub(crate) fn perf_config_path() -> PathBuf {
    default_root().join("data/property/persist.sys.thermal.config")
}
