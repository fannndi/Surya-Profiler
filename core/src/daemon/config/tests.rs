use super::*;
use crate::daemon::test_util::tmpdir;

fn write(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
}

#[test]
fn defaults_are_service_ready() {
    let c = DaemonConfig::default();
    assert!(c.enabled && c.dynamic);
    assert!(c.custom_thermal, "thermal takeover defaults on");
    assert!(c.freq_floor_audit, "freq floor audit defaults on");
    assert_eq!(c.log_level, None, "log level comes from env by default");
    assert_eq!(c.schema, SCHEMA);
    assert_eq!(c.base_profile, "balance");
    assert!(c.app_map.is_empty());
    assert!(!c.rnd_input_suspend, "R&D input suspend is opt-in");
}

#[test]
fn effective_map_drops_unknown_profiles() {
    let c: DaemonConfig = serde_json::from_str(
        r#"{"app_map":{"com.a":"game","com.b":"boost","com.c":"nope","com.d":"sleep"}}"#,
    )
    .unwrap();
    let map = c.effective_app_map();
    assert_eq!(map.get("com.a").map(String::as_str), Some("game"));
    assert!(!map.contains_key("com.b"), "boost no longer exists");
    assert!(!map.contains_key("com.c"), "typo is ignored");
    assert!(
        !map.contains_key("com.d"),
        "sleep is automatic, not a user mapping"
    );
}

#[test]
fn effective_base_profile_falls_back_to_balance() {
    let c = DaemonConfig {
        base_profile: "nope".into(),
        ..Default::default()
    };
    assert_eq!(c.effective_base_profile(), "balance");
    let c = DaemonConfig {
        base_profile: "game".into(),
        ..Default::default()
    };
    assert_eq!(c.effective_base_profile(), "game");
    let c = DaemonConfig {
        base_profile: "sleep".into(),
        ..Default::default()
    };
    assert_eq!(
        c.effective_base_profile(),
        "balance",
        "sleep is automatic, not a selectable base"
    );
}

#[test]
fn partial_json_fills_defaults() {
    let c: DaemonConfig = serde_json::from_str(r#"{"base_profile":"game"}"#).unwrap();
    assert_eq!(c.base_profile, "game");
    assert!(c.enabled, "missing enabled must default to true");
    assert!(c.dynamic);
    assert!(c.custom_thermal, "missing custom_thermal defaults on");
    assert_eq!(c.schema, SCHEMA);
    assert!(!c.rnd_input_suspend);
}

#[test]
fn legacy_schema_one_keys_are_ignored() {
    // A schema-1 file still parses; only the slim keys matter.
    let c: DaemonConfig = serde_json::from_str(
        r#"{"schema":1,"enabled":false,"dynamic":true,"base_profile":"powersave",
            "app_map":{"com.x":"game"},
            "sync_miui_saver":true,"guard_thermal":true,"charge_limit":true,
            "app_profiles":{"com.x":{"profile":"game","dnd":"total"}}}"#,
    )
    .unwrap();
    assert!(!c.enabled);
    assert_eq!(c.base_profile, "powersave");
    assert_eq!(
        c.effective_app_map().get("com.x").map(String::as_str),
        Some("game")
    );
    assert!(!c.rnd_input_suspend);
}

#[test]
fn custom_thermal_can_be_disabled() {
    let c: DaemonConfig = serde_json::from_str(r#"{"custom_thermal":false}"#).unwrap();
    assert!(!c.custom_thermal);
}

#[test]
fn freq_audit_and_log_level_roundtrip() {
    let c: DaemonConfig =
        serde_json::from_str(r#"{"freq_floor_audit":false,"log_level":"debug"}"#).unwrap();
    assert!(!c.freq_floor_audit, "explicit off must stick");
    assert_eq!(c.log_level.as_deref(), Some("debug"));

    // fail-closed keeps the audit off too: no tuning from invented defaults
    let (cfg, _) = ConfigFile::load(&tmpdir("cfg-broken").join("missing.json"));
    assert!(!cfg.get().freq_floor_audit);
}

#[test]
fn rnd_switch_roundtrips() {
    let c: DaemonConfig = serde_json::from_str(r#"{"rnd_input_suspend":true}"#).unwrap();
    assert!(c.rnd_input_suspend);
}

#[test]
fn thread_affinity_is_opt_in() {
    let c = DaemonConfig::default();
    assert!(!c.thread_affinity, "thread placement must default off");
    let c: DaemonConfig = serde_json::from_str(r#"{"thread_affinity":true}"#).unwrap();
    assert!(c.thread_affinity);
    let c: DaemonConfig = serde_json::from_str(r#"{"app_map":{"com.g":"game"}}"#).unwrap();
    assert!(!c.thread_affinity, "missing key stays off");
}

#[test]
fn load_missing_file_fails_closed() {
    let dir = tmpdir("cfg-missing");
    let (cfg, err) = ConfigFile::load(&dir.join("config.json"));
    assert!(err.is_some());
    assert!(!cfg.get().enabled, "missing config must not tune");
    assert!(!cfg.get().custom_thermal);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn newer_schema_is_rejected_on_load_and_reload() {
    let dir = tmpdir("cfg-schema");
    let path = dir.join("config.json");
    write(&path, r#"{"schema":99,"enabled":true}"#);
    let (mut cfg, err) = ConfigFile::load(&path);
    assert!(err.is_some());
    assert!(!cfg.get().enabled, "newer schema must fail closed");

    // A valid file, then a newer-schema rewrite: reload keeps the good one.
    write(
        &path,
        r#"{"schema":2,"enabled":true,"base_profile":"game"}"#,
    );
    assert!(cfg.reload());
    assert!(cfg.get().enabled);
    assert_eq!(cfg.get().base_profile, "game");
    write(
        &path,
        r#"{"schema":99,"enabled":false,"base_profile":"powersave"}"#,
    );
    assert!(!cfg.reload());
    assert!(cfg.get().enabled);
    assert_eq!(cfg.get().base_profile, "game");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn reload_detects_change_and_keeps_good_on_parse_error() {
    let dir = tmpdir("cfg-reload");
    let path = dir.join("config.json");
    write(&path, r#"{"base_profile":"balance"}"#);
    let (mut cfg, err) = ConfigFile::load(&path);
    assert!(err.is_none());
    assert_eq!(cfg.get().base_profile, "balance");

    // no change -> false
    assert!(!cfg.reload_if_changed());

    // change -> true + new value
    std::thread::sleep(std::time::Duration::from_millis(1100)); // mtime granularity
    write(&path, r#"{"base_profile":"game","dynamic":false}"#);
    assert!(cfg.reload_if_changed());
    assert_eq!(cfg.get().base_profile, "game");
    assert!(!cfg.get().dynamic);

    // broken change -> false, previous value kept
    std::thread::sleep(std::time::Duration::from_millis(1100));
    write(&path, "{broken");
    assert!(!cfg.reload_if_changed());
    assert_eq!(cfg.get().base_profile, "game");
    let _ = fs::remove_dir_all(&dir);
}
