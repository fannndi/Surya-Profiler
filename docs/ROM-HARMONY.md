# ROM harmony map — who owns what on MIUI 14 / surya

Surya-Profiler coexists with MIUI instead of fighting it. This document maps
every tuning surface to its owner, derived from the shipped binaries
(RE-confirmed 2026-11-23, see `research/REA-2026-11-23.md`) plus the vendor's
own declarative configs. The engine enforces this via
`engine::catalog::guard_path` (membership gate + `FORBIDDEN_PREFIXES`).

## Writer chains (who actually writes the nodes)

```
com.miui.powerkeeper (system app)
  ├─ thermal: writes thermal_message/sconfig + /data/vendor/thermal/
  │   thermal-global-mode + persist.sys.thermal.config (ThermalSwitchHandler)
  ├─ perf mode: binder → vendor.perfservice (IPerfManager, system_ext proxy)
  │     → dlopen libqti-perfd-client_system.so → perf HAL
  └─ game intents: broadcast to base IDs 9/13/16/20/23

vendor.perfservice (system_ext/bin)  = pure binder proxy, writes NO nodes
  → forwards perf_hint / perf_lock_acq_rel / perf_sync_request

android.hardware.power-service (AIDL, vendor/qcom/opensource/power)
  → dlopen libqti-perfd-client.so → perf_hint / perf_lock_acq
  → reads /vendor/etc/powerhint.xml via libxml2

vendor.qti.hardware.perf@2.2-service  = thin shell (Perf.cpp)
  → libqti-perfd.so + libperfconfig.so / libperfgluelayer.so
  → /vendor/etc/perf/*.xml declare resources + boost hints
  → the ONLY node-writer in the QTI chain (generic "index → value" writes)

miuibooster (system/xbin)  = MiuiBoosterService binder server
  → requestCpu/Ddr/Gpu/IOHighFreq, requestThreadPriority
  → auth via persist.sys.mibridge_auth_uids; freq votes via perf HAL

mi_thermald (vendor/bin, u:r:mi_thermald:s0)
  → sconfig inotify + /data/vendor/thermal/config/ dir inotify
  → writes: sconfig, thermal_message/{cpu_limits,boost,screen_state,
    board_sensor_temp}, cooling cur_state, cpuN scaling_max_freq, cpu online
```

## Shared surface (we write, the perf HAL also writes)

Our Baseline entries overlap the perf HAL's declarative resources for 26
nodes (`commonresourceconfigs.xml` / `targetresourceconfigs.xml`).
Transient perf-HAL hint writes can clobber ours; the daemon supervisor's
drift reconcile is the designed answer — these stay **cataloged**, never
forbidden:

- `/dev/cpuset/{background,foreground,system-background,top-app}/cpus`
- `/dev/stune/foreground/schedtune.prefer_idle`,
  `/dev/stune/top-app/schedtune.{boost,prefer_idle}`
- `/proc/sys/kernel/sched_{downmigrate,upmigrate,group_downmigrate,
  group_upmigrate,little_cluster_coloc_fmin_khz,many_wakeup_threshold,
  migration_cost_ns,min_granularity_ns,min_task_util_for_boost,
  min_task_util_for_colocation,sync_hint_enable}`
- `/sys/class/kgsl/kgsl-3d0/{default,max,min}_pwrlevel`
- `/sys/devices/system/cpu/cpufreq/policy{0,6}/schedutil/{hispeed_freq,
  hispeed_load,pl,down_rate_limit_us}` (XML spells them `cpu0`/`cpuN`)
- `/sys/module/cpu_boost/parameters/input_boost_freq` (post_boot seeds,
  perf HAL saves — see `ALLOWED_EXACT`)

## Perf-HAL-owned (never ours)

Hint-writes outside the shared list are forbidden or simply not cataloged:
`/dev/cpuctl` (uclamp latency_sensitive/min, notify_on_migrate),
per-task `/proc/<pid>/sched_boost|sched_low_latency`,
`/dev/stune/*/schedtune.colocate`, kgsl `devfreq/{min,max}_freq` +
`mod_percent`, bus_dcvs/llcc/cpubw/memlat devfreqs, `interactive`/`ondemand`
governor params, `/dev/cpu_dma_latency`, lpm_levels, process_reclaim,
`vm/swap_ratio`, msm_performance, `sched_boost(_top_app)`, and the
SPECIAL_NODE set (lock_max_cores, display_off, irq_balancer, …).

## Boost hint inventory (sdmmagpie, `perfboostsconfig.xml`)

Timeouts are the transient windows MIUI/QTI boosts run for — they explain
the observed floor decay times in `research/RUNTIME-BEHAVIOR.md`:

| Hint | Type | Timeout | Effect (decoded from opcodes/values) |
|---|---|---:|---|
| `0x1081` launch | 1 | 2000 ms | `sched_boost=1`, max-freq unset (0xFFF) |
| `0x1082` resume | 1 | 400 ms | same as launch, shorter window |
| `0x1081` | 2 | 1500 ms | power-collapse off, storage scaling, CPUBW/L3/GPU-nap lifts |
| `0x1081` | 3/4 | 15000 ms | `sched_boost=3`, sched migrate 95/100 |
| `0x1081` | 5 | 2000 ms | cpuset top-app = cpu4, stune top-app 40, uclamp per-cpu 60/33 |
| `0x1081` | 6 | 2000 ms | per-cpu uclamp 60/33, min-freq big 1324 |
| `0x1087` | 1 | 0 | input boost: min-freq big 1324 / little 1248 |
| `0x1080` | 1/2/4 | 80 ms | min-freq 1094/1248, `sched_boost=2` (touch blip family) |
| `0x1083` | — | 400 ms | restore: min-freq 2438/1804 |
| `0x1086/1088/1089/1090` | — | ≤1500 ms | unlock/recovery/ux variants |
| `0x1040/0x1041` | — | 0 | master enable/disable of the QTI boost stack |

## MIUI thermal ownership (summary)

- `mi_thermald` uses `/data/vendor/thermal/config/` **first**; map IDs must
  be `< 65`; an unknown ID falls back to `thermal-global-mode`.
- PowerKeeper base IDs 0–24 + offsets `+30/+100/+200/+500`; per-game
  virtual-sensor compensation −2 °C for the SP-game list
  (Genshin/sgame/Yuanshen variants — surfaced as `miui_sp_game` in diag).
- SuryaProfiler owns only private IDs 60–63 + the marker-owned files under
  `/data/vendor/thermal/config/`, and yields camera/phone/ARVR/perf-mode.
