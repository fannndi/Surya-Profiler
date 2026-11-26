//! Service-off thermal release.

use super::*;

pub(super) fn set_sconfig_verified(value: i32) -> Result<(), String> {
    write_text(&sconfig_path(), &format!("{value}\n"))?;
    for _ in 0..5 {
        if read_i32(&sconfig_path()) == Some(value) {
            return Ok(());
        }
        sleep(Duration::from_millis(50));
    }
    Err(format!(
        "sconfig read-back is {:?}, wanted {value}",
        read_i32(&sconfig_path())
    ))
}

/// Stock value to write during release, or `None` when MIUI's live scenario
/// must be preserved. Kept pure so every ownership path is regression-tested.
pub(super) fn release_sconfig_target(live: i32, owned_live: bool, saved: i32) -> Option<i32> {
    if owned_live || thermal_scenario::is_default(live) {
        (live != saved).then_some(saved)
    } else if !thermal_scenario::is_miui_owned(live) {
        // Android's mi_thermald keeps the last config for unknown IDs;
        // force vendor normal before deleting our map.
        Some(0)
    } else {
        None
    }
}

impl Bridge {
    pub(crate) fn release_thermal_locked(&self, st: &mut State) -> bool {
        let live = read_i32(&sconfig_path()).unwrap_or(0);
        if !st.holds.thermal_held && !marker_path().exists() && !is_custom_index(live) {
            return false;
        }
        let owned_live = is_custom_index(live) || (marker_path().exists() && is_legacy_index(live));
        let saved = st.holds.thermal_saved_sconfig.unwrap_or(0);

        if let Some(target) = release_sconfig_target(live, owned_live, saved) {
            if let Err(e) = set_sconfig_verified(target) {
                self.log_event(format!("thermal: sconfig restore failed: {e}"));
                return false;
            }
        }

        let removed = remove_owned();
        if !removed {
            self.log_event("thermal: config cleanup failed (marker or foreign map)".into());
            return false;
        }
        st.holds.thermal_held = false;
        st.thermal_retry.recovered();
        st.holds.thermal_saved_sconfig = None;
        st.holds.thermal_profile = None;
        st.holds.thermal_yield = None;
        self.log_event("thermal: custom profiles released (stock map)".into());
        true
    }
}
