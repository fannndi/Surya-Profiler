# Perf HAL 2.2 resource overlap

Total resource path unik: 166. Overlap persis dengan katalog: 22.

| Major | Minor | Node | Catalog key |
|---|---|---|---|
| 0x3 | 0x2 | `/proc/sys/kernel/sched_migration_cost_ns` | `kernel.sched_migration_cost_ns` |
| 0x3 | 0x7 | `/proc/sys/kernel/sched_upmigrate` | `kernel.sched_upmigrate` |
| 0x3 | 0x8 | `/proc/sys/kernel/sched_downmigrate` | `kernel.sched_downmigrate` |
| 0x3 | 0xF | `/dev/cpuset/top-app/cpus` | `cpuset.top-app.cpus` |
| 0x3 | 0x10 | `/dev/cpuset/foreground/cpus` | `cpuset.foreground.cpus` |
| 0x3 | 0x11 | `/dev/cpuset/system-background/cpus` | `cpuset.system-background.cpus` |
| 0x3 | 0x12 | `/dev/cpuset/background/cpus` | `cpuset.background.cpus` |
| 0x3 | 0x15 | `/proc/sys/kernel/sched_group_upmigrate` | `kernel.sched_group_upmigrate` |
| 0x3 | 0x16 | `/proc/sys/kernel/sched_group_downmigrate` | `kernel.sched_group_downmigrate` |
| 0x3 | 0x18 | `/dev/stune/top-app/schedtune.prefer_idle` | `stune.top-app.prefer_idle` |
| 0x3 | 0x1B | `/proc/sys/kernel/sched_little_cluster_coloc_fmin_khz` | `kernel.sched_little_cluster_coloc_fmin_khz` |
| 0x3 | 0x1C | `/dev/stune/top-app/schedtune.boost` | `stune.top-app.boost` |
| 0x3 | 0x1E | `/proc/sys/kernel/sched_min_task_util_for_colocation` | `kernel.sched_min_task_util_for_colocation` |
| 0x3 | 0x1F | `/proc/sys/kernel/sched_min_task_util_for_boost` | `kernel.sched_min_task_util_for_boost` |
| 0x3 | 0x27 | `/proc/sys/kernel/sched_many_wakeup_threshold` | `kernel.sched_many_wakeup_threshold` |
| 0x3 | 0x28 | `/proc/sys/kernel/sched_sync_hint_enable` | `kernel.sched_sync_hint_enable` |
| 0x3 | 0x35 | `/proc/sys/kernel/sched_min_granularity_ns` | `kernel.sched_min_granularity_ns` |
| 0x3 | 0x37 | `/dev/stune/foreground/schedtune.prefer_idle` | `stune.foreground.prefer_idle` |
| 0xA | 0x0 | `/sys/class/kgsl/kgsl-3d0/default_pwrlevel` | `gpu.default_pwrlevel` |
| 0xA | 0x1 | `/sys/class/kgsl/kgsl-3d0/min_pwrlevel` | `gpu.min_pwrlevel` |
| 0xA | 0x2 | `/sys/class/kgsl/kgsl-3d0/max_pwrlevel` | `gpu.max_pwrlevel` |
| 0xB | 0x2 | `/sys/module/cpu_boost/parameters/input_boost_freq` | `cpu_boost.input_boost_freq` |

## Semua path perf HAL

- `/dev/cpu_dma_latency`
- `/dev/cpuctl/cpu.notify_on_migrate`
- `/dev/cpuctl/foreground/cpu.uclamp.latency_sensitive`
- `/dev/cpuctl/top-app/cpu.uclamp.latency_sensitive`
- `/dev/cpuctl/top-app/cpu.uclamp.min`
- `/dev/cpuset/background/cpus`
- `/dev/cpuset/foreground/cpus`
- `/dev/cpuset/system-background/cpus`
- `/dev/cpuset/top-app/cpus`
- `/dev/stune/foreground/schedtune.prefer_idle`
- `/dev/stune/top-app/schedtune.boost`
- `/dev/stune/top-app/schedtune.colocate`
- `/dev/stune/top-app/schedtune.prefer_idle`
- `/proc/%d/sched_boost`
- `/proc/%d/sched_low_latency`
- `/proc/sys/kernel/%s`
- `/proc/sys/kernel/sched_boost`
- `/proc/sys/kernel/sched_busy_hysteresis_enable_cpus`
- `/proc/sys/kernel/sched_coloc_busy_hyst_ns`
- `/proc/sys/kernel/sched_coloc_busy_hysteresis_enable_cpus`
- `/proc/sys/kernel/sched_coloc_downmigrate_ns`
- `/proc/sys/kernel/sched_downmigrate`
- `/proc/sys/kernel/sched_enable_thread_grouping`
- `/proc/sys/kernel/sched_force_lb_enable`
- `/proc/sys/kernel/sched_freq_aggregate`
- `/proc/sys/kernel/sched_freq_aggregate_threshold`
- `/proc/sys/kernel/sched_group_downmigrate`
- `/proc/sys/kernel/sched_group_upmigrate`
- `/proc/sys/kernel/sched_init_task_load`
- `/proc/sys/kernel/sched_initial_task_util`
- `/proc/sys/kernel/sched_little_cluster_coloc_fmin_khz`
- `/proc/sys/kernel/sched_many_wakeup_threshold`
- `/proc/sys/kernel/sched_migration_cost_ns`
- `/proc/sys/kernel/sched_min_granularity_ns`
- `/proc/sys/kernel/sched_min_task_util_for_boost`
- `/proc/sys/kernel/sched_min_task_util_for_colocation`
- `/proc/sys/kernel/sched_mostly_idle_freq`
- `/proc/sys/kernel/sched_mostly_idle_load`
- `/proc/sys/kernel/sched_mostly_idle_nr_run`
- `/proc/sys/kernel/sched_prefer_idle`
- `/proc/sys/kernel/sched_prefer_spread`
- `/proc/sys/kernel/sched_ravg_window_nr_ticks`
- `/proc/sys/kernel/sched_restrict_cluster_spill`
- `/proc/sys/kernel/sched_small_task`
- `/proc/sys/kernel/sched_spill_nr_run`
- `/proc/sys/kernel/sched_sync_hint_enable`
- `/proc/sys/kernel/sched_task_unfilter_nr_windows`
- `/proc/sys/kernel/sched_upmigrate`
- `/proc/sys/kernel/sched_user_hint`
- `/proc/sys/kernel/sched_window_stats_policy`
- `/proc/sys/kernel/walt_low_latency_task_threshold`
- `/proc/sys/kernel/walt_rtg_cfs_boost_prio`
- `/proc/sys/vm/swap_ratio`
- `/proc/sys/walt/sched_coloc_busy_hyst_cpu_busy_pct`
- `/proc/sys/walt/sched_coloc_busy_hyst_cpu_ns`
- `/proc/sys/walt/sched_conservative_pl`
- `/proc/sys/walt/sched_task_unfilter_period`
- `/sys/class/devfreq/18321000.qcom,devfreq-l3:qcom,cpu%d-cpu-l3-lat/max_freq`
- `/sys/class/devfreq/soc:qcom,cpu-cpu-llcc-bw/bw_hwmon/io_percent`
- `/sys/class/devfreq/soc:qcom,cpu-cpu-llcc-bw/bw_hwmon/sample_ms`
- `/sys/class/devfreq/soc:qcom,cpu-cpu-llcc-bw/bw_hwmon/up_scale`
- `/sys/class/devfreq/soc:qcom,cpu-cpu-llcc-bw/bw_hwmon/use_ab`
- `/sys/class/devfreq/soc:qcom,cpu-cpu-llcc-bw/min_freq`
- `/sys/class/devfreq/soc:qcom,cpu-cpu-llcc-bw/polling_interval`
- `/sys/class/devfreq/soc:qcom,cpu-llcc-ddr-bw/bw_hwmon/decay_rate`
- `/sys/class/devfreq/soc:qcom,cpu-llcc-ddr-bw/bw_hwmon/idle_mbps`
- `/sys/class/devfreq/soc:qcom,cpu-llcc-ddr-bw/bw_hwmon/io_percent`
- `/sys/class/devfreq/soc:qcom,cpu-llcc-ddr-bw/bw_hwmon/low_power_ceil_mbps`
- `/sys/class/devfreq/soc:qcom,cpu-llcc-ddr-bw/bw_hwmon/low_power_io_percent`
- `/sys/class/devfreq/soc:qcom,cpu-llcc-ddr-bw/bw_hwmon/sample_ms`
- `/sys/class/devfreq/soc:qcom,cpu-llcc-ddr-bw/bw_hwmon/up_scale`
- `/sys/class/devfreq/soc:qcom,cpu-llcc-ddr-bw/bw_hwmon/use_ab`
- `/sys/class/devfreq/soc:qcom,cpu-llcc-ddr-bw/max_freq`
- `/sys/class/devfreq/soc:qcom,cpu-llcc-ddr-bw/min_freq`
- `/sys/class/devfreq/soc:qcom,cpu-llcc-ddr-bw/polling_interval`
- `/sys/class/devfreq/soc:qcom,cpu0-cpu-ddr-latfloor/max_freq`
- `/sys/class/devfreq/soc:qcom,cpu0-cpu-ddr-latfloor/min_freq`
- `/sys/class/devfreq/soc:qcom,cpu0-cpu-llcc-lat/max_freq`
- `/sys/class/devfreq/soc:qcom,cpu0-cpu-llcc-lat/mem_latency/ratio_ceil`
- `/sys/class/devfreq/soc:qcom,cpu0-cpu-llcc-lat/mem_latency/stall_floor`
- `/sys/class/devfreq/soc:qcom,cpu0-cpu-llcc-lat/min_freq`
- `/sys/class/devfreq/soc:qcom,cpu0-llcc-ddr-lat/max_freq`
- `/sys/class/devfreq/soc:qcom,cpu0-llcc-ddr-lat/mem_latency/ratio_ceil`
- `/sys/class/devfreq/soc:qcom,cpu0-llcc-ddr-lat/mem_latency/stall_floor`
- `/sys/class/devfreq/soc:qcom,cpu0-llcc-ddr-lat/min_freq`
- `/sys/class/devfreq/soc:qcom,cpu4-cpu-llcc-lat/mem_latency/ratio_ceil`
- `/sys/class/devfreq/soc:qcom,cpu4-cpu-llcc-lat/mem_latency/stall_floor`
- `/sys/class/devfreq/soc:qcom,cpu4-llcc-ddr-lat/mem_latency/ratio_ceil`
- `/sys/class/devfreq/soc:qcom,cpu4-llcc-ddr-lat/mem_latency/stall_floor`
- `/sys/class/devfreq/soc:qcom,cpubw/bw_hwmon/decay_rate`
- `/sys/class/devfreq/soc:qcom,cpubw/bw_hwmon/idle_mbps`
- `/sys/class/devfreq/soc:qcom,cpubw/bw_hwmon/io_percent`
- `/sys/class/devfreq/soc:qcom,cpubw/bw_hwmon/low_power_ceil_mbps`
- `/sys/class/devfreq/soc:qcom,cpubw/bw_hwmon/low_power_io_percent`
- `/sys/class/devfreq/soc:qcom,cpubw/bw_hwmon/sample_ms`
- `/sys/class/devfreq/soc:qcom,cpubw/max_freq`
- `/sys/class/devfreq/soc:qcom,cpubw/min_freq`
- `/sys/class/devfreq/soc:qcom,cpubw/polling_interval`
- `/sys/class/devfreq/soc:qcom,gpubw/max_freq`
- `/sys/class/devfreq/soc:qcom,gpubw/min_freq`
- `/sys/class/devfreq/soc:qcom,l3-cpu%d/min_freq`
- `/sys/class/devfreq/soc:qcom,llccbw/bw_hwmon/io_percent`
- `/sys/class/devfreq/soc:qcom,llccbw/bw_hwmon/sample_ms`
- `/sys/class/devfreq/soc:qcom,llccbw/min_freq`
- `/sys/class/devfreq/soc:qcom,memlat-cpu0/mem_latency/ratio_ceil`
- `/sys/class/devfreq/soc:qcom,memlat-cpu0/mem_latency/stall_floor`
- `/sys/class/devfreq/soc:qcom,memlat-cpu0/min_freq`
- `/sys/class/devfreq/soc:qcom,memlat-cpu4/mem_latency/ratio_ceil`
- `/sys/class/devfreq/soc:qcom,memlat-cpu4/mem_latency/stall_floor`
- `/sys/class/devfreq/soc:qcom,memlat-cpu4/min_freq`
- `/sys/class/devfreq/soc:qcom,npu-llcc-ddr-bw/bw_hwmon/io_percent`
- `/sys/class/devfreq/soc:qcom,npu-llcc-ddr-bw/bw_hwmon/sample_ms`
- `/sys/class/devfreq/soc:qcom,npu-npu-llcc-bw/bw_hwmon/io_percent`
- `/sys/class/devfreq/soc:qcom,npu-npu-llcc-bw/bw_hwmon/sample_ms`
- `/sys/class/kgsl/kgsl-3d0/default_pwrlevel`
- `/sys/class/kgsl/kgsl-3d0/devfreq/max_freq`
- `/sys/class/kgsl/kgsl-3d0/devfreq/min_freq`
- `/sys/class/kgsl/kgsl-3d0/devfreq/mod_percent`
- `/sys/class/kgsl/kgsl-3d0/max_pwrlevel`
- `/sys/class/kgsl/kgsl-3d0/min_pwrlevel`
- `/sys/class/kgsl/kgsl/proc/%d/state`
- `/sys/devices/system/cpu/bus_dcvs/DDR/19091000.qcom,bwmon-ddr/max_freq`
- `/sys/devices/system/cpu/bus_dcvs/DDR/19091000.qcom,bwmon-ddr/min_freq`
- `/sys/devices/system/cpu/bus_dcvs/DDR/soc:qcom,memlat:ddr:silver/min_freq`
- `/sys/devices/system/cpu/bus_dcvs/L3/soc:qcom,memlat:l3:prime/min_freq`
- `/sys/devices/system/cpu/bus_dcvs/LLCC/190b6400.qcom,bwmon-llcc/max_freq`
- `/sys/devices/system/cpu/bus_dcvs/LLCC/190b6400.qcom,bwmon-llcc/min_freq`
- `/sys/devices/system/cpu/bus_dcvs/LLCC/soc:qcom,memlat:llcc:silver/min_freq`
- `/sys/devices/system/cpu/cpu0/cpufreq/interactive/above_hispeed_delay`
- `/sys/devices/system/cpu/cpu0/cpufreq/interactive/boost`
- `/sys/devices/system/cpu/cpu0/cpufreq/interactive/boostpulse`
- `/sys/devices/system/cpu/cpu0/cpufreq/interactive/boostpulse_duration`
- `/sys/devices/system/cpu/cpu0/cpufreq/interactive/go_hispeed_load`
- `/sys/devices/system/cpu/cpu0/cpufreq/interactive/hispeed_freq`
- `/sys/devices/system/cpu/cpu0/cpufreq/interactive/ignore_hispeed_on_notif`
- `/sys/devices/system/cpu/cpu0/cpufreq/interactive/io_is_busy`
- `/sys/devices/system/cpu/cpu0/cpufreq/interactive/max_freq_hysteresis`
- `/sys/devices/system/cpu/cpu0/cpufreq/interactive/min_sample_time`
- `/sys/devices/system/cpu/cpu0/cpufreq/interactive/target_loads`
- `/sys/devices/system/cpu/cpu0/cpufreq/interactive/timer_rate`
- `/sys/devices/system/cpu/cpu0/cpufreq/interactive/timer_slack`
- `/sys/devices/system/cpu/cpu0/cpufreq/interactive/use_migration_notif`
- `/sys/devices/system/cpu/cpu0/cpufreq/interactive/use_sched_load`
- `/sys/devices/system/cpu/cpu0/cpufreq/schedutil/down_rate_limit_us`
- `/sys/devices/system/cpu/cpu0/cpufreq/schedutil/hispeed_freq`
- `/sys/devices/system/cpu/cpu0/cpufreq/schedutil/hispeed_load`
- `/sys/devices/system/cpu/cpu0/cpufreq/schedutil/pl`
- `/sys/devices/system/cpu/cpu0/cpufreq/schedutil/rtg_boost_freq`
- `/sys/devices/system/cpu/cpu0/sched_load_boost`
- `/sys/devices/system/cpu/cpu0/sched_static_cpu_pwr_cost`
- `/sys/devices/system/cpu/cpufreq/ondemand/enable_stepup`
- `/sys/devices/system/cpu/cpufreq/ondemand/io_is_busy`
- `/sys/devices/system/cpu/cpufreq/ondemand/max_intermediate_steps`
- `/sys/devices/system/cpu/cpufreq/ondemand/optimal_freq`
- `/sys/devices/system/cpu/cpufreq/ondemand/sampling_down_factor`
- `/sys/devices/system/cpu/cpufreq/ondemand/sampling_rate`
- `/sys/devices/system/cpu/cpufreq/ondemand/sync_freq`
- `/sys/module/cpu_boost/parameters/input_boost_freq`
- `/sys/module/lpm_levels/parameters/bias_hyst`
- `/sys/module/lpm_levels/parameters/lpm_ipi_prediction`
- `/sys/module/lpm_levels/parameters/ref_stddev`
- `/sys/module/lpm_levels/parameters/tmr_add`
- `/sys/module/lpm_levels/system/perf/perf-l2-pc/idle_enabled`
- `/sys/module/msm_performance/parameters/cpu_max_freq`
- `/sys/module/msm_performance/parameters/cpu_min_freq`
- `/sys/module/process_reclaim/parameters/enable_process_reclaim`
