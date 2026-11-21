# Matriks nilai profil MiFineTune (referensi) vs default MIUI 14

| Key | Default MIUI 14 | powersave | balance | game | sleep | boost |
|---|---|---|---|---|---|---|
| `cpu_boost.input_boost_freq` | `0:1324800 1:0 2:0 3:0 4:0 5:0 6:0 7:0` | `0:0 1:0 2:0 3:0 4:0 5:0 6:0 7:0` |  |  |  |  |
| `cpuset.background.cpus` | `0-2` | `0-3` | `0-3` | `0-1` | `0-3` |  |
| `cpuset.foreground.cpus` | `0-2,4-7` | `0-5` | `0-7` | `0-7` | `0-5` |  |
| `cpuset.system-background.cpus` | `0-3` | `0-3` | `0-5` | `0-1` | `0-3` |  |
| `cpuset.top-app.cpus` | `0-7` | `0-5` | `0-7` | `0-7` | `0-5` | `0-7` |
| `devfreq.cpu0_l3_lat.min_freq` | `300000000` | `300000000` | `300000000` | `940800000` | `300000000` |  |
| `devfreq.cpu6_l3_lat.min_freq` | `300000000` | `300000000` | `300000000` | `940800000` | `300000000` |  |
| `gpu.default_pwrlevel` | `6` |  |  | `4` |  | `4` |
| `gpu.max_pwrlevel` | `0` | `5` | `1` | `0` | `5` | `0` |
| `gpu.min_pwrlevel` | `6` | `6` | `6` | `3` | `6` | `3` |
| `io.cfq.fifo_expire_async` | `250` | `500` |  |  | `500` |  |
| `io.cfq.low_latency` | `1` | `0` |  |  | `0` |  |
| `io.cfq.slice_async` | `40` | `60` |  |  | `60` |  |
| `io.iostats` | `1` | `0` | `1` | `1` | `0` |  |
| `io.nomerges` | `0` | `1` | `0` | `0` | `1` |  |
| `io.nr_requests` | `128` | `64` | `128` | `256` | `32` |  |
| `io.read_ahead_kb` | `512` | `128` | `512` | `1024` | `128` |  |
| `io.rq_affinity` | `1` |  | `2` | `2` |  | `2` |
| `io.scheduler` | `noop deadline [cfq]` | `cfq` | `cfq` | `deadline` | `cfq` |  |
| `kernel.sched_conservative_pl` | `0` | `0` | `0` | `1` | `0` | `0` |
| `kernel.sched_downmigrate` | `65` | `50` | `70` | `50` | `50` |  |
| `kernel.sched_group_downmigrate` | `85` |  |  | `120` |  | `120` |
| `kernel.sched_group_upmigrate` | `100` |  |  | `140` |  | `140` |
| `kernel.sched_latency_ns` | `10000000` | `20000000` | `10000000` | `6000000` | `16000000` |  |
| `kernel.sched_little_cluster_coloc_fmin_khz` | `740000` |  | `1248000` | `1017600` |  | `1017600` |
| `kernel.sched_migration_cost_ns` | `500000` | `50000000` | `2000000` | `500000` | `100000000` |  |
| `kernel.sched_min_granularity_ns` | `3000000` | `6000000` | `3000000` | `1000000` | `4000000` |  |
| `kernel.sched_min_task_util_for_boost` | `51` |  | `51` |  |  |  |
| `kernel.sched_min_task_util_for_colocation` | `35` |  | `35` |  |  |  |
| `kernel.sched_nr_migrate` | `32` | `16` | `32` | `8` | `8` |  |
| `kernel.sched_upmigrate` | `71` | `90` | `80` | `60` | `90` |  |
| `kernel.sched_wakeup_granularity_ns` | `2000000` | `4000000` | `3000000` | `500000` | `3000000` |  |
| `kernel.sched_walt_rotate_big_tasks` | `1` |  |  | `0` |  | `0` |
| `net.tcp_slow_start_after_idle` | `1` |  |  | `0` | `0` |  |
| `policy0.core_ctl.busy_down_thres` | `40 40 40 40 40 40` | `55` | `35` | `15` | `65` | `10` |
| `policy0.core_ctl.busy_up_thres` | `60 60 60 60 60 60` | `75` | `55` | `35` | `85` | `25` |
| `policy0.core_ctl.max_cpus` | `6` | `4` |  |  | `2` |  |
| `policy0.core_ctl.min_cpus` | `4` | `2` | `2` | `6` | `1` | `6` |
| `policy0.core_ctl.offline_delay_ms` | `100` | `60` | `120` | `400` | `40` | `800` |
| `policy0.core_ctl.task_thres` | `8` |  | `8` | `12` |  | `12` |
| `policy0.scaling_governor` | `schedutil` | `powersave` | `schedutil` | `schedutil` | `schedutil` | `schedutil` |
| `policy0.scaling_max_freq` | `1804800` | `1248000` | `1708800` | `1804800` | `1017600` | `1804800` |
| `policy0.scaling_min_freq` | `1612800` | `576000` | `576000` | `768000` | `576000` | `768000` |
| `policy0.schedutil.down_rate_limit_us` | `0` |  | `0` | `0` |  | `0` |
| `policy0.schedutil.hispeed_freq` | `1248000` |  | `1324800` | `1497600` | `768000` | `1804800` |
| `policy0.schedutil.hispeed_load` | `90` |  | `90` | `75` | `95` | `75` |
| `policy0.schedutil.pl` | `0` | `0` | `0` | `1` | `0` | `1` |
| `policy0.schedutil.up_rate_limit_us` | `0` |  | `0` | `0` |  | `0` |
| `policy6.core_ctl.min_cpus` | `1` |  |  |  | `1` |  |
| `policy6.scaling_governor` | `schedutil` | `powersave` | `schedutil` | `schedutil` | `schedutil` |  |
| `policy6.scaling_max_freq` | `2304000` | `1324800` | `1324800` | `2304000` | `1324800` | `2304000` |
| `policy6.scaling_min_freq` | `1843200` | `652800` | `652800` | `1094400` | `652800` | `1094400` |
| `policy6.schedutil.down_rate_limit_us` | `0` |  | `0` | `0` |  | `0` |
| `policy6.schedutil.hispeed_freq` | `1324600` |  | `1209600` | `1555200` | `1094400` | `2304000` |
| `policy6.schedutil.hispeed_load` | `85` |  | `90` | `70` | `95` | `70` |
| `policy6.schedutil.pl` | `0` | `0` | `0` | `1` | `0` | `1` |
| `policy6.schedutil.up_rate_limit_us` | `0` |  | `0` | `0` |  | `0` |
| `stune.top-app.boost` | `0` | `0` | `0` | `10` | `0` | `10` |
| `stune.top-app.prefer_idle` | `0` | `0` | `0` | `1` | `0` | `1` |
| `vm.dirty_background_ratio` | `5` | `10` | `10` | `10` | `10` |  |
| `vm.dirty_expire_centisecs` | `200` | `400` | `500` | `600` | `400` |  |
| `vm.dirty_ratio` | `20` | `25` | `25` | `30` | `25` |  |
| `vm.dirty_writeback_centisecs` | `500` | `1000` | `1000` | `1500` | `1000` |  |
| `vm.stat_interval` | `1` | `10` | `1` | `3` | `10` |  |
| `vm.vfs_cache_pressure` | `100` | `150` | `100` | `50` | `120` |  |
| `vm.watermark_scale_factor` | `1` |  |  | `10` |  |  |
