//! Vendor/custom `thermal-map.conf` parsing and generation.

use super::*;

pub(crate) const FALLBACK_STOCK_MAP: &str = "\
[0:thermal-normal.conf]
[1:thermal-high.conf]
[2:thermal-extreme.conf]
[8:thermal-phone.conf]
[9:thermal-tgame.conf]
[10:thermal-nolimits.conf]
[11:thermal-class0.conf]
[12:thermal-camera.conf]
[13:thermal-tgame.conf]
[14:thermal-youtube.conf]
[15:thermal-arvr.conf]
[16:thermal-tgame.conf]
[17:thermal-4k.conf]
[18:thermal-camera.conf]
";

pub(crate) fn parse_map(text: &str) -> Result<Vec<(i32, String)>, String> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let body = line
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .ok_or_else(|| format!("thermal map line {}: expected [id:file]", n + 1))?;
        let (id, name) = body
            .split_once(':')
            .ok_or_else(|| format!("thermal map line {}: missing ':'", n + 1))?;
        let id = id
            .trim()
            .parse::<i32>()
            .map_err(|e| format!("thermal map line {}: bad id: {e}", n + 1))?;
        let name = name.trim();
        if name.is_empty() {
            return Err(format!("thermal map line {}: empty file name", n + 1));
        }
        out.push((id, name.to_string()));
    }
    Ok(out)
}

pub(crate) fn render_map(entries: &[(i32, String)]) -> String {
    let mut out = String::new();
    for (id, name) in entries {
        out.push_str(&format!("[{id}:{name}]\n"));
    }
    out
}

pub(crate) fn vendor_map_entries_at(root: &Path) -> Result<Vec<(i32, String)>, String> {
    let path = root.join("vendor/etc/thermal-map.conf");
    let bytes = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let plain = crypt::decrypt(&bytes)?;
    let text = String::from_utf8(plain).map_err(|e| format!("vendor thermal map: {e}"))?;
    parse_map(&text)
}

#[derive(Debug)]
pub(crate) struct BuiltMap {
    pub(crate) text: String,
    pub(crate) source: &'static str,
}

pub(crate) fn build_map_at(root: &Path) -> Result<BuiltMap, String> {
    let (mut entries, source) = match vendor_map_entries_at(root) {
        Ok(entries) => (entries, "vendor"),
        Err(_) => (parse_map(FALLBACK_STOCK_MAP)?, "fallback"),
    };

    // Fail safe if a future ROM already uses our private indices.
    if entries.iter().any(|(id, _)| CUSTOM_INDICES.contains(id)) {
        return Err(format!(
            "vendor thermal map already uses private indices {CUSTOM_INDICES:?}"
        ));
    }

    // The vendor map has no 4K/8K entries on this build. MIUI's PowerKeeper
    // still selects 17/18 for camera recording; map them to the stock
    // configs so yielding actually loads a camera plan.
    if !entries.iter().any(|(id, _)| *id == 17) {
        entries.push((17, "thermal-4k.conf".into()));
    }
    if !entries.iter().any(|(id, _)| *id == 18) {
        entries.push((18, "thermal-camera.conf".into()));
    }

    for profile in ThermalProfile::all() {
        entries.push((profile.sconfig(), profile.config_name().into()));
    }

    Ok(BuiltMap {
        text: render_map(&entries),
        source,
    })
}

pub(crate) fn build_map() -> Result<BuiltMap, String> {
    build_map_at(&default_root())
}

pub(crate) fn map_contains_owned_at(path: &Path) -> bool {
    let Ok(bytes) = fs::read(path) else {
        return false;
    };
    let Ok(plain) = crypt::decrypt(&bytes) else {
        return false;
    };
    let Ok(text) = String::from_utf8(plain) else {
        return false;
    };
    let Ok(entries) = parse_map(&text) else {
        return false;
    };
    // v0.4.x used 20–23, v0.5.0 preview used 70–73. A map is ours only when
    // an owned index points at one of our four known config names; a foreign
    // map that happens to use the same private IDs must not be deleted.
    entries.iter().any(|(id, name)| {
        (is_custom_index(*id) || is_legacy_index(*id))
            && matches!(
                name.as_str(),
                "thermal-balance.conf"
                    | "thermal-game.conf"
                    | "thermal-powersave.conf"
                    | "thermal-sleep.conf"
            )
    })
}

pub(crate) fn map_contains_owned() -> bool {
    map_contains_owned_at(&map_path())
}

// --- install / remove -------------------------------------------------------
