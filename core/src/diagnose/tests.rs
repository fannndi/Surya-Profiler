//! Doctor parser tests.

use super::battery::*;

#[test]
fn parses_stopped_battery_service() {
    let d = parse_battery_dump(
        "Current Battery Service state:\n  (UPDATES STOPPED -- use 'reset' to restart)\n  level: 91\n  scale: 100\n",
    );
    assert!(d.updates_stopped);
    assert_eq!(d.level, Some(91));
    assert_eq!(d.scale, Some(100));
}

#[test]
fn parses_live_battery_service() {
    let d = parse_battery_dump("  level: 99\n  scale: 100\n  status: 2\n");
    assert!(!d.updates_stopped);
    assert_eq!(d.level, Some(99));
    assert_eq!(d.scale, Some(100));
}

#[test]
fn pct_rounding_is_stable() {
    let d = parse_battery_dump("  level: 91\n  scale: 100\n");
    let pct =
        (d.level.unwrap() as u32 * 100 + d.scale.unwrap() as u32 / 2) / d.scale.unwrap() as u32;
    assert_eq!(pct, 91);
}
