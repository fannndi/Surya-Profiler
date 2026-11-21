//! Kernel scheduler / sysctl catalog entries.

use crate::engine::catalog::{Entry, Kind, Tier};

pub(crate) fn entries() -> Vec<Entry> {
    use Kind::*;
    use Tier::*;
    vec![
        // --- scheduler sysctls (NOT written by the moorea post_boot block) ---
        // WALT damping for the Predictive Load floor (only active while pl=1:
        // predicted load is scaled by TARGET_LOAD). BASELINE: post_boot writes
        // it (1) in the lito/atoll arms — our executed moorea arm (soc
        // 365/366) leaves the kernel default 0, perf XML does not mention it,
        // and a live watch during a game boost stayed 0; device
        // write/readback verified 2026-10-09. Baseline keeps the owner-map
        // audit's static rule ("written by some ROM script") satisfied.
        e!(
            "kernel.sched_conservative_pl",
            "/proc/sys/kernel/sched_conservative_pl",
            Baseline,
            Int,
            ""
        ),
        e!(
            "kernel.sched_latency_ns",
            "/proc/sys/kernel/sched_latency_ns",
            Free,
            Int,
            ""
        ),
        e!(
            "kernel.sched_min_granularity_ns",
            "/proc/sys/kernel/sched_min_granularity_ns",
            Free,
            Int,
            ""
        ),
        e!(
            "kernel.sched_wakeup_granularity_ns",
            "/proc/sys/kernel/sched_wakeup_granularity_ns",
            Free,
            Int,
            ""
        ),
        // perf HAL boost writes this at runtime (commonresourceconfigs.xml
        // Opcode 0x2) — baseline coexist, verified by tools/owner-map-audit.sh
        e!(
            "kernel.sched_migration_cost_ns",
            "/proc/sys/kernel/sched_migration_cost_ns",
            Baseline,
            Int,
            ""
        ),
        e!(
            "kernel.sched_nr_migrate",
            "/proc/sys/kernel/sched_nr_migrate",
            Free,
            Int,
            ""
        ),
        er!(
            "kernel.sched_autogroup_enabled",
            "/proc/sys/kernel/sched_autogroup_enabled",
            Free,
            Int,
            "",
            0,
            1
        ),
        er!(
            "kernel.sched_child_runs_first",
            "/proc/sys/kernel/sched_child_runs_first",
            Free,
            Int,
            "",
            0,
            1
        ),
        // boot-written by the moorea post_boot block -> baseline family
        er!(
            "kernel.sched_walt_rotate_big_tasks",
            "/proc/sys/kernel/sched_walt_rotate_big_tasks",
            Baseline,
            Int,
            "",
            0,
            1
        ),
        er!(
            "kernel.sched_little_cluster_coloc_fmin_khz",
            "/proc/sys/kernel/sched_little_cluster_coloc_fmin_khz",
            Baseline,
            Int,
            "",
            0,
            2000000
        ),
        // migration policy (boot-written by post_boot only — baseline)
        e!(
            "kernel.sched_upmigrate",
            "/proc/sys/kernel/sched_upmigrate",
            Baseline,
            Int,
            ""
        ),
        e!(
            "kernel.sched_downmigrate",
            "/proc/sys/kernel/sched_downmigrate",
            Baseline,
            Int,
            ""
        ),
        e!(
            "kernel.sched_group_upmigrate",
            "/proc/sys/kernel/sched_group_upmigrate",
            Baseline,
            Int,
            ""
        ),
        e!(
            "kernel.sched_group_downmigrate",
            "/proc/sys/kernel/sched_group_downmigrate",
            Baseline,
            Int,
            ""
        ),
        // --- v0.4 additions (igeh evidence: kernel/sysctl.c bounds + writer audit) ---
        // colocation v3 block in post_boot (51/35 default; game branch writes
        // 0/0) AND runtime per-opcode by the perf HAL (commonresourceconfigs
        // minors 0x25/0x26) -> Baseline. Bounds 0..=1000
        // (extra1=&zero, extra2=&one_thousand, kernel/sysctl.c:389).
        er!(
            "kernel.sched_min_task_util_for_boost",
            "/proc/sys/kernel/sched_min_task_util_for_boost",
            Baseline,
            Int,
            "",
            0,
            1000
        ),
        er!(
            "kernel.sched_min_task_util_for_colocation",
            "/proc/sys/kernel/sched_min_task_util_for_colocation",
            Baseline,
            Int,
            "",
            0,
            1000
        ),
        // sync-hint group (XML minor 0x28, runtime) — kernel uses plain
        // proc_dointvec (any count works), we constrain to the 0/1 domain.
        er!(
            "kernel.sched_sync_hint_enable",
            "/proc/sys/kernel/sched_sync_hint_enable",
            Baseline,
            Int,
            "",
            0,
            1
        ),
        // XML minor 0x27 (runtime). Kernel bound: extra1=&two (>=2),
        // extra2=&one_thousand (kernel/sched/walt.c:926 default 1000).
        er!(
            "kernel.sched_many_wakeup_threshold",
            "/proc/sys/kernel/sched_many_wakeup_threshold",
            Baseline,
            Int,
            "",
            2,
            1000
        ),
        // sched_time_avg: proc_dointvec_minmax extra1=&one (>=1 dd)
        // (kernel/sysctl.c; core.c:76 const_debug default 1000ms).
        // Free: no ROM/script writer found in post_boot/perf XML.
        er!(
            "kernel.sched_time_avg_ms",
            "/proc/sys/kernel/sched_time_avg_ms",
            Free,
            Int,
            "",
            1,
            10000
        ),
        // timer_migration_handler, extra1=&zero extra2=&one (kernel/sysctl.c
        // 1357..1363 and the live node accepts the 0..1 domain); post_boot
        // writes "power_aware_timer_migration" which does NOT exist in this
        // kernel (dead write, documented in ROM-HARMONY.md) — timer_migration
        // itself has no ROM writer -> Free.
        er!(
            "kernel.timer_migration",
            "/proc/sys/kernel/timer_migration",
            Free,
            Int,
            "",
            0,
            1
        ),
        // sched_rr_handler (kernel/sched/rt.c:2912): writing <=0 RESETS the
        // timeslice to the default — keep a sane band so a profile never
        // means "reset".
        er!(
            "kernel.sched_rr_timeslice_ms",
            "/proc/sys/kernel/sched_rr_timeslice_ms",
            Free,
            Int,
            "",
            1,
            1000
        ),
    ]
}
