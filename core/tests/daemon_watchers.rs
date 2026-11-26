//! Host E2E for the daemon's foreground watcher, using a fake `logcat`
//! script. Exercises the real spawn/parse/freshness pipeline without a
//! device: initial package, live switch on demand, and the `-d` wake peek.

mod common;

use common::{tmp, Daemon};
use serde_json::json;
use std::time::Duration;

fn write_fake_logcat(dir: &std::path::Path) {
    let script = r#"#!/bin/sh
case "$*" in
  *"-d"*)
    EPOCH=$(date +%s.%3N)
    echo "$EPOCH  1234  1234 I wm_resume_activity: [0,1,2,com.YoStarEN.AzurLane/.Main]"
    ;;
  *)
    if [ "${SURYAPROFILER_SILENT_STREAM:-0}" = "1" ]; then
      sleep 120
      exit 0
    fi
    EPOCH=$(date +%s.%3N)
    echo "$EPOCH  1234  1234 I wm_resume_activity: [0,1,2,com.whatsapp/.Main]"
    i=0
    while [ $i -lt 480 ]; do
      if [ -f "$SURYAPROFILER_FG_FILE" ]; then
        EPOCH=$(date +%s.%3N)
        echo "$EPOCH  1234  1234 I wm_resume_activity: [0,1,2,com.google.android.youtube/.Main]"
        rm -f "$SURYAPROFILER_FG_FILE"
      fi
      sleep 0.25
      i=$((i+1))
    done
    ;;
esac
"#;
    let path = dir.join("fake-logcat.sh");
    std::fs::write(&path, script).unwrap();
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).unwrap();
}

#[test]
fn startup_peek_keeps_mapped_foreground_with_a_silent_stream() {
    let dir = tmp("e2e-startup-peek");
    write_fake_logcat(&dir);
    std::fs::write(
        dir.join("config.json"),
        r#"{"schema":2,"enabled":true,"dynamic":true,"base_profile":"balance",
            "app_map":{"com.YoStarEN.AzurLane":"game"}}"#,
    )
    .unwrap();

    let logcat = dir.join("fake-logcat.sh");
    let mut d = Daemon::spawn_env(
        &dir.join("state"),
        &dir.join("config.json"),
        &[
            ("SURYAPROFILER_LOGCAT_BIN", logcat.to_str().unwrap()),
            ("SURYAPROFILER_SILENT_STREAM", "1"),
        ],
    );

    d.send(json!({"cmd":"hello"}));
    let applied = d.wait_for(
        |v| v["event"] == "applied" && v["profile"] == "game",
        Duration::from_secs(45),
    );
    assert_eq!(applied["reason"], "app");
    assert_eq!(applied["src_pkg"], "com.YoStarEN.AzurLane");

    d.send(json!({"cmd":"shutdown"}));
    d.wait_for(|v| v["event"] == "bye", Duration::from_secs(5));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn watchers_drive_the_full_pipeline() {
    let dir = tmp("e2e");
    write_fake_logcat(&dir);
    std::fs::write(
        dir.join("config.json"),
        r#"{"schema":2,"enabled":true,"dynamic":true,"base_profile":"balance",
            "app_map":{"com.YoStarEN.AzurLane":"game","com.google.android.youtube":"powersave"}}"#,
    )
    .unwrap();

    let logcat = dir.join("fake-logcat.sh");
    let fg = dir.join("fg-on");
    let mut d = Daemon::spawn_env(
        &dir.join("state"),
        &dir.join("config.json"),
        &[
            ("SURYAPROFILER_LOGCAT_BIN", logcat.to_str().unwrap()),
            ("SURYAPROFILER_FG_FILE", fg.to_str().unwrap()),
        ],
    );

    // The fake foreground stream starts with whatsapp -> base apply.
    d.send(json!({"cmd":"hello"}));
    let applied = d.wait_for(
        |v| v["event"] == "applied" && v["profile"] == "balance",
        Duration::from_secs(45),
    );
    assert_eq!(applied["reason"], "base");

    // Wake seed: screen on/unlocked -> peek (-d) returns Azur Lane -> game.
    d.send(json!({"cmd":"screen","on":true,"locked":false}));
    let applied = d.wait_for(
        |v| v["event"] == "applied" && v["profile"] == "game",
        Duration::from_secs(45),
    );
    assert_eq!(applied["reason"], "app");
    assert_eq!(applied["src_pkg"], "com.YoStarEN.AzurLane");

    // Live foreground switch to a powersave-mapped app.
    std::fs::write(dir.join("fg-on"), "1").unwrap();
    let applied = d.wait_for(
        |v| v["event"] == "applied" && v["profile"] == "powersave",
        Duration::from_secs(45),
    );
    assert_eq!(applied["reason"], "app");
    assert_eq!(applied["src_pkg"], "com.google.android.youtube");

    d.send(json!({"cmd":"shutdown"}));
    d.wait_for(|v| v["event"] == "bye", Duration::from_secs(5));

    let _ = std::fs::remove_dir_all(&dir);
}
