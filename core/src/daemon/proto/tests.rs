use super::*;

#[test]
fn commands_parse_from_tagged_json() {
    let c: Command = serde_json::from_str(r#"{"cmd":"screen","on":false,"locked":true}"#).unwrap();
    assert_eq!(
        c,
        Command::Screen {
            on: false,
            locked: true
        }
    );

    let c: Command = serde_json::from_str(r#"{"cmd":"set_base","profile":"game"}"#).unwrap();
    assert_eq!(
        c,
        Command::SetBase {
            profile: "game".into()
        }
    );

    let c: Command = serde_json::from_str(r#"{"cmd":"diag"}"#).unwrap();
    assert_eq!(c, Command::Diag);
    let c: Command = serde_json::from_str(r#"{"cmd":"stats"}"#).unwrap();
    assert_eq!(c, Command::Stats);
    let c: Command = serde_json::from_str(r#"{"cmd":"restore"}"#).unwrap();
    assert_eq!(c, Command::Restore);

    let c: Command = serde_json::from_str(r#"{"cmd":"power","watch":true}"#).unwrap();
    assert_eq!(
        c,
        Command::Power {
            watch: true,
            window_s: None
        }
    );
    let c: Command =
        serde_json::from_str(r#"{"cmd":"power","watch":true,"window_s":3600}"#).unwrap();
    assert_eq!(
        c,
        Command::Power {
            watch: true,
            window_s: Some(3600)
        }
    );
    let c: Command = serde_json::from_str(r#"{"cmd":"power_clear"}"#).unwrap();
    assert_eq!(c, Command::PowerClear);
}

#[test]
fn removed_commands_are_rejected() {
    for raw in [
        r#"{"cmd":"boost"}"#,
        r#"{"cmd":"dnd_access","granted":true}"#,
        r#"{"cmd":"ultra","on":true}"#,
        r#"{"cmd":"seed","pkg":"com.x"}"#,
    ] {
        assert!(serde_json::from_str::<Command>(raw).is_err(), "{raw}");
    }
}

#[test]
fn unknown_command_is_rejected_not_panicked() {
    let r = serde_json::from_str::<Command>(r#"{"cmd":"nope"}"#);
    assert!(r.is_err());
}

#[test]
fn events_serialize_with_tag_and_skip_none() {
    let ev = Event::Decision {
        trigger: "event".into(),
        action: "apply".into(),
        profile: Some("game".into()),
        reason: None,
    };
    let s = serde_json::to_string(&ev).unwrap();
    assert!(s.contains(r#""event":"decision""#));
    assert!(s.contains(r#""trigger":"event""#));
    assert!(!s.contains("reason")); // None is skipped
}

#[test]
fn snapshot_default_is_all_off() {
    let s = Snapshot::default();
    assert!(!s.screen_on);
    assert!(s.active.is_none());
}
