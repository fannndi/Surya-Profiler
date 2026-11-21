//! Marker-owned config/map installation and removal.

use super::*;

pub(crate) fn install_all() -> Result<&'static str, String> {
    let dir = config_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    if map_path().exists() && !marker_path().exists() {
        if !map_contains_owned() {
            return Err(format!(
                "{} exists but is not Surya-owned; refusing to overwrite",
                map_path().display()
            ));
        }
        // Adopt a marker-less map left by a crash between map write and
        // marker write in an older build, then rewrite it below.
        write_atomic(&marker_path(), MARKER_BODY.as_bytes())?;
    }
    let built = build_map()?;
    write_atomic(&marker_path(), MARKER_BODY.as_bytes())?;
    for profile in ThermalProfile::all() {
        let encrypted = crypt::encrypt(profile.plaintext().as_bytes());
        write_atomic(&config_path(profile), &encrypted)?;
    }
    // Map last: mi_thermald must never see an index whose config is missing.
    let encrypted_map = crypt::encrypt(built.text.as_bytes());
    write_atomic(&map_path(), &encrypted_map)?;
    Ok(built.source)
}

/// Removes only files carrying our marker (or a marker-less map that is
/// recognizably ours). Returns true only when both the map and marker are
/// gone; the map is removed first so `mi_thermald` never sees entries that
/// point at deleted configs.
pub(crate) fn remove_owned() -> bool {
    let marker = marker_path();
    let map = map_path();
    if !marker.exists() && !map_contains_owned() {
        return true;
    }
    if marker.exists() && map.exists() && !map_contains_owned() {
        return false;
    }
    if map.exists() {
        let _ = fs::remove_file(&map);
        if map.exists() {
            return false;
        }
    }
    for profile in ThermalProfile::all() {
        let _ = fs::remove_file(config_path(profile));
    }
    let _ = fs::remove_file(&marker);
    !map.exists() && !marker.exists()
}

// --- profile selection ------------------------------------------------------
