//! Bridge recovery + holds persistence.

use super::*;

impl Bridge {
    /// Loads persisted holds from a previous run (crash recovery).
    pub(crate) fn recover(
        state_dir: &Path,
        publisher: Arc<Publisher>,
        flags: Option<Arc<LiveFlags>>,
    ) -> Arc<Bridge> {
        let holds_path = state_dir.join("holds.json");
        let affinity_path = state_dir.join("affinity.json");
        let holds = fs::read_to_string(&holds_path)
            .ok()
            .and_then(|s| serde_json::from_str::<HoldsFile>(&s).ok())
            .map(|f| Holds {
                saver_held: f.saver_held,
                saver_saved: f.saver_saved,
                input_suspend_held: f.input_suspend_held,
                input_suspend_saved: f.input_suspend_saved,
                thermal_held: f.thermal_held,
                thermal_saved_sconfig: f.thermal_saved_sconfig,
                thermal_profile: f.thermal_profile,
                thermal_yield: f.thermal_yield,
            })
            .unwrap_or_default();
        let affinity = fs::read_to_string(&affinity_path)
            .ok()
            .and_then(|s| serde_json::from_str::<affinity::AffinityState>(&s).ok())
            .unwrap_or_default();
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
        let bridge = Arc::new(Bridge {
            holds_path,
            affinity_path,
            publisher,
            state: Mutex::new(State {
                thermal_refresh_pending: holds.thermal_held,
                thermal_retry_at: None,
                thermal_failures: 0,
                affinity,
                holds,
            }),
            saver_override: AtomicI8::new(-1),
            flags,
            blocked: AtomicBool::new(false),
        });
        bridge.refresh_attribution();
        bridge
    }

    /// Atomic write (tmp + rename): a crash mid-write must never corrupt
    /// the restore points.
    pub(crate) fn persist(&self, st: &State) {
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
        let Ok(s) = serde_json::to_string_pretty(&file) else {
            return;
        };
        use std::io::Write;
        let tmp = self.holds_path.with_extension("json.tmp");
        let Ok(mut f) = fs::File::create(&tmp) else {
            return;
        };
        if f.write_all(s.as_bytes()).is_ok() && f.sync_all().is_ok() {
            let _ = fs::rename(&tmp, &self.holds_path);
        }
    }

    /// Persists the thread-placement state (`affinity.json`), separately from
    /// `holds.json` because a game can own a dozen per-tid restore points.
    pub(crate) fn persist_affinity(&self, st: &State) {
        let Ok(s) = serde_json::to_string_pretty(&st.affinity) else {
            return;
        };
        use std::io::Write;
        let tmp = self.affinity_path.with_extension("json.tmp");
        let Ok(mut f) = fs::File::create(&tmp) else {
            return;
        };
        if f.write_all(s.as_bytes()).is_ok() && f.sync_all().is_ok() {
            let _ = fs::rename(&tmp, &self.affinity_path);
        }
    }
}
