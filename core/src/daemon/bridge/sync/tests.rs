//! sync unit tests (split from sync.rs).

use super::*;
use crate::daemon::test_util::map;

fn ctx(pairs: &[(&str, &str)], fg: Option<&str>) -> SyncCtx {
    SyncCtx {
        enabled: true,
        last_real: fg.map(str::to_string),
        screen_on: true,
        locked: false,
        dynamic: true,
        app_map: map(pairs),
        multi_window: false,
        live_saver: false,
        profile: None,
        custom_thermal: false,
        rnd_input_suspend: false,
        thread_affinity: false,
        app_downscale: std::collections::BTreeMap::new(),
        display_scale: "off".into(),
    }
}

#[test]
fn saver_gate_requires_mapped_powersave() {
    assert!(saver_want(&ctx(&[("com.p", "powersave")], Some("com.p"))));
    assert!(!saver_want(&ctx(&[("com.g", "game")], Some("com.g"))));
    assert!(!saver_want(&ctx(&[], Some("com.p"))));
    let mut c = ctx(&[("com.p", "powersave")], Some("com.p"));
    c.dynamic = false;
    assert!(!saver_want(&c), "dynamic OFF must stop the saver follow");
}

#[test]
fn saver_gate_requires_visible_unlocked_screen() {
    let mut c = ctx(&[("com.p", "powersave")], Some("com.p"));
    c.screen_on = false;
    assert!(!saver_want(&c));
    let mut c = ctx(&[("com.p", "powersave")], Some("com.p"));
    c.locked = true;
    assert!(!saver_want(&c));
}

#[test]
fn split_screen_disables_the_saver_follow() {
    let mut c = ctx(&[("com.p", "powersave")], Some("com.p"));
    c.multi_window = true;
    assert!(!saver_want(&c), "multi-window forces Balance, not saver");
}
