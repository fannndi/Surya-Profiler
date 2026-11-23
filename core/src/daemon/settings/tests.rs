//! settings unit tests (split from settings.rs).

use super::*;
use std::fs;
use std::path::PathBuf;

/// Fake `settings` script: get/put against a state dir (one file per key).
fn fake_settings(tag: &str) -> (PathBuf, PathBuf) {
    let dir = crate::daemon::test_util::tmpdir(tag);
    let state = dir.join("state");
    fs::create_dir_all(&state).unwrap();
    let script = dir.join("settings");
    fs::write(
        &script,
        format!(
            "#!/bin/sh\ns={}\ncase \"$1\" in\nget) cat \"$s/$2.$3\" 2>/dev/null ;;\nput) printf '%s' \"$4\" > \"$s/$2.$3\" ;;\nesac\n",
            state.display()
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perm = fs::metadata(&script).unwrap().permissions();
        perm.set_mode(0o755);
        fs::set_permissions(&script, perm).unwrap();
    }
    (script, state)
}

#[test]
fn read_put_roundtrip_through_a_fake_binary() {
    let (script, _state) = fake_settings("settings-rt");
    let bin = script.to_str().unwrap();

    // unset value -> None (script exits 0 with empty output)
    assert_eq!(read_with(bin, "global", SAVER_KEY), None);

    put_with(bin, "global", SAVER_KEY, "1").unwrap();
    assert_eq!(read_with(bin, "global", SAVER_KEY).as_deref(), Some("1"));

    put_with(bin, "global", SAVER_KEY, "0").unwrap();
    assert_eq!(read_with(bin, "global", SAVER_KEY).as_deref(), Some("0"));
}

#[test]
fn missing_binary_is_none_not_panic() {
    assert_eq!(
        read_with("/nonexistent/settings-bin", "global", SAVER_KEY),
        None
    );
    assert!(put_with("/nonexistent/settings-bin", "global", SAVER_KEY, "1").is_err());
}
