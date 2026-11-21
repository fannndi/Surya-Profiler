# Scheduler / WALT — MIUI 14 surya

Status: **R&D track for v0.5.x; no profile values shipped yet.**
Source: `/vendor/bin/init.qcom.post_boot.sh` (sm6150 / SDMMAGPIE branch),
`vendor/etc/perf/*`, and the perf-HAL overlap audit.

## Why this track

The current profiles tune cpufreq, core_ctl, cpuset, stune, GPU, IO and VM.
MIUI's own boot script also writes WALT task-packing and migration nodes that
are not in the 95-entry catalog. They can improve responsiveness or battery
without touching the framework-owned CPU floor. We only add them after a
device audit and an A/B run.

## ROM baseline (from `init.qcom.post_boot.sh`)

The sdmmagpie/msmsteppe branch writes, for example:

```text
/proc/sys/kernel/sched_window_stats_policy        3
/proc/sys/kernel/sched_ravg_hist_size             3
/proc/sys/kernel/sched_restrict_tasks_spread      1   # some branches
/proc/sys/kernel/sched_small_task                 20
/sys/devices/system/cpu/cpuN/sched_mostly_idle_load   30
/sys/devices/system/cpu/cpuN/sched_mostly_idle_nr_run  3
/sys/devices/system/cpu/cpuN/sched_prefer_idle         0
```

These are boot baselines, not necessarily the live values after PowerKeeper
and the perf HAL have run. Always audit the live values first:

```bash
adb push research/scripts/audit_walt.sh /data/local/tmp/
adb shell su -c 'sh /data/local/tmp/audit_walt.sh'
```

## Node inventory / ownership

| Node | Proposed tier | Notes |
|---|---|---|
| `kernel.sched_restrict_tasks_spread` | Baseline | WALT packing; not in current catalog |
| `kernel.sched_small_task` | Baseline | WALT small-task threshold |
| `kernel.sched_window_stats_policy` | Baseline | window statistics policy |
| `kernel.sched_ravg_hist_size` | Baseline | window history size |
| `kernel.sched_mostly_idle_load` | Baseline (per CPU) | task packing |
| `kernel.sched_mostly_idle_nr_run` | Baseline (per CPU) | task packing |
| `kernel.sched_prefer_idle` | Baseline (per CPU) | prefer idle CPUs |
| `kernel.sched_upmigrate` / `downmigrate` | Baseline | already catalogued |
| `kernel.sched_group_up/downmigrate` | Baseline | already catalogued |
| `kernel.sched_min_task_util_for_boost` | Baseline | already catalogued |
| `kernel.sched_sync_hint_enable` | Baseline | perf-HAL overlap |
| `kernel.sched_many_wakeup_threshold` | Baseline | perf-HAL overlap |

Forbidden, even if a profile wants them:

- `/sys/module/msm_performance/*` (screen-on floor)
- `/proc/sys/kernel/sched_boost`
- `/proc/sys/kernel/sched_lib_*` / WALT lib hints
- perf-HAL-only GPU nodes (`force_no_nap`, `idle_timer`, ...)
- `/dev/cpuset/game`, `gamelite`, `vr`, `audio-app`, `camera-daemon`
- `/dev/stune/rt`, `/dev/stune/audio-app`

## Draft candidate matrix (not shipped)

| Profile | `restrict_tasks_spread` | `small_task` | `mostly_idle_load` | `mostly_idle_nr_run` | `prefer_idle` |
|---|---:|---:|---:|---:|---:|
| Power Save | 1 | 20 | 30 | 3 | 0 (little), 1 (big) |
| Balance | stock | stock | stock | stock | stock |
| Game | 0 | 0 | 0 | 1 | 0 |

Rationale:

- **Power Save:** pack small tasks aggressively, keep them on the little
  cluster, let big cores idle. The screen-on floor still wins.
- **Balance:** do not fight MIUI's stock WALT baseline.
- **Game:** reduce packing, allow faster spread/migration for frame-critical
  threads. Use only after thermal headroom is verified.

`mostly_idle_load` and `mostly_idle_nr_run` are per-CPU; a catalog entry must
write the same value to all 8 CPUs (or the profile pack must use 8 keys).
Prefer a repeat-int catalog kind or a helper that expands the mask.

## A/B protocol

1. Baseline: Service OFF, stock MIUI, 10 min workload.
2. Candidate: Service ON, one profile, same workload.
3. Workloads: idle (screen on), Chrome scroll, 1080p video, Epic Seven.
4. Record every 5 s:
   - battery current (`current_now`), battery/quiet/nvm temperatures,
   - `policy0/6` current and max frequency,
   - `measured_fps`, `gpu_busy_percentage`,
   - `thermal.dump` events.
5. Compare:
   - average/peak power,
   - 1% low FPS / jank (if measurable),
   - time above 55 °C nvm,
   - thermal throttle events.
6. Keep a candidate only if it wins on at least one axis without a thermal
   or stability regression. Revert otherwise.

## Open questions

- Does the perf HAL rewrite `sched_prefer_idle`/`sched_mostly_idle_*` at
  runtime, or only at boot?
- Are per-CPU `sched_mostly_idle_*` nodes writable by root on this kernel?
- Does `sched_restrict_tasks_spread=1` interact with cpuset `game`/`gamelite`?
- Does PowerKeeper's `mPowerMode` change any of these nodes when the user
  switches MIUI power mode?

Until these are answered, the scheduler track stays read-only.
