//! Input-suspend orphan policy tests.

use super::*;
use crate::daemon::Publisher;

#[test]
fn only_an_unowned_live_one_is_an_orphan() {
    assert_eq!(orphan_release_value(false, Some("1")), Some("0"));
    assert_eq!(orphan_release_value(false, Some("0")), None);
    assert_eq!(orphan_release_value(false, None), None);
    assert_eq!(
        orphan_release_value(true, Some("1")),
        None,
        "a persisted hold owns the live value"
    );
}

#[test]
fn engage_fails_closed_when_the_restore_point_cannot_be_persisted() {
    let dir = crate::daemon::test_util::tmpdir("input-suspend-write-ahead-fail");
    fs::create_dir_all(dir.join("holds.json")).unwrap();
    let bridge = Bridge::recover(&dir, Publisher::new(), None);
    let ctx = SyncCtx {
        enabled: true,
        last_real: None,
        screen_on: true,
        locked: false,
        dynamic: true,
        app_map: std::collections::BTreeMap::new(),
        multi_window: false,
        live_saver: false,
        profile: None,
        custom_thermal: false,
        rnd_input_suspend: true,
        thread_affinity: false,
        app_downscale: std::collections::BTreeMap::new(),
        display_scale: "off".into(),
    };
    let mut st = bridge.lock();
    assert!(!bridge.sync_input_suspend_with_live(&mut st, &ctx, None));
    assert!(
        !st.holds.input_suspend_held,
        "failed persistence must roll back"
    );
    let _ = fs::remove_dir_all(&dir);
}
