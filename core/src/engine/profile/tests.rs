//! profile unit tests (split from profile.rs).

use super::*;
use crate::engine::catalog;

#[test]
fn embedded_profiles_parse_and_validate_shape() {
    let json = include_str!("../../../profiles.json");
    let file = parse_profiles(json).expect("bundled profiles.json must parse");
    assert_eq!(file.profiles.len(), 4);
    let ids: Vec<&str> = file.profiles.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, vec!["powersave", "balance", "game", "sleep"]);
    // every key of every profile must exist in the catalog
    for prof in &file.profiles {
        for key in prof.params.keys() {
            assert!(
                catalog::find(key).is_some(),
                "{}: unknown key {key}",
                prof.id
            );
        }
    }
}

#[test]
fn embedded_profiles_do_not_fight_the_network_stack() {
    // ConnectivityService + netd rewrite tcp buffers from the carrier's
    // LinkProperties.TcpBufferSizes on network re-apply (observed across
    // display-off cycles, 2026-10-07). The network stack owns these nodes;
    // profiles must not ship values for them.
    let json = include_str!("../../../profiles.json");
    let file = parse_profiles(json).expect("parse");
    for p in &file.profiles {
        for k in ["net.tcp_rmem", "net.tcp_wmem"] {
            assert!(
                !p.params.contains_key(k),
                "profile {} must not tune {k} (ConnectivityService+netd own it)",
                p.id
            );
        }
    }
}
