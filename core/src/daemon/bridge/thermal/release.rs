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

impl Bridge {
    pub(crate) fn release_thermal_locked(&self, st: &mut State) -> bool {
        if !st.holds.thermal_held && !marker_path().exists() {
            return false;
        }
        let live = read_i32(&sconfig_path()).unwrap_or(0);
        let owned_live = is_custom_index(live) || (marker_path().exists() && is_legacy_index(live));
        let default_live = live == 0 || thermal_scenario::is_default(live);

        if owned_live || default_live {
            let saved = st.holds.thermal_saved_sconfig.unwrap_or(0);
            if live != saved {
                if let Err(e) = set_sconfig_verified(saved) {
                    self.log_event(format!("thermal: sconfig restore failed: {e}"));
                    return false;
                }
            }
        } else if !thermal_scenario::has_vendor_map_entry(live) {
            // Android's mi_thermald keeps the last config for unknown IDs;
            // force vendor normal before deleting our map.
            if let Err(e) = set_sconfig_verified(0) {
                self.log_event(format!("thermal: sconfig stock fallback failed: {e}"));
                return false;
            }
        }

        let removed = remove_owned();
        if !removed {
            self.log_event("thermal: config cleanup failed (marker or foreign map)".into());
            return false;
        }
        st.holds.thermal_held = false;
        st.thermal_failures = 0;
        st.thermal_retry_at = None;
        st.holds.thermal_saved_sconfig = None;
        st.holds.thermal_profile = None;
        st.holds.thermal_yield = None;
        self.log_event("thermal: custom profiles released (stock map)".into());
        true
    }
}
