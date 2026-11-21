//! State dir storage: snapshot/state JSON (atomic writes) + profile loading.

use super::{now_secs, read_json, read_json_strict, write_json, SnapValue, Snapshot, State};
use crate::engine::catalog;
use crate::engine::plan::{OpStatus, PlannedOp};
use crate::engine::probe::{self, ProbeData};
use crate::engine::readback::normalize_snapshot;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Store {
    dir: PathBuf,
    /// Cached parsed profile pack keyed by file mtime (None for embedded).
    profiles_cache: std::cell::RefCell<
        Option<(
            Option<std::time::SystemTime>,
            crate::engine::profile::ProfilesFile,
        )>,
    >,
}

impl Store {
    pub fn new(dir: &Path) -> Self {
        Store {
            dir: dir.to_path_buf(),
            profiles_cache: std::cell::RefCell::new(None),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn snapshot_path(&self) -> PathBuf {
        self.dir.join("snapshot.json")
    }

    fn stock_path(&self) -> PathBuf {
        self.dir.join("stock.json")
    }

    /// Persistent union-stock map: one entry per catalog key the profiles
    /// ever touch; never consumed. Seeded once (first apply after install)
    /// and grown when a new profile version introduces a key. Reconcile
    /// ("auto-revive") writes these values for keys the active profile does
    /// not set; the CLI `restore` parks every key back to stock.
    pub fn load_stock(&self) -> Option<Snapshot> {
        read_json(&self.stock_path())
    }

    /// Strict restore baseline load: a corrupt `stock.json` is an error, not
    /// an excuse to re-capture the (possibly tuned) live values.
    pub fn load_stock_strict(&self) -> Result<Option<Snapshot>, String> {
        read_json_strict(&self.stock_path())
    }

    pub fn save_stock(&self, s: &Snapshot) -> Result<(), String> {
        write_json(&self.stock_path(), s)
    }

    fn state_path(&self) -> PathBuf {
        self.dir.join("state.json")
    }

    pub fn load_snapshot(&self) -> Option<Snapshot> {
        read_json(&self.snapshot_path())
    }

    pub fn load_snapshot_strict(&self) -> Result<Option<Snapshot>, String> {
        read_json_strict(&self.snapshot_path())
    }

    pub fn save_snapshot(&self, s: &Snapshot) -> Result<(), String> {
        write_json(&self.snapshot_path(), s)
    }

    pub fn load_state(&self) -> State {
        read_json(&self.state_path()).unwrap_or_default()
    }

    pub fn save_state(&self, s: &State) -> Result<(), String> {
        write_json(&self.state_path(), s)
    }

    /// Load user profiles from state dir, falling back to the embedded set.
    pub fn load_profiles(
        &self,
        override_path: Option<&Path>,
    ) -> Result<crate::engine::profile::ProfilesFile, String> {
        // overrides (CLI/tests) are never cached
        if let Some(p) = override_path {
            let s = fs::read_to_string(p).map_err(|e| format!("read {p:?}: {e}"))?;
            return crate::engine::profile::parse_profiles(&s);
        }
        let local = self.dir.join("profiles.json");
        let stamp = if local.exists() {
            fs::metadata(&local).ok().and_then(|m| m.modified().ok())
        } else {
            None
        };
        // cache hit?
        if let Some((cached_stamp, cached)) = self.profiles_cache.borrow().as_ref() {
            if *cached_stamp == stamp {
                return Ok(cached.clone());
            }
        }
        let parsed = if local.exists() {
            let s = fs::read_to_string(&local).map_err(|e| format!("read {local:?}: {e}"))?;
            crate::engine::profile::parse_profiles(&s)?
        } else {
            crate::engine::profile::parse_profiles(crate::DEFAULT_PROFILES_JSON)?
        };
        *self.profiles_cache.borrow_mut() = Some((stamp, parsed.clone()));
        Ok(parsed)
    }
}

/// Snapshot originals for every key we are about to touch (first write wins:
/// the earliest value is the stock one and is never overwritten).
/// Seed/extend the persistent `stock.json` with every profile-union key.
/// Fallback chain when a key is missing: existing `stock.json` value ->
/// legacy `snapshot.json` value -> live device value (post-boot MIUI stock).
/// The device file for union keys that never appeared is only useful on a
/// clean boot (sysfs resets at boot); the installer's stock.json is seeded
/// right after the first `apply` on a v0.11 device.
pub fn ensure_stock(
    store: &Store,
    probe: &ProbeData,
    override_path: Option<&std::path::Path>,
) -> Result<(), String> {
    let profiles = store.load_profiles(override_path)?;
    let mut keys: Vec<&str> = profiles
        .profiles
        .iter()
        .flat_map(|p| p.params.keys().map(|k| k.as_str()))
        .collect();
    keys.sort_unstable();
    keys.dedup();

    // Snapshot baseline: prefer the existing stock.json, else migrate the
    // legacy session snapshot (pre-v0.11 devices). Both are strict: a corrupt
    // restore point must fail the apply, not silently capture tuned values.
    let stock = store.load_stock_strict()?;
    let legacy = store.load_snapshot_strict()?;
    let created = stock
        .as_ref()
        .or(legacy.as_ref())
        .map(|s| s.created)
        .unwrap_or_else(now_secs);
    let device = stock
        .as_ref()
        .or(legacy.as_ref())
        .map(|s| s.device.clone())
        .unwrap_or_else(|| probe.device.device.clone());
    let mut snap = stock.unwrap_or_else(|| Snapshot {
        created,
        device,
        values: BTreeMap::new(),
    });
    let mut changed = false;
    for key in keys {
        // Refill a previously-empty capture (e.g. cfq tunables captured while
        // the deadline elevator was active — the node didn't exist then).
        if snap
            .values
            .get(key)
            .map(|v| !v.value.is_empty())
            .unwrap_or(false)
        {
            continue;
        }
        // Legacy per-key fallback before touching the live device.
        if let Some(v) = legacy
            .as_ref()
            .and_then(|s| s.values.get(key))
            .filter(|v| !v.value.is_empty())
        {
            snap.values.insert(key.to_owned(), v.clone());
            changed = true;
            continue;
        }
        let Some(entry) = catalog::find(key) else {
            continue;
        };
        let value = normalize_snapshot(entry.kind, &probe::read(entry.path).unwrap_or_default());
        if value.is_empty() {
            // Node unavailable right now (elevator/governor-dependent): skip,
            // a later apply re-captures. Never store an empty "stock".
            continue;
        }
        snap.values.insert(
            key.to_owned(),
            SnapValue {
                path: entry.path.to_owned(),
                value,
            },
        );
        changed = true;
    }
    if changed {
        store.save_stock(&snap)?;
    }
    Ok(())
}

pub(super) fn ensure_snapshot(
    store: &Store,
    probe: &ProbeData,
    ops: &[PlannedOp],
) -> Result<(Snapshot, bool), String> {
    let existing = store.load_snapshot_strict()?;
    let existing_was_none = existing.is_none();
    let mut snap = existing.unwrap_or_else(|| Snapshot {
        created: now_secs(),
        device: probe.device.device.clone(),
        values: BTreeMap::new(),
    });
    let mut changed = false;
    for op in ops {
        if matches!(op.status, OpStatus::Locked(_)) {
            continue;
        }
        if snap.values.contains_key(&op.key) {
            continue;
        }
        let value = op
            .current
            .clone()
            .or_else(|| probe.entries.get(&op.key).and_then(|e| e.value.clone()))
            .unwrap_or_default();
        let kind = catalog::find(&op.key)
            .map(|e| e.kind)
            .unwrap_or(catalog::Kind::Text);
        let value = normalize_snapshot(kind, &value);
        if value.is_empty() {
            // never store an empty stock/snapshot value (would truncate a node
            // on restore)
            continue;
        }
        snap.values.insert(
            op.key.clone(),
            SnapValue {
                path: op.path.clone(),
                value,
            },
        );
        changed = true;
    }
    if changed {
        store.save_snapshot(&snap)?;
    }
    let created_now = existing_was_none && changed;
    Ok((snap, created_now))
}

#[cfg(test)]
mod tests;
