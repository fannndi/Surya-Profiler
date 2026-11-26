//! Thermal retry state-machine tests.

use super::super::ThermalRetry;
use super::*;
use crate::daemon::Publisher;
use std::time::Duration;

#[test]
fn same_profile_failure_keeps_its_backoff() {
    let now = Instant::now();
    let mut retry = ThermalRetry::default();
    assert_eq!(retry.failed("balance", now), Duration::from_secs(15));
    assert!(retry.blocks("balance", now + Duration::from_secs(14)));
    assert!(!retry.blocks("balance", now + Duration::from_secs(15)));
    assert_eq!(
        retry.failed("balance", now + Duration::from_secs(15)),
        Duration::from_secs(30),
        "an expired same-profile retry keeps the failure ladder"
    );
}

#[test]
fn profile_change_bypasses_the_old_retry_deadline() {
    let now = Instant::now();
    let mut retry = ThermalRetry::default();
    retry.failed("balance", now);

    assert!(
        !retry.blocks("game", now + Duration::from_secs(1)),
        "a newly requested profile must not wait for balance's backoff"
    );
    assert_eq!(
        retry.failed("game", now + Duration::from_secs(1)),
        Duration::from_secs(15),
        "the new profile starts its own failure ladder"
    );
    assert!(retry.blocks("game", now + Duration::from_secs(2)));
    assert!(!retry.blocks("balance", now + Duration::from_secs(2)));
}

#[test]
fn repeated_same_profile_failures_increase_backoff() {
    let now = Instant::now();
    let mut retry = ThermalRetry::default();
    assert_eq!(retry.failed("game", now), Duration::from_secs(15));
    assert_eq!(
        retry.failed("game", now + Duration::from_secs(15)),
        Duration::from_secs(30)
    );
    assert!(retry.blocks("game", now + Duration::from_secs(44)));
    assert!(!retry.blocks("game", now + Duration::from_secs(45)));
}

#[test]
fn recovery_clears_the_deadline_and_failure_ladder() {
    let now = Instant::now();
    let mut retry = ThermalRetry::default();
    retry.failed("game", now);
    retry.failed("game", now + Duration::from_secs(15));
    retry.recovered();

    assert!(!retry.blocks("game", now + Duration::from_secs(16)));
    assert_eq!(
        retry.failed("game", now + Duration::from_secs(17)),
        Duration::from_secs(15)
    );
}

#[test]
fn thermal_write_ahead_refuses_a_persistence_failure() {
    let dir = crate::daemon::test_util::tmpdir("thermal-write-ahead-fail");
    fs::create_dir_all(dir.join("holds.json")).unwrap();
    let bridge = Bridge::recover(&dir, Publisher::new(), None);
    let mut st = bridge.lock();
    st.holds.thermal_held = true;
    st.holds.thermal_saved_sconfig = Some(0);

    assert!(!bridge.persist_thermal_before_write(&st));
    let _ = fs::remove_dir_all(&dir);
}
