//! Hold persistence and recovery tests.

use super::*;
use crate::daemon::test_util::tmpdir;

#[test]
fn persist_round_trips_restore_points_and_reports_success() {
    let dir = tmpdir("bridge-persist-ok");
    let bridge = Bridge::recover(&dir, Publisher::new(), None);
    {
        let mut st = bridge.lock();
        st.holds.input_suspend_held = true;
        st.holds.input_suspend_saved = Some("0".into());
        st.holds.thermal_held = true;
        st.holds.thermal_saved_sconfig = Some(112);
        bridge.persist(&st).expect("persist must succeed");
    }

    let saved: HoldsFile =
        serde_json::from_str(&fs::read_to_string(dir.join("holds.json")).unwrap()).unwrap();
    assert!(saved.input_suspend_held);
    assert_eq!(saved.input_suspend_saved.as_deref(), Some("0"));
    assert!(saved.thermal_held);
    assert_eq!(saved.thermal_saved_sconfig, Some(112));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn persist_returns_an_error_instead_of_pretending_a_hold_is_durable() {
    let dir = tmpdir("bridge-persist-fail");
    // A directory at the destination makes the final atomic rename fail.
    fs::create_dir_all(dir.join("holds.json")).unwrap();
    let bridge = Bridge::recover(&dir, Publisher::new(), None);
    let mut st = bridge.lock();
    st.holds.input_suspend_held = true;
    st.holds.input_suspend_saved = Some("0".into());

    let err = bridge
        .persist(&st)
        .expect_err("persistence must fail closed");
    assert!(err.contains("holds.json"), "{err}");
    assert!(dir.join("holds.json").is_dir());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn persist_json_propagates_atomic_write_failures() {
    let dir = tmpdir("bridge-persist-json-fail");
    let bridge = Bridge::recover(&dir, Publisher::new(), None);
    fs::create_dir_all(dir.join("affinity.json")).unwrap();

    let err = bridge
        .persist_json(
            &dir.join("affinity.json"),
            &serde_json::json!({"held": true}),
        )
        .expect_err("JSON persistence must report failure");
    assert!(err.contains("affinity.json"), "{err}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn bad_holds_state_blocks_only_when_a_thermal_takeover_is_live() {
    assert!(holds_recovery_blocked(true, None, true));
    assert!(holds_recovery_blocked(false, Some("corrupt JSON"), true));
    assert!(!holds_recovery_blocked(true, None, false));
    assert!(!holds_recovery_blocked(false, Some("corrupt JSON"), false));
}

#[test]
fn corrupt_holds_file_is_reported_as_a_load_error() {
    let dir = tmpdir("bridge-load-corrupt");
    fs::write(dir.join("holds.json"), "{not json").unwrap();
    assert!(read_holds_file(&dir.join("holds.json")).is_err());
    let _ = fs::remove_dir_all(&dir);
}
