//! Bridge recovery + holds persistence.

use super::*;
use std::io::Write;

impl Bridge {
    /// Loads persisted holds from a previous run (crash recovery).
    pub(crate) fn recover(
        state_dir: &Path,
        publisher: Arc<Publisher>,
        flags: Option<Arc<LiveFlags>>,
    ) -> Arc<Bridge> {
        let holds_path = state_dir.join("holds.json");
        let affinity_path = state_dir.join("affinity.json");
        let downscale_path = state_dir.join("downscale.json");
        let display_path = state_dir.join("display.json");
        let holds_missing = !holds_path.exists();
        let loaded_holds = read_holds_file(&holds_path);
        let holds_load_error = loaded_holds.as_ref().err().cloned();
        let holds = loaded_holds.unwrap_or_default();
        let affinity = fs::read_to_string(&affinity_path)
            .ok()
            .and_then(|s| serde_json::from_str::<affinity::AffinityState>(&s).ok())
            .unwrap_or_default();
        let downscale = fs::read_to_string(&downscale_path)
            .ok()
            .and_then(|s| serde_json::from_str::<downscale::DownscaleState>(&s).ok())
            .unwrap_or_default();
        let display = fs::read_to_string(&display_path)
            .ok()
            .and_then(|s| serde_json::from_str::<display::DisplayState>(&s).ok())
            .unwrap_or_default();
        let live_thermal_takeover = thermal::marker_exists()
            || thermal::map_contains_owned()
            || thermal::read_i32(&thermal::sconfig_path()).is_some_and(thermal::is_custom_index);
        let holds_recovery_blocked = holds_recovery_blocked(
            holds_missing,
            holds_load_error.as_deref(),
            live_thermal_takeover,
        );
        if let Some(err) = &holds_load_error {
            if !holds_missing {
                publisher.log(&format!("bridge: could not recover holds: {err}"));
            }
        }
        if holds_recovery_blocked {
            publisher.log(
                "bridge: holds state is missing/corrupt while a thermal takeover is live; refusing new thermal writes until release",
            );
        }
        if holds.saver_held || holds.input_suspend_held || holds.thermal_held {
            publisher.log("bridge: recovered holds from previous run");
        }
        if affinity.active() {
            publisher.log(&format!(
                "bridge: recovered {} affinity thread(s) for {}",
                affinity.threads.len(),
                affinity.target.as_deref().unwrap_or("?")
            ));
        }
        if downscale.active() {
            publisher.log(&format!(
                "bridge: recovered {} downscale app(s)",
                downscale.apps.len()
            ));
        }
        if display.active() {
            publisher.log(&format!(
                "bridge: recovered display preset {:?}",
                display.applied.as_deref().unwrap_or("?")
            ));
        }
        let bridge = Arc::new(Bridge {
            holds_path,
            affinity_path,
            downscale_path,
            display_path,
            publisher,
            state: Mutex::new(State {
                thermal_refresh_pending: holds.thermal_held,
                thermal_retry: Default::default(),
                affinity,
                downscale,
                display,
                holds,
                holds_recovery_blocked,
            }),
            saver_override: AtomicI8::new(-1),
            flags,
            blocked: AtomicBool::new(false),
        });
        bridge.refresh_attribution();
        bridge
    }

    /// Atomic, durable write (tmp + fsync + rename + directory fsync). A crash
    /// mid-write must never corrupt the restore points. Callers performing a
    /// write-ahead hold must check the returned error before touching the device.
    pub(crate) fn persist(&self, st: &State) -> Result<(), String> {
        let file = HoldsFile {
            saver_held: st.holds.saver_held,
            saver_saved: st.holds.saver_saved,
            input_suspend_held: st.holds.input_suspend_held,
            input_suspend_saved: st.holds.input_suspend_saved.clone(),
            thermal_held: st.holds.thermal_held,
            thermal_saved_sconfig: st.holds.thermal_saved_sconfig,
            thermal_profile: st.holds.thermal_profile.clone(),
            thermal_yield: st.holds.thermal_yield,
        };
        let result = write_json_atomic(&self.holds_path, &file);
        if let Err(e) = &result {
            self.log_event(format!("holds persistence failed: {e}"));
        }
        result
    }

    /// Persists the thread-placement state (`affinity.json`), separately from
    /// `holds.json` because a game can own a dozen per-tid restore points.
    pub(crate) fn persist_affinity(&self, st: &State) -> Result<(), String> {
        let result = self.persist_json(&self.affinity_path, &st.affinity);
        if let Err(e) = &result {
            self.log_event(format!("affinity state persistence failed: {e}"));
        }
        result
    }

    /// Persists per-app render-scale holds (`downscale.json`).
    pub(crate) fn persist_downscale(&self, st: &State) -> Result<(), String> {
        let result = self.persist_json(&self.downscale_path, &st.downscale);
        if let Err(e) = &result {
            self.log_event(format!("render-scale state persistence failed: {e}"));
        }
        result
    }

    /// Persists the global display original/preset (`display.json`).
    pub(crate) fn persist_display(&self, st: &State) -> Result<(), String> {
        let result = self.persist_json(&self.display_path, &st.display);
        if let Err(e) = &result {
            self.log_event(format!("display state persistence failed: {e}"));
        }
        result
    }

    fn persist_json<T: serde::Serialize>(&self, path: &Path, value: &T) -> Result<(), String> {
        write_json_atomic(path, value)
    }
}

fn read_holds_file(path: &Path) -> Result<Holds, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let file: HoldsFile =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Holds {
        saver_held: file.saver_held,
        saver_saved: file.saver_saved,
        input_suspend_held: file.input_suspend_held,
        input_suspend_saved: file.input_suspend_saved,
        thermal_held: file.thermal_held,
        thermal_saved_sconfig: file.thermal_saved_sconfig,
        thermal_profile: file.thermal_profile,
        thermal_yield: file.thermal_yield,
    })
}

fn holds_recovery_blocked(missing: bool, load_error: Option<&str>, live_thermal: bool) -> bool {
    live_thermal && (missing || load_error.is_some())
}

fn write_json_atomic<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|e| format!("{}: serialize: {e}", path.display()))?;
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    if let Some(parent) = parent {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let tmp = path.with_extension("json.tmp");
    let result = (|| {
        let mut file = fs::File::create(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
        file.write_all(text.as_bytes())
            .map_err(|e| format!("{}: {e}", tmp.display()))?;
        file.sync_all()
            .map_err(|e| format!("{}: fsync: {e}", tmp.display()))?;
        fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))?;
        sync_parent(path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

#[cfg(unix)]
fn sync_parent(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::File::open(parent)
        .and_then(|dir| dir.sync_all())
        .map_err(|e| format!("{}: fsync: {e}", parent.display()))
}

#[cfg(not(unix))]
fn sync_parent(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests;
