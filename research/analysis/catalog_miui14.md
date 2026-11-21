# Catalog probe — MIUI 14 V14.0.2.0.SJGIDXM / Android 12

Sumber: 95 entri katalog MiFineTune diprobe ulang di device (root).

| Key | Tier | Kind | Exists | Perm | Value | Path |
|---|---|---|---|---|---|---|
| `policy0.scaling_governor` | Baseline | Gov | yes | -rw-r--r-- | `schedutil` | `/sys/devices/system/cpu/cpufreq/policy0/scaling_governor` |
| `policy0.scaling_min_freq` | Baseline | FreqMin | yes | -rw-rw-r-- | `1612800` | `/sys/devices/system/cpu/cpufreq/policy0/scaling_min_freq` |
| `policy0.scaling_max_freq` | Baseline | FreqMax | yes | -rw-rw---- | `1804800` | `/sys/devices/system/cpu/cpufreq/policy0/scaling_max_freq` |
| `policy0.schedutil.hispeed_freq` | Baseline | Freq | yes | -rw-r--r-- | `1248000` | `/sys/devices/system/cpu/cpufreq/policy0/schedutil/hispeed_freq` |
| `policy0.schedutil.hispeed_load` | Baseline | Int | yes | -rw-r--r-- | `90` | `/sys/devices/system/cpu/cpufreq/policy0/schedutil/hispeed_load` |
| `policy0.schedutil.up_rate_limit_us` | Baseline | Int | yes | -rw-r--r-- | `0` | `/sys/devices/system/cpu/cpufreq/policy0/schedutil/up_rate_limit_us` |
| `policy0.schedutil.down_rate_limit_us` | Baseline | Int | yes | -rw-r--r-- | `0` | `/sys/devices/system/cpu/cpufreq/policy0/schedutil/down_rate_limit_us` |
| `policy0.schedutil.pl` | Baseline | Int | yes | -rw-r--r-- | `0` | `/sys/devices/system/cpu/cpufreq/policy0/schedutil/pl` |
| `policy0.core_ctl.min_cpus` | Baseline | MinCpus | yes | -rw-r--r-- | `4` | `/sys/devices/system/cpu/cpu0/core_ctl/min_cpus` |
| `policy0.core_ctl.busy_up_thres` | Baseline | RepeatInt | yes | -rw-r--r-- | `60 60 60 60 60 60` | `/sys/devices/system/cpu/cpu0/core_ctl/busy_up_thres` |
| `policy0.core_ctl.busy_down_thres` | Baseline | RepeatInt | yes | -rw-r--r-- | `40 40 40 40 40 40` | `/sys/devices/system/cpu/cpu0/core_ctl/busy_down_thres` |
| `policy0.core_ctl.max_cpus` | Baseline | Int | yes | -rw-r--r-- | `6` | `/sys/devices/system/cpu/cpu0/core_ctl/max_cpus` |
| `policy0.core_ctl.offline_delay_ms` | Baseline | Int | yes | -rw-r--r-- | `100` | `/sys/devices/system/cpu/cpu0/core_ctl/offline_delay_ms` |
| `policy6.scaling_governor` | Baseline | Gov | yes | -rw-r--r-- | `schedutil` | `/sys/devices/system/cpu/cpufreq/policy6/scaling_governor` |
| `policy6.scaling_min_freq` | Baseline | FreqMin | yes | -rw-rw-r-- | `1843200` | `/sys/devices/system/cpu/cpufreq/policy6/scaling_min_freq` |
| `policy6.scaling_max_freq` | Baseline | FreqMax | yes | -rw-rw-r-- | `2304000` | `/sys/devices/system/cpu/cpufreq/policy6/scaling_max_freq` |
| `policy6.schedutil.hispeed_freq` | Baseline | Freq | yes | -rw-r--r-- | `1324600` | `/sys/devices/system/cpu/cpufreq/policy6/schedutil/hispeed_freq` |
| `policy6.schedutil.hispeed_load` | Baseline | Int | yes | -rw-r--r-- | `85` | `/sys/devices/system/cpu/cpufreq/policy6/schedutil/hispeed_load` |
| `policy6.schedutil.up_rate_limit_us` | Baseline | Int | yes | -rw-r--r-- | `0` | `/sys/devices/system/cpu/cpufreq/policy6/schedutil/up_rate_limit_us` |
| `policy6.schedutil.down_rate_limit_us` | Baseline | Int | yes | -rw-r--r-- | `0` | `/sys/devices/system/cpu/cpufreq/policy6/schedutil/down_rate_limit_us` |
| `policy6.schedutil.pl` | Baseline | Int | yes | -rw-r--r-- | `0` | `/sys/devices/system/cpu/cpufreq/policy6/schedutil/pl` |
| `policy6.core_ctl.min_cpus` | Baseline | MinCpus | yes | -rw-r--r-- | `1` | `/sys/devices/system/cpu/cpu6/core_ctl/min_cpus` |
| `gpu.governor` | Baseline | GpuGov | yes | -rw-r--r-- | `msm-adreno-tz` | `/sys/class/kgsl/kgsl-3d0/devfreq/governor` |
| `gpu.max_pwrlevel` | Baseline | PwrLevel | yes | -rw-r--r-- | `0` | `/sys/class/kgsl/kgsl-3d0/max_pwrlevel` |
| `gpu.min_pwrlevel` | Baseline | PwrLevel | yes | -rw-r--r-- | `6` | `/sys/class/kgsl/kgsl-3d0/min_pwrlevel` |
| `gpu.default_pwrlevel` | Baseline | PwrLevel | yes | -rw-r--r-- | `6` | `/sys/class/kgsl/kgsl-3d0/default_pwrlevel` |
| `io.scheduler` | Baseline | IoSched | yes | -rw-r--r-- | `noop deadline [cfq]` | `/sys/block/sda/queue/scheduler` |
| `io.nr_requests` | Free | Int | yes | -rw-r--r-- | `128` | `/sys/block/sda/queue/nr_requests` |
| `io.nomerges` | Free | Int | yes | -rw-r--r-- | `0` | `/sys/block/sda/queue/nomerges` |
| `io.rq_affinity` | Free | Int | yes | -rw-r--r-- | `1` | `/sys/block/sda/queue/rq_affinity` |
| `io.iostats` | Free | Int | yes | -rw-r--r-- | `1` | `/sys/block/sda/queue/iostats` |
| `io.cfq.quantum` | Free | 1 | yes | -rw-r--r-- | `8` | `/sys/block/sda/queue/iosched/quantum` |
| `io.cfq.fifo_expire_sync` | Free | 0 | yes | -rw-r--r-- | `125` | `/sys/block/sda/queue/iosched/fifo_expire_sync` |
| `io.cfq.fifo_expire_async` | Free | 0 | yes | -rw-r--r-- | `250` | `/sys/block/sda/queue/iosched/fifo_expire_async` |
| `io.cfq.back_seek_max` | Free | 0 | yes | -rw-r--r-- | `16384` | `/sys/block/sda/queue/iosched/back_seek_max` |
| `io.cfq.slice_sync` | Free | 1 | yes | -rw-r--r-- | `100` | `/sys/block/sda/queue/iosched/slice_sync` |
| `io.cfq.slice_async` | Free | 1 | yes | -rw-r--r-- | `40` | `/sys/block/sda/queue/iosched/slice_async` |
| `io.cfq.slice_idle_us` | Free | 0 | yes | -rw-r--r-- | `0` | `/sys/block/sda/queue/iosched/slice_idle_us` |
| `io.cfq.target_latency_us` | Free | 1 | yes | -rw-r--r-- | `300000` | `/sys/block/sda/queue/iosched/target_latency_us` |
| `io.cfq.group_idle` | Free | 0 | yes | -rw-r--r-- | `8` | `/sys/block/sda/queue/iosched/group_idle` |
| `io.cfq.low_latency` | Free | 0 | yes | -rw-r--r-- | `1` | `/sys/block/sda/queue/iosched/low_latency` |
| `io.read_ahead_kb` | Free | Int | yes | -rw-r--r-- | `512` | `/sys/block/sda/queue/read_ahead_kb` |
| `devfreq.cpu0_l3_lat.min_freq` | Baseline | Int | yes | -rw-r--r-- | `300000000` | `/sys/class/devfreq/soc:qcom,cpu0-cpu-l3-lat/min_freq` |
| `devfreq.cpu6_l3_lat.min_freq` | Baseline | Int | yes | -rw-r--r-- | `300000000` | `/sys/class/devfreq/soc:qcom,cpu6-cpu-l3-lat/min_freq` |
| `f2fs.gc_urgent` | Baseline | Int | yes | -rw-r--r-- | `0` | `/sys/fs/f2fs/sda16/gc_urgent` |
| `f2fs.gc_urgent_sleep_time` | Baseline | Int | yes | -rw-r--r-- | `500` | `/sys/fs/f2fs/sda16/gc_urgent_sleep_time` |
| `charge.battery_charging_enabled` | Baseline | Int | yes | -rwxrwxrwx | `1` | `/sys/class/power_supply/battery/battery_charging_enabled` |
| `charge.input_suspend` | Baseline | Int | yes | -rwxrwxrwx | `0` | `/sys/class/power_supply/battery/input_suspend` |
| `kernel.sched_conservative_pl` | Baseline | Int | yes | -rw-r--r-- | `0` | `/proc/sys/kernel/sched_conservative_pl` |
| `kernel.sched_latency_ns` | Free | Int | yes | -rw-r--r-- | `10000000` | `/proc/sys/kernel/sched_latency_ns` |
| `kernel.sched_min_granularity_ns` | Free | Int | yes | -rw-r--r-- | `3000000` | `/proc/sys/kernel/sched_min_granularity_ns` |
| `kernel.sched_wakeup_granularity_ns` | Free | Int | yes | -rw-r--r-- | `2000000` | `/proc/sys/kernel/sched_wakeup_granularity_ns` |
| `kernel.sched_migration_cost_ns` | Baseline | Int | yes | -rw-r--r-- | `500000` | `/proc/sys/kernel/sched_migration_cost_ns` |
| `kernel.sched_nr_migrate` | Free | Int | yes | -rw-r--r-- | `32` | `/proc/sys/kernel/sched_nr_migrate` |
| `kernel.sched_autogroup_enabled` | Free | Int | yes | -rw-r--r-- | `1` | `/proc/sys/kernel/sched_autogroup_enabled` |
| `kernel.sched_child_runs_first` | Free | Int | yes | -rw-r--r-- | `0` | `/proc/sys/kernel/sched_child_runs_first` |
| `kernel.sched_walt_rotate_big_tasks` | Baseline | Int | yes | -rw-r--r-- | `1` | `/proc/sys/kernel/sched_walt_rotate_big_tasks` |
| `kernel.sched_little_cluster_coloc_fmin_khz` | Baseline | Int | yes | -rw-r--r-- | `740000` | `/proc/sys/kernel/sched_little_cluster_coloc_fmin_khz` |
| `kernel.sched_upmigrate` | Baseline | Int | yes | -rw-r--r-- | `71` | `/proc/sys/kernel/sched_upmigrate` |
| `kernel.sched_downmigrate` | Baseline | Int | yes | -rw-r--r-- | `65` | `/proc/sys/kernel/sched_downmigrate` |
| `kernel.sched_group_upmigrate` | Baseline | Int | yes | -rw-r--r-- | `100` | `/proc/sys/kernel/sched_group_upmigrate` |
| `kernel.sched_group_downmigrate` | Baseline | Int | yes | -rw-r--r-- | `85` | `/proc/sys/kernel/sched_group_downmigrate` |
| `kernel.sched_min_task_util_for_boost` | Baseline | Int | yes | -rw-r--r-- | `51` | `/proc/sys/kernel/sched_min_task_util_for_boost` |
| `kernel.sched_min_task_util_for_colocation` | Baseline | Int | yes | -rw-r--r-- | `35` | `/proc/sys/kernel/sched_min_task_util_for_colocation` |
| `kernel.sched_sync_hint_enable` | Baseline | Int | yes | -rw-r--r-- | `1` | `/proc/sys/kernel/sched_sync_hint_enable` |
| `kernel.sched_many_wakeup_threshold` | Baseline | Int | yes | -rw-r--r-- | `1000` | `/proc/sys/kernel/sched_many_wakeup_threshold` |
| `kernel.sched_time_avg_ms` | Free | Int | yes | -rw-r--r-- | `1000` | `/proc/sys/kernel/sched_time_avg_ms` |
| `kernel.timer_migration` | Free | Int | yes | -rw-r--r-- | `1` | `/proc/sys/kernel/timer_migration` |
| `kernel.sched_rr_timeslice_ms` | Free | Int | yes | -rw-r--r-- | `100` | `/proc/sys/kernel/sched_rr_timeslice_ms` |
| `vm.vfs_cache_pressure` | Free | Int | yes | -rw-r--r-- | `100` | `/proc/sys/vm/vfs_cache_pressure` |
| `vm.dirty_ratio` | Free | Int | yes | -rw-r--r-- | `20` | `/proc/sys/vm/dirty_ratio` |
| `vm.dirty_background_ratio` | Free | Int | yes | -rw-r--r-- | `5` | `/proc/sys/vm/dirty_background_ratio` |
| `vm.dirty_expire_centisecs` | Free | Int | yes | -rw-r--r-- | `200` | `/proc/sys/vm/dirty_expire_centisecs` |
| `vm.dirty_writeback_centisecs` | Free | Int | yes | -rw-r--r-- | `500` | `/proc/sys/vm/dirty_writeback_centisecs` |
| `vm.stat_interval` | Free | Int | yes | -rw-r--r-- | `1` | `/proc/sys/vm/stat_interval` |
| `vm.watermark_scale_factor` | Baseline | Int | yes | -rw-r--r-- | `1` | `/proc/sys/vm/watermark_scale_factor` |
| `vm.extra_free_kbytes` | Free | Int | yes | -rw-r--r-- | `30375` | `/proc/sys/vm/extra_free_kbytes` |
| `net.tcp_congestion_control` | Free | TcpCc | yes | -rw-r--r-- | `cubic` | `/proc/sys/net/ipv4/tcp_congestion_control` |
| `net.tcp_rmem` | Baseline | Ints | yes | -rw-r--r-- | `524288	1048576	8808040` | `/proc/sys/net/ipv4/tcp_rmem` |
| `net.tcp_wmem` | Baseline | Ints | yes | -rw-r--r-- | `262144	524288	6710886` | `/proc/sys/net/ipv4/tcp_wmem` |
| `net.tcp_slow_start_after_idle` | Free | Int | yes | -rw-r--r-- | `1` | `/proc/sys/net/ipv4/tcp_slow_start_after_idle` |
| `cpu_boost.input_boost_freq` | Baseline | Text | yes | -rw-r--r-- | `0:1324800 1:0 2:0 3:0 4:0 5:0 6:0 7:0` | `/sys/module/cpu_boost/parameters/input_boost_freq` |
| `net.tcp_mtu_probing` | Free | Int | yes | -rw-r--r-- | `2` | `/proc/sys/net/ipv4/tcp_mtu_probing` |
| `net.tcp_fin_timeout` | Free | Int | yes | -rw-r--r-- | `60` | `/proc/sys/net/ipv4/tcp_fin_timeout` |
| `net.tcp_fastopen` | Free | Int | yes | -rw-r--r-- | `1` | `/proc/sys/net/ipv4/tcp_fastopen` |
| `stune.top-app.boost` | Baseline | Int | yes | -rw-r--r-- | `0` | `/dev/stune/top-app/schedtune.boost` |
| `stune.top-app.prefer_idle` | Baseline | Int | yes | -rw-r--r-- | `0` | `/dev/stune/top-app/schedtune.prefer_idle` |
| `stune.foreground.boost` | Baseline | Int | yes | -rw-r--r-- | `0` | `/dev/stune/foreground/schedtune.boost` |
| `stune.foreground.prefer_idle` | Baseline | Int | yes | -rw-r--r-- | `0` | `/dev/stune/foreground/schedtune.prefer_idle` |
| `cpuset.background.cpus` | Baseline | Mask | yes | -rw-r--r-- | `0-2` | `/dev/cpuset/background/cpus` |
| `cpuset.system-background.cpus` | Baseline | Mask | yes | -rw-r--r-- | `0-3` | `/dev/cpuset/system-background/cpus` |
| `cpuset.foreground.cpus` | Baseline | Mask | yes | -rw-rw-r-- | `0-2,4-7` | `/dev/cpuset/foreground/cpus` |
| `cpuset.top-app.cpus` | Baseline | Mask | yes | -rw-rw-r-- | `0-7` | `/dev/cpuset/top-app/cpus` |
