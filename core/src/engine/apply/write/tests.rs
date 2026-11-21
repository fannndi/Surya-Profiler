//! Apply writer tests.

use super::*;

use super::super::SnapValue;
use crate::engine::plan::build_plan;
use crate::engine::profile::Profile;

#[test]
fn migrate_pair_write_order_is_kernel_safe() {
    use crate::engine::plan::OpStatus;
    use crate::engine::profile::Profile;
    // plan: up=60 down=50 (down-first by rank, up > cur_down?)
    let mut params = std::collections::BTreeMap::new();
    params.insert("kernel.sched_upmigrate".to_string(), "60".to_string());
    params.insert("kernel.sched_downmigrate".to_string(), "50".to_string());
    let prof = Profile {
        id: "t".into(),
        label: "T".into(),
        desc: String::new(),
        short: String::new(),
        hidden: false,
        params,
    };

    let mut probe = crate::engine::probe::ProbeData {
        device: Default::default(),
        entries: Default::default(),
        options: Default::default(),
        framework: Default::default(),
    };
    // stock: up=71 down=65 -> want_up(60) <= cur_down(65): down first
    probe.entries.insert(
        "kernel.sched_upmigrate".into(),
        crate::engine::probe::EntryState {
            exists: true,
            value: Some("71".into()),
        },
    );
    probe.entries.insert(
        "kernel.sched_downmigrate".into(),
        crate::engine::probe::EntryState {
            exists: true,
            value: Some("65".into()),
        },
    );
    let plan = build_plan(&prof, &probe);
    assert!(plan.ok, "errors: {:?}", plan.errors);
    let ord = plan_ops_ordered(&plan, &probe);
    let keys: Vec<&str> = ord.iter().map(|o| o.key.as_str()).collect();
    let d = keys
        .iter()
        .position(|k| *k == "kernel.sched_downmigrate")
        .unwrap();
    let u = keys
        .iter()
        .position(|k| *k == "kernel.sched_upmigrate")
        .unwrap();
    assert!(d < u, "stock->game must write downmigrate first: {keys:?}");

    // reverse case: cur down=40, want up=60 -> up first
    let mut probe2 = probe.clone();
    probe2
        .entries
        .get_mut("kernel.sched_downmigrate")
        .unwrap()
        .value = Some("40".into());
    let plan2 = build_plan(&prof, &probe2);
    let ord2 = plan_ops_ordered(&plan2, &probe2);
    let keys2: Vec<&str> = ord2.iter().map(|o| o.key.as_str()).collect();
    let d2 = keys2
        .iter()
        .position(|k| *k == "kernel.sched_downmigrate")
        .unwrap();
    let u2 = keys2
        .iter()
        .position(|k| *k == "kernel.sched_upmigrate")
        .unwrap();
    assert!(u2 < d2, "up-first when want_up > cur_down: {keys2:?}");
    let _ = OpStatus::Ok;
}

#[test]
fn group_migrate_pair_write_order_is_kernel_safe() {
    // stock up=100 / down=85 -> game wants 140/120: want_up(140) >
    // cur_down(85) => write up first (140 >= 85), then down (120 <= 140).
    let mut params = std::collections::BTreeMap::new();
    params.insert(
        "kernel.sched_group_upmigrate".to_string(),
        "140".to_string(),
    );
    params.insert(
        "kernel.sched_group_downmigrate".to_string(),
        "120".to_string(),
    );
    let prof = Profile {
        id: "t".into(),
        label: "T".into(),
        desc: String::new(),
        short: String::new(),
        hidden: false,
        params,
    };
    let mut probe = crate::engine::probe::ProbeData {
        device: Default::default(),
        entries: Default::default(),
        options: Default::default(),
        framework: Default::default(),
    };
    probe.entries.insert(
        "kernel.sched_group_upmigrate".into(),
        crate::engine::probe::EntryState {
            exists: true,
            value: Some("100".into()),
        },
    );
    probe.entries.insert(
        "kernel.sched_group_downmigrate".into(),
        crate::engine::probe::EntryState {
            exists: true,
            value: Some("85".into()),
        },
    );
    let plan = build_plan(&prof, &probe);
    assert!(plan.ok, "errors: {:?}", plan.errors);
    let ord = plan_ops_ordered(&plan, &probe);
    let keys: Vec<&str> = ord.iter().map(|o| o.key.as_str()).collect();
    let u = keys
        .iter()
        .position(|k| *k == "kernel.sched_group_upmigrate")
        .unwrap();
    let d = keys
        .iter()
        .position(|k| *k == "kernel.sched_group_downmigrate")
        .unwrap();
    assert!(u < d, "up-first when want_up > cur_down: {keys:?}");
}

#[test]
fn merged_profile_injects_stock_for_missing_keys() {
    use std::collections::BTreeMap;
    let base = Profile {
        id: "t".into(),
        label: "T".into(),
        desc: String::new(),
        short: String::new(),
        hidden: false,
        params: [("kernel.sched_nr_migrate".to_string(), "16".to_string())]
            .into_iter()
            .collect(),
    };
    let mut stock = BTreeMap::new();
    stock.insert(
        "policy6.core_ctl.min_cpus".to_string(),
        SnapValue {
            path: "/sys/devices/system/cpu/cpu6/core_ctl/min_cpus".into(),
            value: "1".into(),
        },
    );
    let merged = merged_profile(&base, Some(stock));
    assert_eq!(merged.params.get("kernel.sched_nr_migrate").unwrap(), "16");
    assert_eq!(merged.params.get("policy6.core_ctl.min_cpus").unwrap(), "1");
}

#[test]
fn merged_profile_without_stock_is_identity() {
    let base = Profile {
        id: "t".into(),
        label: "T".into(),
        desc: String::new(),
        short: String::new(),
        hidden: false,
        params: [("kernel.sched_nr_migrate".to_string(), "16".to_string())]
            .into_iter()
            .collect(),
    };
    let merged = merged_profile(&base, None);
    assert_eq!(merged.params, base.params);
}

#[test]
fn guard_blocks_restore_writes_to_forbidden_paths() {
    // defense in depth: even a tampered snapshot cannot write framework nodes
    assert!(write_one("/sys/module/msm_performance/parameters/cpu_max_freq", "0").is_err());
    assert!(write_one("/proc/sys/vm/swappiness", "50").is_err());
}
