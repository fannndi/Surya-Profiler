//! Plan builder tests.

use super::*;

use crate::engine::testutil::{empty_probe, entry_state, fake_probe, profile};

#[test]
fn forbidden_key_voids_plan() {
    let p = profile(&[("vm.swappiness", "50")]);
    let plan = build_plan(&p, &fake_probe());
    assert!(!plan.ok);
    assert!(plan.errors[0].contains("FORBIDDEN"));
}

#[test]
fn unknown_key_voids_plan() {
    let p = profile(&[("gpu.turbo_mode", "1")]);
    let plan = build_plan(&p, &fake_probe());
    assert!(!plan.ok);
    assert!(plan.errors[0].contains("unknown key"));
}

#[test]
fn freq_clamps_to_real_opp_by_direction() {
    // A cap must round DOWN: never exceed the requested maximum.
    let p = profile(&[("policy0.scaling_max_freq", "1700000")]);
    let plan = build_plan(&p, &fake_probe());
    assert_eq!(plan.ops[0].resolved, "1612800");
    assert_eq!(plan.ops[0].status, OpStatus::Ok);

    // A floor must round UP: never go below the requested minimum.
    let mut probe = fake_probe();
    probe.entries.insert(
        "policy0.scaling_min_freq".into(),
        entry_state(true, "576000"),
    );
    let p = profile(&[("policy0.scaling_min_freq", "700000")]);
    let plan = build_plan(&p, &probe);
    assert_eq!(plan.ops[0].resolved, "768000");
    assert_eq!(plan.ops[0].status, OpStatus::Ok);

    // hispeed is a plain frequency: nearest OPP.
    let mut probe = fake_probe();
    probe.entries.insert(
        "policy0.schedutil.hispeed_freq".into(),
        entry_state(true, "1612800"),
    );
    let p = profile(&[("policy0.schedutil.hispeed_freq", "1700000")]);
    let plan = build_plan(&p, &probe);
    assert_eq!(plan.ops[0].resolved, "1708800");
}

#[test]
fn min_cpus_uses_the_max_cpus_this_plan_writes() {
    let mut probe = fake_probe();
    // Live cap is the sleep profile's 2; the new profile raises it to 6.
    probe.options.core_ctl_max.insert("policy0".into(), 2);
    probe
        .entries
        .insert("policy0.core_ctl.max_cpus".into(), entry_state(true, "2"));
    let p = profile(&[
        ("policy0.core_ctl.max_cpus", "6"),
        ("policy0.core_ctl.min_cpus", "6"),
    ]);
    let plan = build_plan(&p, &probe);
    let min = plan
        .ops
        .iter()
        .find(|o| o.key == "policy0.core_ctl.min_cpus")
        .unwrap();
    assert_eq!(min.resolved, "6", "must not clamp to the stale live max");
}

#[test]
fn freq_max_below_miui14_floor_is_locked() {
    // MIUI 14 screen-on floor is 1612800 (msm_performance QoS); cpufreq
    // silently clamps a lower cap back to the floor, so planning it as a
    // writable op would produce a permanent read-back mismatch.
    let p = profile(&[("policy0.scaling_max_freq", "1248000")]);
    let plan = build_plan(&p, &fake_probe());
    assert!(matches!(plan.ops[0].status, OpStatus::Locked(_)));
    assert!(
        plan.ops[0].resolved == "1248000",
        "locked ops keep the resolved candidate"
    );

    // At the floor is legal and writable.
    let p = profile(&[("policy0.scaling_max_freq", "1612800")]);
    let plan = build_plan(&p, &fake_probe());
    assert_eq!(plan.ops[0].status, OpStatus::Ok);
}

#[test]
fn exact_reconcile_never_raises_a_stricter_live_freq_max() {
    let p = profile(&[("policy0.scaling_max_freq", "1804800")]);
    let mut probe = fake_probe();
    probe.entries.insert(
        "policy0.scaling_max_freq".into(),
        entry_state(true, "1209600"),
    );
    let plan = build_plan_opt(&p, &probe, true);
    assert_eq!(plan.ops[0].status, OpStatus::Unchanged);

    // A lost cap (live above wanted) is still repaired by forced reconcile.
    probe.entries.insert(
        "policy0.scaling_max_freq".into(),
        entry_state(true, "2169600"),
    );
    let plan = build_plan_opt(&p, &probe, true);
    assert_eq!(plan.ops[0].status, OpStatus::Ok);
}

#[test]
fn governor_not_available_is_locked() {
    let p = profile(&[("policy0.scaling_governor", "performance")]);
    let plan = build_plan(&p, &fake_probe());
    assert!(matches!(plan.ops[0].status, OpStatus::Locked(_)));
}

#[test]
fn unchanged_detected_with_tabs() {
    let p = profile(&[("net.tcp_rmem", "524288 1048576 5505024")]);
    let plan = build_plan(&p, &fake_probe());
    assert_eq!(plan.ops[0].status, OpStatus::Unchanged);
}

#[test]
fn write_order_is_safe() {
    // sysctls first, governor before max before min
    let p = profile(&[
        ("policy0.scaling_min_freq", "768000"),
        ("policy0.scaling_max_freq", "1804800"),
        ("policy0.scaling_governor", "schedutil"),
        ("vm.dirty_ratio", "30"),
    ]);
    let plan = build_plan(&p, &fake_probe());
    let keys: Vec<&str> = plan.ops.iter().map(|o| o.key.as_str()).collect();
    assert_eq!(
        keys,
        vec![
            "vm.dirty_ratio",
            "policy0.scaling_governor",
            "policy0.scaling_max_freq",
            "policy0.scaling_min_freq"
        ]
    );
}

#[test]
fn io_sched_plan_detects_real_switch() {
    // regression: profile wants deadline while live=cfq (bracketed) — the
    // op must be Ok (to write), never Unchanged.
    let p = profile(&[("io.scheduler", "deadline")]);
    let mut probe = fake_probe();
    probe.entries.insert(
        "io.scheduler".into(),
        entry_state(true, "noop deadline [cfq]"),
    );
    let plan = build_plan(&p, &probe);
    assert_eq!(plan.ops[0].status, OpStatus::Ok, "must plan a real write");
}

#[test]
fn stune_boost_range_enforced() {
    let p = profile(&[("stune.top-app.boost", "101")]);
    let plan = build_plan(&p, &fake_probe());
    assert!(
        matches!(plan.ops[0].status, OpStatus::Locked(_)),
        "101 must be locked"
    );

    let p = profile(&[("stune.top-app.boost", "100")]);
    let plan = build_plan(&p, &fake_probe());
    assert_eq!(plan.ops[0].status, OpStatus::Ok);
}

#[test]
fn task_thres_bound_is_cluster_cpu_count() {
    // kernel core_ctl.c store_task_thres: val < num_cpus -> EINVAL
    let p = profile(&[("policy0.core_ctl.task_thres", "4")]);
    let plan = build_plan(&p, &fake_probe());
    assert!(
        matches!(plan.ops[0].status, OpStatus::Locked(_)),
        "4 < 6 must be locked"
    );

    let p = profile(&[("policy0.core_ctl.task_thres", "6")]);
    let plan = build_plan(&p, &fake_probe());
    assert_eq!(plan.ops[0].status, OpStatus::Ok);
}

#[test]
fn min_cpus_preclamps_to_max_cpus() {
    // kernel core_ctl.c store_min_cpus: min(val, max_cpus) — silent clamp
    let p = profile(&[("policy0.core_ctl.min_cpus", "9")]);
    let plan = build_plan(&p, &fake_probe());
    assert_eq!(plan.ops[0].resolved, "6", "pre-clamped to live max_cpus");
    assert_eq!(plan.ops[0].status, OpStatus::Ok);
}

#[test]
fn gpu_pwrlevel_pair_allows_equality() {
    // kgsl_pwrctrl.c:692 clamps level > min_pwrlevel to min; equality OK
    let p = profile(&[("gpu.max_pwrlevel", "4"), ("gpu.min_pwrlevel", "4")]);
    let plan = build_plan(&p, &fake_probe());
    assert!(plan.ok, "max == min must be valid: {:?}", plan.errors);

    let p = profile(&[("gpu.max_pwrlevel", "5"), ("gpu.min_pwrlevel", "3")]);
    let plan = build_plan(&p, &fake_probe());
    assert!(!plan.ok, "max > min must be rejected");
}

#[test]
fn migrate_pair_invariant_rejects_bad_profile() {
    let p = profile(&[
        ("kernel.sched_upmigrate", "60"),
        ("kernel.sched_downmigrate", "80"),
    ]);
    let plan = build_plan(&p, &empty_probe());
    assert!(!plan.ok);
    assert!(
        plan.errors.iter().any(|e| e.contains("must be <")),
        "errors: {:?}",
        plan.errors
    );
}

#[test]
fn group_migrate_pair_invariant_allows_equality_rejects_bad() {
    // sysctl.c registers sched_group_upmigrate with extra1 = downmigrate
    // and sched_group_downmigrate with extra2 = upmigrate — the kernel
    // rejects down > up; equality is legal (post_boot has a 900/900 branch).
    let p = profile(&[
        ("kernel.sched_group_upmigrate", "120"),
        ("kernel.sched_group_downmigrate", "140"),
    ]);
    let plan = build_plan(&p, &empty_probe());
    assert!(!plan.ok);
    assert!(
        plan.errors
            .iter()
            .any(|e| e.contains("must be <= upmigrate")),
        "errors: {:?}",
        plan.errors
    );

    let p = profile(&[
        ("kernel.sched_group_upmigrate", "120"),
        ("kernel.sched_group_downmigrate", "120"),
    ]);
    let plan = build_plan(&p, &empty_probe());
    assert!(plan.ok, "equality must be legal: {:?}", plan.errors);
}
