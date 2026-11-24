//! Host E2E for per-app render scale + global display scale, using fake
//! `cmd`, `dumpsys` and `wm` binaries. Exercises capture → apply → restore.

mod common;

use common::{tmp, Daemon};
use serde_json::json;
use std::path::Path;
use std::time::{Duration, Instant};

fn executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn write_scripts(dir: &Path) -> (String, String, String, String) {
    let cmd_state = dir.join("cmd-state");
    let cmd_log = dir.join("cmd.log");
    let cmd = dir.join("cmd.sh");
    std::fs::write(
        &cmd,
        format!(
            r#"#!/bin/sh
echo "$@" >> "{log}"
case "$1 $2" in
  "game help") echo "Game manager (game) commands:"; echo "  downscale [0.3|...|disable] <PACKAGE_NAME>"; exit 0 ;;
  "game downscale")
    if [ "$3" = "disable" ]; then echo off > "{state}.$4"; else echo "$3" > "{state}.$4"; fi
    exit 0 ;;
esac
exit 0
"#,
            log = cmd_log.display(),
            state = cmd_state.display()
        ),
    )
    .unwrap();
    executable(&cmd);

    let dumpsys = dir.join("dumpsys.sh");
    let state = cmd_state.clone();
    std::fs::write(
        &dumpsys,
        format!(
            r#"#!/bin/sh
case "$*" in
  *platform_compat*)
    pkg=com.test.game
    s=$(cat "{state}.$pkg" 2>/dev/null || echo off)
    gate=false; s70=false; s80=false
    case "$s" in
      0.7) gate=true; s70=true ;;
      0.8) gate=true; s80=true ;;
    esac
    printf 'ChangeId(168419799; name=DOWNSCALED; disabled; rawOverrides={{%s=%s}})\n' "$pkg" "$gate"
    printf 'ChangeId(176926829; name=DOWNSCALE_70; disabled; rawOverrides={{%s=%s}})\n' "$pkg" "$s70"
    printf 'ChangeId(176926753; name=DOWNSCALE_80; disabled; rawOverrides={{%s=%s}})\n' "$pkg" "$s80"
    exit 0 ;;
esac
exit 0
"#,
            state = state.display()
        ),
    )
    .unwrap();
    executable(&dumpsys);

    let wm = dir.join("wm.sh");
    let wm_log = dir.join("wm.log");
    std::fs::write(
        &wm,
        format!(
            r#"#!/bin/sh
echo "$@" >> "{log}"
case "$1" in
  size)
    if [ "$2" = "reset" ]; then rm -f "{d}/size"; exit 0; fi
    if [ -n "$2" ]; then echo "$2" > "{d}/size"; exit 0; fi
    echo "Physical size: 1080x2400"
    [ -f "{d}/size" ] && echo "Override size: $(cat {d}/size)"
    exit 0 ;;
  density)
    if [ "$2" = "reset" ]; then rm -f "{d}/density"; exit 0; fi
    if [ -n "$2" ]; then echo "$2" > "{d}/density"; exit 0; fi
    echo "Physical density: 440"
    [ -f "{d}/density" ] && echo "Override density: $(cat {d}/density)"
    exit 0 ;;
esac
exit 0
"#,
            log = wm_log.display(),
            d = dir.display()
        ),
    )
    .unwrap();
    executable(&wm);

    let logcat = dir.join("logcat.sh");
    std::fs::write(
        &logcat,
        "#!/bin/sh\ncase \"$*\" in *\"-d\"*) exit 0 ;; esac\nwhile true; do sleep 1; done\n",
    )
    .unwrap();
    executable(&logcat);

    (
        cmd.to_str().unwrap().to_string(),
        dumpsys.to_str().unwrap().to_string(),
        wm.to_str().unwrap().to_string(),
        logcat.to_str().unwrap().to_string(),
    )
}

fn wait_text(path: &Path, needle: &str) -> String {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        if text.contains(needle) {
            return text;
        }
        assert!(
            Instant::now() < deadline,
            "timeout waiting for {needle} in {}: {text}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn write_config(dir: &Path, body: &str) {
    std::fs::write(dir.join("config.json"), body).unwrap();
}

#[test]
fn per_app_downscale_captures_applies_and_restores() {
    let dir = tmp("downscale-e2e");
    let (cmd, dumpsys, wm, logcat) = write_scripts(&dir);
    write_config(
        &dir,
        r#"{"schema":2,"enabled":true,"dynamic":true,"base_profile":"balance",
            "app_map":{"com.test.game":"game"},
            "app_downscale":{"com.test.game":"0.7"}}"#,
    );
    // MIUI had already set 0.8 for this package: restore must go back to 0.8.
    std::fs::write(dir.join("cmd-state.com.test.game"), "0.8").unwrap();

    let mut d = Daemon::spawn_env(
        &dir.join("state"),
        &dir.join("config.json"),
        &[
            ("SURYAPROFILER_CMD_BIN", cmd.as_str()),
            ("SURYAPROFILER_DUMPSYS_BIN", dumpsys.as_str()),
            ("SURYAPROFILER_WM_BIN", wm.as_str()),
            ("SURYAPROFILER_LOGCAT_BIN", logcat.as_str()),
        ],
    );
    d.send(json!({"cmd":"hello"}));
    d.wait_for(|v| v["event"] == "state", Duration::from_secs(10));
    d.send(json!({"cmd":"screen","on":true,"locked":false}));
    d.send(json!({"cmd":"user_present"}));
    d.send(json!({"cmd":"fg","pkg":"com.test.game"}));

    let cmd_log = dir.join("cmd.log");
    wait_text(&cmd_log, "game downscale 0.7 com.test.game");
    let state_path = dir.join("state/downscale.json");
    let state = wait_text(&state_path, "com.test.game");
    assert!(state.contains("com.test.game"), "{state}");
    assert!(state.contains("0.7"), "{state}");

    // Remove the per-app setting: restore the captured 0.8.
    write_config(
        &dir,
        r#"{"schema":2,"enabled":true,"dynamic":true,"base_profile":"balance",
            "app_map":{"com.test.game":"game"}}"#,
    );
    d.send(json!({"cmd":"config_changed"}));
    let text = wait_text(&cmd_log, "game downscale 0.8 com.test.game");
    assert!(text.contains("game downscale 0.8 com.test.game"), "{text}");

    // Apply again, then Service OFF must restore the prior value.
    write_config(
        &dir,
        r#"{"schema":2,"enabled":true,"dynamic":true,"base_profile":"balance",
            "app_map":{"com.test.game":"game"},
            "app_downscale":{"com.test.game":"0.7"}}"#,
    );
    d.send(json!({"cmd":"config_changed"}));
    wait_text(&cmd_log, "game downscale 0.7 com.test.game");
    d.send(json!({"cmd":"restore"}));
    let restored = d.wait_for(|v| v["event"] == "restored", Duration::from_secs(30));
    assert_eq!(restored["downscale_ok"], true, "{restored}");
    let text = std::fs::read_to_string(&cmd_log).unwrap();
    assert!(text.contains("game downscale 0.8 com.test.game"), "{text}");

    d.send(json!({"cmd":"shutdown"}));
    d.wait_for(|v| v["event"] == "bye", Duration::from_secs(5));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn display_scale_applies_and_restores_the_exact_original() {
    let dir = tmp("display-e2e");
    let (cmd, dumpsys, wm, logcat) = write_scripts(&dir);
    // User has a density override of 400 and no size override.
    std::fs::write(dir.join("density"), "400").unwrap();
    write_config(
        &dir,
        r#"{"schema":2,"enabled":true,"dynamic":true,"base_profile":"balance",
            "display_scale":"hd"}"#,
    );
    let mut d = Daemon::spawn_env(
        &dir.join("state"),
        &dir.join("config.json"),
        &[
            ("SURYAPROFILER_CMD_BIN", cmd.as_str()),
            ("SURYAPROFILER_DUMPSYS_BIN", dumpsys.as_str()),
            ("SURYAPROFILER_WM_BIN", wm.as_str()),
            ("SURYAPROFILER_LOGCAT_BIN", logcat.as_str()),
        ],
    );
    d.send(json!({"cmd":"hello"}));
    d.wait_for(|v| v["event"] == "state", Duration::from_secs(10));
    d.send(json!({"cmd":"screen","on":true,"locked":false}));
    d.send(json!({"cmd":"user_present"}));

    let log = dir.join("wm.log");
    wait_text(&log, "size 720x1600");
    wait_text(&log, "density 320");
    assert_eq!(
        std::fs::read_to_string(dir.join("size")).unwrap().trim(),
        "720x1600"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("density")).unwrap().trim(),
        "320"
    );
    let state = std::fs::read_to_string(dir.join("state/display.json")).unwrap();
    assert!(state.contains("\"hd\""), "{state}");

    // Off restores the exact original: no size override, density 400.
    write_config(
        &dir,
        r#"{"schema":2,"enabled":true,"dynamic":true,"base_profile":"balance",
            "display_scale":"off"}"#,
    );
    d.send(json!({"cmd":"config_changed"}));
    wait_text(&log, "size reset");
    wait_text(&log, "density 400");
    assert!(!dir.join("size").exists());
    assert_eq!(
        std::fs::read_to_string(dir.join("density")).unwrap().trim(),
        "400"
    );

    // Apply again, and verify Service OFF restores.
    write_config(
        &dir,
        r#"{"schema":2,"enabled":true,"dynamic":true,"base_profile":"balance",
            "display_scale":"hd"}"#,
    );
    d.send(json!({"cmd":"config_changed"}));
    wait_text(&log, "size 720x1600");
    d.send(json!({"cmd":"restore"}));
    let restored = d.wait_for(|v| v["event"] == "restored", Duration::from_secs(30));
    assert_eq!(restored["display_ok"], true, "{restored}");
    assert!(!dir.join("size").exists());
    assert_eq!(
        std::fs::read_to_string(dir.join("density")).unwrap().trim(),
        "400"
    );

    d.send(json!({"cmd":"shutdown"}));
    d.wait_for(|v| v["event"] == "bye", Duration::from_secs(5));
    let _ = std::fs::remove_dir_all(&dir);
}
