//! Bridge state/release tests.

use super::*;

#[test]
fn holds_file_roundtrip() {
    let dir = super::super::test_util::tmpdir("bridge-holds");
    let f = HoldsFile {
        saver_held: true,
        saver_saved: true,
        input_suspend_held: true,
        input_suspend_saved: Some("0".into()),
        thermal_held: true,
        thermal_saved_sconfig: Some(0),
        thermal_profile: Some("game".into()),
        thermal_yield: Some(12),
    };
    std::fs::write(dir.join("holds.json"), serde_json::to_string(&f).unwrap()).unwrap();
    let publisher = Publisher::new();
    let bridge = Bridge::recover(&dir, publisher, None);
    let st = bridge.lock();
    assert!(st.holds.saver_held);
    assert!(st.holds.saver_saved);
    assert!(st.holds.input_suspend_held);
    assert_eq!(st.holds.input_suspend_saved.as_deref(), Some("0"));
    drop(st);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn blocked_bridge_ignores_syncs() {
    let dir = super::super::test_util::tmpdir("bridge-blocked");
    let bridge = Bridge::recover(&dir, Publisher::new(), None);
    bridge.blocked.store(true, Ordering::Release);
    let ctx = SyncCtx {
        enabled: true,
        last_real: Some("com.p".into()),
        screen_on: true,
        locked: false,
        dynamic: true,
        app_map: [("com.p".to_string(), "powersave".to_string())]
            .into_iter()
            .collect(),
        multi_window: false,
        live_saver: false,
        profile: Some("powersave".into()),
        custom_thermal: true,
        rnd_input_suspend: true,
    };
    bridge.sync(&ctx);
    let st = bridge.lock();
    assert!(!st.holds.saver_held);
    assert!(!st.holds.input_suspend_held);
    drop(st);
    bridge.resume();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn release_all_reports_a_failed_saver_restore() {
    let dir = super::super::test_util::tmpdir("bridge-release-fail");
    let script = dir.join("settings-fail");
    fs::write(&script, "#!/bin/sh\nexit 1\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fs::write(
        dir.join("holds.json"),
        r#"{"saver_held":true,"saver_saved":false,
            "input_suspend_held":false,"input_suspend_saved":null,
            "thermal_held":false,"thermal_saved_sconfig":null,
            "thermal_profile":null,"thermal_yield":null}"#,
    )
    .unwrap();

    std::env::set_var(settings::SETTINGS_BIN_ENV, &script);
    let bridge = Bridge::recover(&dir, Publisher::new(), None);
    let report = bridge.release_all();
    std::env::remove_var(settings::SETTINGS_BIN_ENV);

    assert!(!report.saver_ok, "{report:?}");
    assert!(!report.ok());
    assert_eq!(report.failures(), vec!["saver"]);
    // A failed release must keep the hold persisted for the next retry.
    let v: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("holds.json")).unwrap()).unwrap();
    assert_eq!(v["saver_held"], true, "{v:?}");
    let _ = fs::remove_dir_all(&dir);
}
