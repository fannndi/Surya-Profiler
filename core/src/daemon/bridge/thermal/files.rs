//! Foreign-file detection/repair inside `/data/vendor/thermal/config`.
//!
//! `mi_thermald` prefers the `/data/vendor/thermal/config` dir over
//! `/vendor/etc`, so a leftover `thermal-*.conf` installed by another tuning
//! tool keeps shadowing the stock config even after our Service-OFF restore
//! — `remove_owned()` only deletes marker-owned files. Scene's
//! `thermal_conf3/config.sh` guards exactly this (foreign-file scan +
//! Magisk-module ships + `chattr -i` repair + `restorecon`).
//!
//! `scan` is read-only and safe at any time; `fix` removes foreign files and
//! repairs dir metadata and is only run from the CLI with the daemon stopped.

use super::*;

/// The one thermal base dir, honouring MIUI's path override props.
///
/// REA-confirmed (2026-11-23): `mi_thermald` hardcodes its own base to
/// `/data/vendor/thermal/` (main() sets `vendor.sys.thermal.data.path` to it,
/// it never reads a prop), and PowerKeeper resolves the same base as
/// `getprop vendor.sys.thermal.data.path` -> `getprop sys.thermal.data.path`
/// -> hardcoded `/data/vendor/thermal/`. We accept either prop only when it
/// still points under `/data/vendor/thermal`; anything else (or empty) keeps
/// the default instead of following a suspicious value.
fn thermal_base(root: &Path) -> PathBuf {
    let default = root.join("data/vendor/thermal");
    if root != Path::new("/") {
        return default;
    }
    ["vendor.sys.thermal.data.path", "sys.thermal.data.path"]
        .iter()
        .find_map(|p| getprop_str(p))
        .filter(|p| p.starts_with("/data/vendor/thermal"))
        .map(|p| PathBuf::from(p.trim_end_matches('/')))
        .unwrap_or(default)
}

fn getprop_str(name: &str) -> Option<String> {
    let out = crate::platform::run(
        "/system/bin/getprop",
        &[name],
        std::time::Duration::from_secs(2),
    )?;
    if !out.success() {
        return None;
    }
    let s = out.stdout_lossy().trim().to_string();
    (!s.is_empty()).then_some(s)
}

/// Classified view of one entry in the config dir.
fn classify(path: &Path) -> Value {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("?")
        .to_string();
    let meta = fs::metadata(path).ok();
    let size = meta.as_ref().map(|m| m.len());
    let modified = meta
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());
    let (kind, hint) = match fs::read(path) {
        Ok(bytes) => {
            if crypt::is_encrypted(&bytes) {
                // Encrypted blob whose plaintext we can also read: a map
                // (parseable) or an opaque conf.
                match crypt::decrypt(&bytes)
                    .ok()
                    .and_then(|p| String::from_utf8(p).ok())
                    .and_then(|t| super::parse_map(&t).ok())
                {
                    Some(entries) => {
                        let ids: Vec<i32> = entries.iter().map(|(id, _)| *id).collect();
                        ("encrypted-map".to_string(), json!(ids))
                    }
                    None => ("encrypted-conf".to_string(), Value::Null),
                }
            } else if crypt::decrypt_or_plain(&bytes).is_ok() {
                // Plain text that decrypt_or_plain accepted (plaintext map /
                // conf). Our own configs are always encrypted, so this is a
                // foreign write even when it parses.
                match String::from_utf8(bytes.to_vec())
                    .ok()
                    .and_then(|t| super::parse_map(&t).ok())
                {
                    Some(entries) => {
                        let ids: Vec<i32> = entries.iter().map(|(id, _)| *id).collect();
                        ("plaintext-map".to_string(), json!(ids))
                    }
                    None => ("plaintext-conf".to_string(), Value::Null),
                }
            } else {
                ("unknown-binary".to_string(), Value::Null)
            }
        }
        Err(_) => ("unreadable".to_string(), Value::Null),
    };
    json!({ "name": name, "kind": kind, "size": size, "modified": modified, "detail": hint })
}

fn entries_in(dir: &Path) -> Vec<Value> {
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            // Only files; an unexpected subdirectory is reported as one entry
            // so the scan still notices it.
            out.push(classify(&e.path()));
        }
        out.sort_by(|a, b| {
            a["name"]
                .as_str()
                .unwrap_or("")
                .cmp(b["name"].as_str().unwrap_or(""))
        });
    }
    out
}

/// Read-only scan of foreign thermal ships. Never mutates anything.
pub(crate) fn scan() -> Value {
    scan_at(&default_root())
}

pub(crate) fn scan_at(root: &Path) -> Value {
    let base = thermal_base(root);
    let config = base.join("config");
    let config_present = config.exists();
    let config_is_dir = config.is_dir();
    let config_ok = !config_present || config_is_dir;
    let mut entries = entries_in(&config);
    // Owned files are reported but never flagged foreign.
    let owned = |name: &str| config_file_allowed(name);
    for e in entries.iter_mut() {
        let name = e["name"].as_str().unwrap_or("");
        e["owned"] = json!(owned(name));
    }
    let foreign: Vec<&Value> = entries
        .iter()
        .filter(|e| !e["owned"].as_bool().unwrap_or(false))
        .collect();
    let modules = scan_modules(root);
    json!({
        "base": base.display().to_string(),
        "data_path_prop": {
            // PowerKeeper's resolution chain (new prop first). Both props
            // point at the same base on surya; the scan shows what it saw.
            "new": getprop_str("vendor.sys.thermal.data.path"),
            "old": getprop_str("sys.thermal.data.path"),
        },
        "global_mode": read_i32(&base.join("thermal-global-mode")),
        "config_present": config_present,
        "config_is_dir": config_is_dir,
        "config_repair_hint": if config_ok {
            Value::Null
        } else {
            json!("config path is a file; `doctor --fix-thermal` rebuilds it")
        },
        "entries": entries,
        "foreign": foreign,
        "modules": modules,
        "clean": config_ok && foreign.is_empty() && modules.is_empty(),
    })
}

/// Magisk/APatch module overlays that ship thermal files into /vendor or
/// /system — they re-appear on every boot, so removal alone cannot fix them.
fn scan_modules(root: &Path) -> Vec<Value> {
    let modules_dir = root.join("data/adb/modules");
    let Ok(rd) = fs::read_dir(&modules_dir) else {
        return Vec::new();
    };
    let mut hits = Vec::new();
    for m in rd.flatten() {
        let mdir = m.path();
        let module_name = mdir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();
        let mut files = Vec::new();
        let mut stack = vec![(mdir.clone(), 0usize)];
        while let Some((dir, depth)) = stack.pop() {
            if depth > 5 || files.len() >= 10 {
                continue;
            }
            if let Ok(rd) = fs::read_dir(&dir) {
                for e in rd.flatten() {
                    let p = e.path();
                    if p.is_dir() {
                        stack.push((p, depth + 1));
                    } else if p
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.to_ascii_lowercase().contains("thermal"))
                        .unwrap_or(false)
                    {
                        files.push(p.strip_prefix(&mdir).unwrap_or(&p).display().to_string());
                    }
                }
            }
        }
        if !files.is_empty() {
            hits.push(json!({ "module": module_name, "files": files }));
        }
    }
    hits
}

/// Removes foreign files and repairs dir metadata.
///
/// Order (Scene's reset_permissions / clear_thermal_dir pattern):
/// `chattr -R -i` (release immutable flags a broken install may have set),
/// delete every non-owned file in the config dir, then `restorecon -DFR`
/// (SELinux labels must match vendor expectations after manual writes).
/// Owned files (marker, map, our four configs) are never touched.
pub(crate) fn fix() -> Value {
    fix_at(&default_root())
}

pub(crate) fn fix_at(root: &Path) -> Value {
    let config = thermal_base(root).join("config");
    let on_device = root == Path::new("/");
    let foreign_before = scan_at(root)["foreign"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    // A corrupted `/data/vendor/thermal` tree can leave `config` as a file;
    // create_dir_all then fails forever. Rebuild it like Scene's
    // `clear_thermal_dir` (chattr -i first, then remove + recreate).
    let mut config_rebuilt = false;
    if config.exists() && !config.is_dir() {
        if on_device {
            let _ = crate::platform::run(
                "/system/bin/chattr",
                &["-R", "-i", &config.display().to_string()],
                std::time::Duration::from_secs(10),
            );
        }
        let _ = fs::remove_file(&config);
        config_rebuilt = fs::create_dir_all(&config).is_ok();
    }

    let chattr_ok = on_device
        .then(|| {
            crate::platform::run(
                "/system/bin/chattr",
                &["-R", "-i", &config.display().to_string()],
                std::time::Duration::from_secs(10),
            )
        })
        .flatten()
        .map(|o| o.success());

    let mut removed = Vec::new();
    let mut failed = Vec::new();
    if let Ok(rd) = fs::read_dir(&config) {
        for e in rd.flatten() {
            let path = e.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if config_file_allowed(name) {
                continue; // never remove marker-owned files
            }
            match fs::remove_file(&path) {
                Ok(()) => removed.push(name.to_string()),
                Err(err) => failed.push(format!("{name}: {err}")),
            }
        }
    }

    let restorecon_ok = on_device
        .then(|| {
            crate::platform::run(
                "/system/bin/restorecon",
                &["-DFR", &config.display().to_string()],
                std::time::Duration::from_secs(20),
            )
        })
        .flatten()
        .map(|o| o.success());

    let after = scan_at(root);
    json!({
        "foreign_before": foreign_before.len(),
        "removed": removed,
        "failed": failed,
        "config_rebuilt": config_rebuilt,
        "chattr_ok": chattr_ok,
        "restorecon_ok": restorecon_ok,
        "clean_after": after["clean"],
        "foreign_after": after["foreign"],
    })
}
