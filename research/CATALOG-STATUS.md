# Catalog status — 95 entri referensi di MIUI 14

> **0.2 slim build:** 3 entries were removed from the active catalog
> (`charge.battery_charging_enabled` and the two f2fs GC nodes), leaving
> **92**. The remaining charge node is `charge.input_suspend` (R&D only).
> This document stays as the 95-entry R&D reference.
>
> **2026-11-21 R&D:** additional SoC nodes were audited (write/readback/
> restore verified on device) but **not cataloged** — stock values are
> already optimal and no profile uses them: kgsl `ifpc`/`bus_split`/
> `gpu_llc_slice_enable`/`gpuhtw_llc_slice_enable`/`preempt_level`,
> `devfreq/cpu*-cpu-l3-lat/max_freq`, `stune/background/*`,
> `cpu0/core_ctl/enable`. WALT nodes are absent on this kernel. Evidence
> and bounds: [RND-BATTERY-PERF-2026-11-21.md](RND-BATTERY-PERF-2026-11-21.md).

Sumber: `research/analysis/catalog_miui14.md` (diprobe ulang di device),
`research/analysis/perf_overlap.md`, `research/rom/init.qcom.post_boot.sh`
(arm moorea soc 365/366), dan `research/rom/config/perfd_default_values`.

## Ringkasan

- **95/95 path ada** di MIUI 14; tidak ada yang hilang.
- Permission: mayoritas `-rw-r--r-- root:root`; beberapa `system:system`
  (scaling min/max, charge nodes) dan `root:system` (cpuset foreground/top-app).
- Root APatch bisa menulis semua path yang diuji.
- Readback `io.scheduler` hanya token aktif dalam bracket; restore harus
  menulis token, bukan string readback.
- `core_ctl` ada di `cpu0/core_ctl` dan `cpu6/core_ctl`; big cluster
  `enable=0`, `task_thres=4294967295`.

## Perbedaan nilai baseline MIUI 14 vs asumsi referensi

| Key | MIUI 14 stock | Catatan |
|---|---|---|
| `policy0.scaling_min_freq` | 576000 (off) / 1612800 saat boost | boost = `msm_performance` (interaction-scoped) |
| `policy6.scaling_min_freq` | 652800 (off) / 1843200 saat boost | boost = floor (interaction-scoped) |
| `policy0.scaling_max_freq` | 1804800 | cap minimal saat on = 1612800 |
| `policy6.scaling_max_freq` | 2304000 | cap minimal saat on = 1843200 |
| `policy0.core_ctl.min/max` | 4 / 6 | bukan 4/6 di policy path |
| `policy6.core_ctl` | enable=0 | jangan harap hotplug big |
| `cpuset.background` | 0-2 | referensi 0-3 |
| `cpuset.system-background` | 0-3 | referensi 0-5 |
| `cpuset.foreground` | 0-2,4-7 | referensi 0-5/0-7 |
| `cpuset.top-app` | 0-7 | sama |
| `io.cfq.*` | quantum 8, fifo 125/250, slice 100/40, low_latency 1 | referensi beda |
| `vm.dirty_ratio` | 20 | referensi 25/30 |
| `vm.dirty_background_ratio` | 5 | referensi 10 |
| `vm.dirty_expire_centisecs` | 200 | referensi 400/500/600 |
| `vm.dirty_writeback_centisecs` | 500 | referensi 1000/1500 |
| `gpu.default_pwrlevel` | 6 | referensi game 4 |
| `cpu_boost.input_boost_freq` | 0:1324800 | powerkey 4:1804800/7:2208000 |
| `kernel.sched_up/downmigrate` | 71/65 | sama |
| `kernel.sched_group_up/down` | 100/85 | sama |
| `kernel.sched_little_cluster_coloc_fmin_khz` | 740000 | sama |
| `kernel.sched_walt_rotate_big_tasks` | 1 | sama |
| `kernel.sched_conservative_pl` | 0 | sama |

## Tier / ownership

### Perf HAL 2.2 overlap (Baseline)

22 path persis overlap dengan katalog (lihat `research/analysis/perf_overlap.md`):

- `sched_migration_cost_ns`, `sched_upmigrate`, `sched_downmigrate`,
  `sched_group_upmigrate`, `sched_group_downmigrate`,
  `sched_min_granularity_ns`, `sched_min_task_util_for_boost`,
  `sched_min_task_util_for_colocation`, `sched_many_wakeup_threshold`,
  `sched_sync_hint_enable`, `sched_little_cluster_coloc_fmin_khz`
- `cpuset.top-app`, `cpuset.foreground`, `cpuset.system-background`,
  `cpuset.background`
- `stune.top-app.boost`, `stune.top-app.prefer_idle`,
  `stune.foreground.prefer_idle`
- `gpu.default_pwrlevel`, `gpu.min_pwrlevel`, `gpu.max_pwrlevel`
- `cpu_boost.input_boost_freq`

Ini **Baseline**: kita boleh menulis baseline, framework boleh menimpa;
watchdog/reconcile harus memulihkan.

### Forbidden (framework-owned)

- `/sys/module/msm_performance/*` (sumber floor CPU screen-on)
- `/sys/module/cpu_boost/*` selain `input_boost_freq`
- `/sys/class/thermal/*`, `/data/vendor/thermal`
- `/dev/cpuset/game`, `gamelite`, `vr`, `audio-app`, `camera-daemon`,
  `restricted`, `foreground/boost`
- `/dev/stune/rt`, `/dev/stune/audio-app`
- `/sys/class/kgsl/kgsl-3d0/{force_no_nap,force_clk_on,force_rail_on,idle_timer}`,
  devfreq Hz view `min_freq`/`max_freq`
- charger paths `main`, `dc`, `usb`, `bms`, `bq2597x-standalone`, `pc_port`
- `/proc/sys/vm/{swappiness,min_free_kbytes,page-cluster,watermark_boost_factor,swap_ratio}`
- `/sys/block/zram*`, `/sys/module/lowmemorykiller`, `/sys/module/process_reclaim`
- `/proc/sys/kernel/sched_boost`, WALT lib hint, `sched_lib_*`
- SELinux paths
- `/proc/sys/kernel/sched_coloc_*` (absent di kernel ini; future guard)

### Free / boot baseline

- Semua path katalog lain dianggap Free atau boot-Baseline sesuai evidence
  `post_boot` arm moorea. Tidak ada runtime writer tambahan yang ditemukan
  pada audit ini selain perf HAL/PowerKeeper/thermal/mcd/millet.

## Catatan path

- `post_boot` menulis melalui alias `cpu0/cpufreq/...` dan `cpu6/cpufreq/...`;
  path efektif sama dengan `policy0`/`policy6`.
- `cpuset.foreground` stock **0-2,4-7** (cpu3 tidak dipakai foreground).
  Jangan menganggap ini typo tanpa bukti; profile yang mengubahnya harus diuji.
- `policy6.core_ctl.enable=0`; menulis min/max big tidak akan menghidupkan
  hotplug. Jangan masukkan core_ctl big ke profile.
- `charge_full_design` rusak; gunakan `bms.charge_full_design` (5216 mAh) atau
  spec 5160 mAh untuk wear estimate.
