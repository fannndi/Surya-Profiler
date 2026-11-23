//! catalog unit tests (split from mod.rs).

use super::*;

#[test]
fn catalog_has_no_forbidden_paths() {
    for e in catalog() {
        guard_path(e.path).unwrap_or_else(|err| panic!("{}: {err}", e.key));
    }
}

#[test]
fn catalog_keys_are_unique() {
    let mut keys: Vec<&str> = catalog().iter().map(|e| e.key).collect();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), catalog().len());
}

#[test]
fn forbidden_paths_rejected() {
    assert!(guard_path("/sys/module/msm_performance/parameters/cpu_max_freq").is_err());
    assert!(guard_path("/sys/class/thermal/thermal_message/sconfig").is_err());
    assert!(guard_path("/proc/sys/vm/swappiness").is_err());
    assert!(guard_path("/dev/cpuset/game/cpus").is_err());
    assert!(guard_path("/sys/class/power_supply/battery/sw_jeita_enabled").is_err());
    assert!(guard_path("/sys/block/zram0/disksize").is_err());
    // trailing-slash prefixes must still reject descendants
    assert!(
        guard_path("/sys/devices/system/cpu/cpu0/cpuidle/state0/disable").is_err(),
        "cpuidle descendant must be forbidden"
    );
    assert!(
        guard_path("/sys/devices/system/cpu/cpuidle/state1/disable").is_err(),
        "cluster cpuidle descendant must be forbidden"
    );
    // the audited cpu_boost touch-floor param is allowed now; its
    // siblings (powerkey boost, sched boost) stay under the prefix
    assert!(guard_path("/sys/module/cpu_boost/parameters/input_boost_freq").is_ok());
    assert!(guard_path("/sys/module/cpu_boost/parameters/powerkey_input_boost_freq").is_err());
    assert!(guard_path("/sys/module/cpu_boost/parameters/input_boost_ms").is_err());
    // 2026-11-23 reference audit: sched-boost family variants and the
    // vendor-owned UFS clock-scaling nodes (evidence:
    // docs/HISTORY.md).
    assert!(guard_path("/proc/sys/kernel/sched_boost_top_app").is_err());
    assert!(guard_path("/proc/sys/kernel/sched_prefer_sync_wakee_to_waker").is_err());
    assert!(guard_path("/sys/devices/platform/soc/1d84000.ufshc/clkscale_enable").is_err());
    assert!(guard_path("/sys/devices/platform/soc/1d84000.ufshc/clkgate_enable").is_err());
    assert!(guard_path("/sys/devices/platform/soc/1d84000.ufshc/hibern8_on_idle_enable").is_err());
    // REA 2026-11-23: perf HAL uclamp hint surface (commonresourceconfigs).
    assert!(guard_path("/dev/cpuctl/top-app/cpu.uclamp.min").is_err());
    assert!(guard_path("/dev/cpuctl/cpu.notify_on_migrate").is_err());
}

#[test]
fn allowed_paths_pass() {
    assert!(guard_path("/proc/sys/vm/vfs_cache_pressure").is_ok());
    assert!(guard_path("/sys/devices/system/cpu/cpufreq/policy0/scaling_governor").is_ok());
    // the single audited R&D charge node (ROM opens it for userspace;
    // the charge-enable/JEITA/current nodes stay forbidden)
    assert!(guard_path("/sys/class/power_supply/battery/input_suspend").is_ok());
    assert!(guard_path("/sys/class/power_supply/battery/battery_charging_enabled").is_err());
    // A path that is not in the catalog is never writable, even when it
    // is not explicitly forbidden.
    assert!(guard_path("/proc/sys/vm/vfs_cache_pressure2").is_err());
    assert!(guard_path("/etc/passwd").is_err());
    // Lexical traversal must not dodge a forbidden prefix.
    assert!(guard_path("/sys/class/../module/msm_performance/parameters/cpu_max_freq").is_err());
}

#[test]
fn profile_keys_resolve() {
    for k in [
        "policy0.scaling_governor",
        "gpu.governor",
        "io.scheduler",
        "vm.dirty_ratio",
        "net.tcp_rmem",
        "cpuset.top-app.cpus",
        "stune.top-app.boost",
    ] {
        assert!(find(k).is_some(), "missing catalog entry: {k}");
    }
}

#[test]
fn read_only_kernel_params_are_never_cataloged() {
    // kernel/workqueue.c:294 module_param_named(power_efficient, ..., 0444)
    // — the kernel makes this read-only on every device; it must never
    // re-enter the catalog or any profile.
    assert!(
        find("workqueue.power_efficient").is_none(),
        "workqueue.power_efficient is kernel read-only (0444)"
    );
    // perf HAL transient globals stay forbidden
    assert!(guard_path("/proc/sys/kernel/sched_boost").is_err());
    assert!(guard_path("/sys/devices/system/cpu/cpu0/sched_static_cpu_pwr_cost").is_err());
    // runtime-owned perf HAL nodes (libqti-perfd.so strings) stay forbidden
    for p in [
        "/dev/cpuset/foreground/boost/cpus",
        "/dev/cpu_dma_latency",
        "/sys/kernel/mm/ksm/run",
        "/sys/class/kgsl/kgsl-3d0/force_no_nap",
        "/sys/class/kgsl/kgsl-3d0/force_clk_on",
        "/sys/class/kgsl/kgsl-3d0/force_rail_on",
        "/sys/class/kgsl/kgsl-3d0/idle_timer",
        "/sys/class/mmc_host/mmc0/clk_scaling/enable",
        "/proc/sys/vm/swap_ratio",
    ] {
        assert!(
            guard_path(p).is_err(),
            "{p} must be forbidden (perf HAL runtime-owned)"
        );
    }
}
