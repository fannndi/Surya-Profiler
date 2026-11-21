//! `sconfig` selection with MIUI preemption detection.

use super::*;

pub(crate) enum SelectOutcome {
    Ok,
    /// MIUI wrote a yield scenario while we were selecting.
    Preempted(i32),
    Failed(String),
}

pub(crate) fn wait_loaded(profile: ThermalProfile) -> bool {
    let target = profile.sconfig();
    for _ in 0..20 {
        if read_i32(&sconfig_path()) == Some(target) && verify_loaded(profile) {
            return true;
        }
        sleep(Duration::from_millis(100));
    }
    false
}

pub(crate) fn is_yield_live(value: i32) -> bool {
    thermal_scenario::is_yield(value) || miui_performance_mode()
}

pub(crate) fn select_profile(profile: ThermalProfile) -> SelectOutcome {
    let target = profile.sconfig();
    if let Err(e) = write_text(&sconfig_path(), &format!("{target}\n")) {
        return SelectOutcome::Failed(e);
    }
    if wait_loaded(profile) {
        return SelectOutcome::Ok;
    }
    if let Some(live) = read_i32(&sconfig_path()) {
        if is_yield_live(live) {
            return SelectOutcome::Preempted(live);
        }
    }
    // One forced reload through the stock scenario. The transient normal
    // config is harmless; mi_thermald applies it for a few hundred ms.
    let _ = write_text(&sconfig_path(), "0\n");
    sleep(Duration::from_millis(250));
    if let Some(live) = read_i32(&sconfig_path()) {
        if is_yield_live(live) {
            return SelectOutcome::Preempted(live);
        }
    }
    if let Err(e) = write_text(&sconfig_path(), &format!("{target}\n")) {
        return SelectOutcome::Failed(e);
    }
    if wait_loaded(profile) {
        return SelectOutcome::Ok;
    }
    if let Some(live) = read_i32(&sconfig_path()) {
        if is_yield_live(live) {
            return SelectOutcome::Preempted(live);
        }
    }
    SelectOutcome::Failed(format!("mi_thermald did not load {}", profile.id()))
}
