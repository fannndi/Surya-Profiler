//! Thermal takeover unit tests.

use super::io::config_file_allowed;
use super::map::{build_map_at, map_contains_owned_at, render_map};
use super::profile::{BALANCE_TXT, GAME_TXT, POWERSAVE_TXT};
use super::*;

use std::time::{SystemTime, UNIX_EPOCH};

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "surya-thermal-{tag}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn write_vendor_map(root: &Path, body: &str) {
    let p = root.join("vendor/etc/thermal-map.conf");
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, crypt::encrypt(body.as_bytes())).unwrap();
}

#[test]
fn profile_indices_are_private_and_distinct() {
    let all = ThermalProfile::all();
    let mut seen = Vec::new();
    for profile in all {
        assert!((60..=63).contains(&profile.sconfig()));
        assert!(profile.sconfig() <= 64, "mi_thermald rejects map IDs > 64");
        assert!(!seen.contains(&profile.sconfig()));
        seen.push(profile.sconfig());
    }
    assert_eq!(
        ThermalProfile::from_id("game").map(|p| p.sconfig()),
        Some(61)
    );
    assert_eq!(ThermalProfile::from_id("boost"), None);
}

#[test]
fn custom_indices_do_not_overlap_miui_scenarios() {
    // MIUI base IDs 0–25 and every offset combination are not ours.
    for raw in (0..=25).chain([
        30, 100, 121, 130, 151, 200, 230, 300, 330, 500, 521, 600, 620,
    ]) {
        for custom in CUSTOM_INDICES {
            assert_ne!(raw, custom, "MIUI {raw} collides with custom {custom}");
        }
    }
    // Also verify the decoder's yield/default classification at 60–63.
    for custom in CUSTOM_INDICES {
        assert!(!thermal_scenario::decode(custom).known);
        assert!(!thermal_scenario::is_yield(custom));
    }
}

#[test]
fn map_keeps_every_stock_entry_and_adds_four() {
    let root = tmp("map-fallback");
    let entries = parse_map(&build_map_at(&root).unwrap().text).unwrap();
    for id in [0, 1, 2, 8, 9, 10, 11, 12, 13, 14, 15, 16] {
        assert!(entries.iter().any(|(v, _)| *v == id), "stock {id}");
    }
    for id in CUSTOM_INDICES {
        assert!(entries.iter().any(|(v, _)| *v == id), "custom {id}");
    }
    for id in LEGACY_INDICES {
        assert!(!entries.iter().any(|(v, _)| *v == id), "legacy {id}");
    }
    assert!(entries.iter().any(|(v, _)| *v == 17));
    assert!(entries.iter().any(|(v, _)| *v == 18));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn map_parser_rejects_bad_lines() {
    assert!(parse_map("[70:thermal-balance.conf]").is_ok());
    assert!(parse_map("70:thermal-balance.conf").is_err());
    assert!(parse_map("[70:]").is_err());
    assert!(parse_map("[x:thermal-balance.conf]").is_err());
}

#[test]
fn map_renderer_round_trips() {
    let text = "[0:thermal-normal.conf]\n[70:thermal-balance.conf]\n";
    let entries = parse_map(text).unwrap();
    assert_eq!(render_map(&entries), text);
}

#[test]
fn custom_ss_uses_direct_sensor_and_floor_aware_targets() {
    // MIUI's VIRTUAL-SENSOR is rejected on this boot; the SS/GPU steps
    // must read nvm-therm-adc directly. Big-cluster steps stay at or above
    // the 1.8432 GHz screen-on floor.
    for profile in ThermalProfile::all() {
        let text = profile.plaintext();
        assert!(!text.contains("VIRTUAL-SENSOR"), "{}", profile.id());
        assert!(text.contains("sensor\t\tnvm-therm-adc"), "{}", profile.id());
    }
    assert!(GAME_TXT.contains("target\t\t2169600\t1939200"));
    assert!(BALANCE_TXT.contains("target\t\t2169600\t1939200\t1843200"));
    assert!(POWERSAVE_TXT.contains("target\t\t1939200\t1843200"));
}

#[test]
fn custom_configs_keep_the_stock_charging_ladder() {
    // We never add charge controls; every profile carries the stock
    // MONITOR-BAT ladder unchanged.
    let stock = "\
trig\t\t38000\t39000\t40000\t41000\t42000\t43000\t44000\t45000
clr\t\t37000\t38000\t39000\t40000\t41000\t42000\t43000\t44000
target\t\t201\t602\t907\t1312\t1312\t1313\t1414\t1515";
    for profile in ThermalProfile::all() {
        assert!(profile.plaintext().contains(stock), "{}", profile.id());
    }
}

#[test]
fn custom_configs_keep_the_stock_safety_net() {
    // Battery/BCL/temp_state and CCC are the MIUI safety net. Keep them
    // byte-for-byte identical to the vendor normal config.
    for profile in ThermalProfile::all() {
        let text = profile.plaintext();
        assert!(
            text.contains("[MONITOR-TEMP_STATE]") && text.contains("target\t\t4"),
            "{}",
            profile.id()
        );
        assert!(
            text.contains("[MONITOR-BCL]") && text.contains("target\t\t1+1"),
            "{}",
            profile.id()
        );
        assert!(text.contains("CCC_CTRL"), "{}", profile.id());
    }
}

#[test]
fn build_map_reads_the_vendor_blob_and_preserves_stock_entries() {
    let root = tmp("vendor-map");
    write_vendor_map(
        &root,
        "[0:thermal-normal.conf]\n[8:thermal-phone.conf]\n[9:thermal-tgame.conf]\n",
    );
    let built = build_map_at(&root).unwrap();
    assert_eq!(built.source, "vendor");
    let entries = parse_map(&built.text).unwrap();
    for id in [0, 8, 9, 17, 18, 60, 61, 62, 63] {
        assert!(entries.iter().any(|(v, _)| *v == id), "missing {id}");
    }
    for id in LEGACY_INDICES {
        assert!(!entries.iter().any(|(v, _)| *v == id), "legacy {id}");
    }
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn build_map_preserves_an_existing_4k_entry() {
    let root = tmp("vendor-4k");
    write_vendor_map(
        &root,
        "[0:thermal-normal.conf]\n[17:thermal-4k-custom.conf]\n",
    );
    let built = build_map_at(&root).unwrap();
    let entries = parse_map(&built.text).unwrap();
    let four_k: Vec<&String> = entries
        .iter()
        .filter(|(id, _)| *id == 17)
        .map(|(_, name)| name)
        .collect();
    assert_eq!(four_k, vec!["thermal-4k-custom.conf"]);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn build_map_falls_back_when_vendor_blob_is_missing() {
    let root = tmp("vendor-missing");
    let built = build_map_at(&root).unwrap();
    assert_eq!(built.source, "fallback");
    let entries = parse_map(&built.text).unwrap();
    assert!(entries.iter().any(|(v, _)| *v == 60));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn build_map_refuses_a_future_rom_that_uses_our_indices() {
    let root = tmp("vendor-collision");
    write_vendor_map(&root, "[60:some-other.conf]\n");
    let err = build_map_at(&root).unwrap_err();
    assert!(err.contains("private indices"), "{err}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn owned_map_detection_accepts_legacy_and_new_indices() {
    let root = tmp("owned-map");
    let p = root.join("thermal-map.conf");
    fs::write(
        &p,
        crypt::encrypt(b"[0:thermal-normal.conf]\n[20:thermal-balance.conf]\n"),
    )
    .unwrap();
    assert!(map_contains_owned_at(&p), "v0.4 legacy map is ours");
    fs::write(
        &p,
        crypt::encrypt(b"[0:thermal-normal.conf]\n[60:thermal-balance.conf]\n"),
    )
    .unwrap();
    assert!(map_contains_owned_at(&p), "v0.5 map is ours");
    fs::write(
        &p,
        crypt::encrypt(b"[0:thermal-normal.conf]\n[70:thermal-balance.conf]\n"),
    )
    .unwrap();
    assert!(map_contains_owned_at(&p), "v0.5 preview map is ours");
    fs::write(
        &p,
        crypt::encrypt(b"[0:thermal-normal.conf]\n[8:thermal-phone.conf]\n"),
    )
    .unwrap();
    assert!(!map_contains_owned_at(&p), "vendor map is not ours");
    let _ = fs::remove_dir_all(&root);
}

#[cfg(test)]
mod guard_tests {
    use super::*;

    #[test]
    fn thermal_config_allowlist_is_exact() {
        for p in ThermalProfile::all() {
            assert!(config_file_allowed(p.config_name()), "{}", p.config_name());
        }
        assert!(config_file_allowed(MARKER_NAME));
        assert!(config_file_allowed("thermal-map.conf"));
        assert!(!config_file_allowed("thermal-normal.conf"));
        assert!(!config_file_allowed("../../vendor/etc/thermal-map.conf"));
        assert!(!config_file_allowed("thermal-balance.conf.tmp"));
    }
}
