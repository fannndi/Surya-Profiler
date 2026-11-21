//! VM / net / schedtune / cpuset catalog entries.

use crate::engine::catalog::{Entry, Kind, Tier};

pub(crate) fn entries() -> Vec<Entry> {
    use Kind::*;
    use Tier::*;
    vec![
        // --- vm (swappiness/min_free_kbytes/page-cluster are forbidden) ---
        e!(
            "vm.vfs_cache_pressure",
            "/proc/sys/vm/vfs_cache_pressure",
            Free,
            Int,
            ""
        ),
        e!("vm.dirty_ratio", "/proc/sys/vm/dirty_ratio", Free, Int, ""),
        e!(
            "vm.dirty_background_ratio",
            "/proc/sys/vm/dirty_background_ratio",
            Free,
            Int,
            ""
        ),
        e!(
            "vm.dirty_expire_centisecs",
            "/proc/sys/vm/dirty_expire_centisecs",
            Free,
            Int,
            ""
        ),
        e!(
            "vm.dirty_writeback_centisecs",
            "/proc/sys/vm/dirty_writeback_centisecs",
            Free,
            Int,
            ""
        ),
        e!(
            "vm.stat_interval",
            "/proc/sys/vm/stat_interval",
            Free,
            Int,
            ""
        ),
        // configure_memory_parameters sets 1 at boot for every target
        // (post_boot: "Disable wsf ... using efk"; range 1..1000 per its own
        // comment) -> boot baseline, audit-tool verified.
        er!(
            "vm.watermark_scale_factor",
            "/proc/sys/vm/watermark_scale_factor",
            Baseline,
            Int,
            "",
            1,
            1000
        ),
        e!(
            "vm.extra_free_kbytes",
            "/proc/sys/vm/extra_free_kbytes",
            Free,
            Int,
            ""
        ),
        // --- net (no ROM writer confirmed) ---
        e!(
            "net.tcp_congestion_control",
            "/proc/sys/net/ipv4/tcp_congestion_control",
            Free,
            TcpCc,
            ""
        ),
        // ConnectivityService applies LinkProperties.TcpBufferSizes (carrier
        // config, visible in `dumpsys connectivity`) through netd's
        // setTcpBufferSizes -> rewrites these on every network re-apply.
        // Empirically observed across display-off cycles (2026-10-07,
        // tools/display-off-diff.sh): the network stack owns them, so they are
        // baseline tier and no shipped profile writes them (profile.rs test).
        e!(
            "net.tcp_rmem",
            "/proc/sys/net/ipv4/tcp_rmem",
            Baseline,
            Ints,
            ""
        ),
        e!(
            "net.tcp_wmem",
            "/proc/sys/net/ipv4/tcp_wmem",
            Baseline,
            Ints,
            ""
        ),
        e!(
            "net.tcp_slow_start_after_idle",
            "/proc/sys/net/ipv4/tcp_slow_start_after_idle",
            Free,
            Int,
            ""
        ),
        // Touch input boost floor (kernel cpu-boost.c): on every real input
        // event the little cluster policy min is raised to this per-CPU
        // `cpu:freq` list for `input_boost_ms` (stock: 0:1324800 @120 ms).
        // powersave zeroes it so the min-lock governor is not defeated by
        // touches; other profiles leave it stock. Baseline: post_boot seeds
        // it and the perf HAL saves/restores it (perfd default_values
        // index_113), so a perfd boost release can transiently restore the
        // stock value — the reconcile/events heal that like any Baseline
        // node. Read-back format is `<cpu>:<khz> ...` (Text, verbatim).
        e!(
            "cpu_boost.input_boost_freq",
            "/sys/module/cpu_boost/parameters/input_boost_freq",
            Baseline,
            Text,
            ""
        ),
        e!(
            "net.tcp_mtu_probing",
            "/proc/sys/net/ipv4/tcp_mtu_probing",
            Free,
            Int,
            ""
        ),
        e!(
            "net.tcp_fin_timeout",
            "/proc/sys/net/ipv4/tcp_fin_timeout",
            Free,
            Int,
            ""
        ), // proc_dointvec_jiffies, no minmax
        e!(
            "net.tcp_fastopen",
            "/proc/sys/net/ipv4/tcp_fastopen",
            Free,
            Int,
            ""
        ),
        // workqueue.power_efficient intentionally ABSENT:
        // kernel/workqueue.c:294 module_param(..., 0444) — read-only by kernel
        // on every device; never cataloged, never written.
        // --- schedtune (perf HAL boosts top-app transiently; baseline) ---
        er!(
            "stune.top-app.boost",
            "/dev/stune/top-app/schedtune.boost",
            Baseline,
            Int,
            "",
            0,
            100
        ), // tune.c boost_write: 0..=100
        er!(
            "stune.top-app.prefer_idle",
            "/dev/stune/top-app/schedtune.prefer_idle",
            Baseline,
            Int,
            "",
            0,
            1
        ),
        er!(
            "stune.foreground.boost",
            "/dev/stune/foreground/schedtune.boost",
            Baseline,
            Int,
            "",
            0,
            100
        ),
        er!(
            "stune.foreground.prefer_idle",
            "/dev/stune/foreground/schedtune.prefer_idle",
            Baseline,
            Int,
            "",
            0,
            1
        ),
        // --- cpusets (game/gamelite/vr are PowerKeeper-owned: forbidden) ---
        e!(
            "cpuset.background.cpus",
            "/dev/cpuset/background/cpus",
            Baseline,
            Mask,
            ""
        ),
        e!(
            "cpuset.system-background.cpus",
            "/dev/cpuset/system-background/cpus",
            Baseline,
            Mask,
            ""
        ),
        e!(
            "cpuset.foreground.cpus",
            "/dev/cpuset/foreground/cpus",
            Baseline,
            Mask,
            ""
        ),
        e!(
            "cpuset.top-app.cpus",
            "/dev/cpuset/top-app/cpus",
            Baseline,
            Mask,
            ""
        ),
    ]
}
