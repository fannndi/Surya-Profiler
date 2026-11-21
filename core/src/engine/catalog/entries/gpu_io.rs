//! GPU / block I/O catalog entries.

use crate::engine::catalog::{Entry, Kind, Tier};

pub(crate) fn entries() -> Vec<Entry> {
    use Kind::*;
    use Tier::*;
    vec![
        // --- GPU (Adreno 618, 7 pwrlevels 0..6) ---
        e!(
            "gpu.governor",
            "/sys/class/kgsl/kgsl-3d0/devfreq/governor",
            Baseline,
            GpuGov,
            ""
        ),
        e!(
            "gpu.max_pwrlevel",
            "/sys/class/kgsl/kgsl-3d0/max_pwrlevel",
            Baseline,
            PwrLevel,
            ""
        ),
        e!(
            "gpu.min_pwrlevel",
            "/sys/class/kgsl/kgsl-3d0/min_pwrlevel",
            Baseline,
            PwrLevel,
            ""
        ),
        e!(
            "gpu.default_pwrlevel",
            "/sys/class/kgsl/kgsl-3d0/default_pwrlevel",
            Baseline,
            PwrLevel,
            ""
        ),
        // --- block I/O (UFS sda, single-queue: noop/deadline/cfq) ---
        e!(
            "io.scheduler",
            "/sys/block/sda/queue/scheduler",
            Baseline,
            IoSched,
            ""
        ),
        e!(
            "io.nr_requests",
            "/sys/block/sda/queue/nr_requests",
            Free,
            Int,
            ""
        ),
        e!(
            "io.nomerges",
            "/sys/block/sda/queue/nomerges",
            Free,
            Int,
            ""
        ),
        e!(
            "io.rq_affinity",
            "/sys/block/sda/queue/rq_affinity",
            Free,
            Int,
            ""
        ),
        e!("io.iostats", "/sys/block/sda/queue/iostats", Free, Int, ""),
        // --- v0.4: per-scheduler I/O latency tunables (cfq-iosched.c) ---
        // NOT written by post_boot or any perf XML (verified in ROM v12.0.9 +
        // strings of libqti-perfd/netd) -> Free. Per-CFQ knobs follow the
        // same lifecycle as schedutil tunables: their directory only exists
        // while cfq is the active scheduler (kernel/block/cfq-iosched.c
        // creates it in cfq_init_queue) — handled by the same pass-2 re-plan
        // in apply (see main.rs).
        ei!(
            "io.cfq.quantum",
            "/sys/block/sda/queue/iosched/quantum",
            Free,
            1,
            64
        ),
        ei!(
            "io.cfq.fifo_expire_sync",
            "/sys/block/sda/queue/iosched/fifo_expire_sync",
            Free,
            0,
            100000
        ),
        ei!(
            "io.cfq.fifo_expire_async",
            "/sys/block/sda/queue/iosched/fifo_expire_async",
            Free,
            0,
            10000
        ),
        ei!(
            "io.cfq.back_seek_max",
            "/sys/block/sda/queue/iosched/back_seek_max",
            Free,
            0,
            1000000
        ),
        ei!(
            "io.cfq.slice_sync",
            "/sys/block/sda/queue/iosched/slice_sync",
            Free,
            1,
            500
        ),
        ei!(
            "io.cfq.slice_async",
            "/sys/block/sda/queue/iosched/slice_async",
            Free,
            1,
            500
        ),
        ei!(
            "io.cfq.slice_idle_us",
            "/sys/block/sda/queue/iosched/slice_idle_us",
            Free,
            0,
            100000
        ),
        ei!(
            "io.cfq.target_latency_us",
            "/sys/block/sda/queue/iosched/target_latency_us",
            Free,
            1,
            1000000
        ),
        ei!(
            "io.cfq.group_idle",
            "/sys/block/sda/queue/iosched/group_idle",
            Free,
            0,
            1
        ),
        ei!(
            "io.cfq.low_latency",
            "/sys/block/sda/queue/iosched/low_latency",
            Free,
            0,
            1
        ),
        // --- v0.7 additions (evidence: pulled ROM V12.0.9 + kernel source) ---
        // read_ahead_kb: init.qcom.rc writes only dm-*/mmcblk* (dm-* at boot,
        // reset to 512 after boot); sda (userdata, f2fs) is never written by
        // any script/XML, and the patched kernel default is VM_MAX_READAHEAD
        // = 512 (include/linux/mm.h:2378, Xiaomi patch) — Free.
        // queue_ra_store sets bdi->ra_pages (block/blk-sysfs.c:101).
        er!(
            "io.read_ahead_kb",
            "/sys/block/sda/queue/read_ahead_kb",
            Free,
            Int,
            "",
            0,
            8192
        ),
        // L3-latency devfreq floors: post_boot writes cpu0/cpu6 l3-lat
        // min/max per SKU (the SA6150 block sets min 940800000); the perf
        // HAL does not touch them at runtime -> Baseline. Values must be in
        // the node's available_frequencies (device-verified list).
        er!(
            "devfreq.cpu0_l3_lat.min_freq",
            "/sys/class/devfreq/soc:qcom,cpu0-cpu-l3-lat/min_freq",
            Baseline,
            Int,
            "",
            0,
            2000000000
        ),
        er!(
            "devfreq.cpu6_l3_lat.min_freq",
            "/sys/class/devfreq/soc:qcom,cpu6-cpu-l3-lat/min_freq",
            Baseline,
            Int,
            "",
            0,
            2000000000
        ),
        // R&D charger-input suspend: init.target.rc + init.miui.rc chmod
        // 0777 + chown system; MIUI's own mishow.sh writes it and hvdcp_opti
        // only reads it (therm-balance monitor). Kernel: vote 0 mA on usb_icl
        // + suspend dc (USER_VOTER) -> input cut, the device runs on battery.
        // Device-verified 2026-10-08. This is the ONLY charge node the app may
        // write; battery_charging_enabled stays framework-owned.
        er!(
            "charge.input_suspend",
            "/sys/class/power_supply/battery/input_suspend",
            Baseline,
            Int,
            "",
            0,
            1
        ),
    ]
}
