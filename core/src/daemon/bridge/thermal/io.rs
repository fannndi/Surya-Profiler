//! Guarded thermal writes + read-back verification.

use super::*;

pub(crate) fn read_i32(path: &Path) -> Option<i32> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

/// The only sysfs/proc node the thermal takeover may write.
pub(crate) fn guard_sconfig(path: &Path) -> Result<(), String> {
    if path == sconfig_path() {
        Ok(())
    } else {
        Err(format!(
            "thermal write rejected: {} is not the sconfig node",
            path.display()
        ))
    }
}

pub(crate) fn config_file_allowed(name: &str) -> bool {
    name == MARKER_NAME
        || name == "thermal-map.conf"
        || ThermalProfile::all()
            .iter()
            .any(|p| p.config_name() == name)
}

/// The only `/data/vendor/thermal/config` files the takeover may write.
pub(crate) fn guard_config_file(path: &Path) -> Result<(), String> {
    let allowed = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(config_file_allowed)
        .unwrap_or(false);
    if path.parent() == Some(config_dir().as_path()) && allowed {
        Ok(())
    } else {
        Err(format!(
            "thermal write rejected: {} is not a Surya config file",
            path.display()
        ))
    }
}

pub(crate) fn write_text(path: &Path, body: &str) -> Result<(), String> {
    guard_sconfig(path)?;
    fs::write(path, body).map_err(|e| format!("{}: {e}", path.display()))
}

pub(crate) fn write_atomic(path: &Path, body: &[u8]) -> Result<(), String> {
    guard_config_file(path)?;
    // Encrypted configs are deterministic; skip identical rewrites.
    if fs::read(path).ok().as_deref() == Some(body) {
        return Ok(());
    }
    let tmp = path.with_extension("surya-tmp");
    {
        let mut f = fs::File::create(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
        f.write_all(body)
            .map_err(|e| format!("{}: {e}", tmp.display()))?;
        f.flush().map_err(|e| format!("{}: {e}", tmp.display()))?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&tmp, fs::Permissions::from_mode(0o644));
    }
    fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Normalized comparison: mi_thermald's dump may differ only in trailing
/// whitespace/CRLF.
pub(crate) fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n").trim_end().to_string()
}

pub(crate) fn verify_loaded(profile: ThermalProfile) -> bool {
    match fs::read_to_string(decrypt_path()) {
        Ok(raw) => normalize(&raw) == normalize(profile.plaintext()),
        // Host/simulation has no mi_thermald dump. On the real device a
        // missing dump means the daemon is not running, so reporting the
        // custom profile as "active" would be a false positive.
        Err(_) => crate::engine::probe::allow_env_overrides(),
    }
}
