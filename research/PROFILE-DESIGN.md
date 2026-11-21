> **0.2 slim build:** the shipped pack is Power Save, Balance, Game and the
> hidden Sleep profile. The Boost section below is kept as R&D history only.

# Profile design — constraint MIUI 14 dan kandidat awal

Status: **draft untuk R&D**; belum final dan belum diuji sebagai satu set
profil utuh. Nilai harus divalidasi lewat `plan/apply/verify` di device.

## Constraint baru yang mengikat semua profil

1. **CPU floor saat boost tidak bisa diturunkan** (floor interaction-scoped: wake ~3 dtk, touch ~1 dtk):
   - little `scaling_min_freq` efektif >= **1612800**
   - big `scaling_min_freq` efektif >= **1843200**
   - Sumber: `msm_performance.cpu_min_freq` (PowerKeeper/perf stack).
2. **`scaling_max_freq` harus >= floor saat ini**, bukan hanya >= OPP:
   - little max minimal 1612800 saat screen-on, 576000 saat screen-off.
   - big max minimal 1843200 saat screen-on, 652800 saat screen-off.
   - Jika max < floor, kernel **meng-clamp ke floor** dengan write rc=0
     (readback = floor, bukan nilai yang diminta). Validator harus mendeteksi
     clamp ini dan menandai `Locked("max < effective min")` atau menaikkan
     max ke floor dengan alasan yang terlihat di plan/readback.
3. **Jangan menulis `msm_performance`, `sched_boost`, atau `cpu_boost` selain
   `input_boost_freq`.**
4. **Governor `powersave` tidak otomatis membuat CPU rendah** saat screen-on;
   ia akan berjalan di floor. Efeknya mirip schedutil dengan max=floor.
5. **Big cluster core_ctl disabled**; jangan andalkan hotplug big.
6. **Cpuset stock MIUI 14** bukan cpuset MIUI 12; profile harus memakai
   baseline baru dan diuji.
7. **Refresh** butuh SF 1035; settings key saja no-op.
8. **Balance big cap** tidak boleh 1324800 (referensi lama); minimal 1843200.

## Kandidat profil (draft)

Nilai di bawah adalah titik awal. Kolom "Alasan" menjelaskan intent, bukan
jaminan hasil.

### Power Save

| Key | Nilai draft | Alasan |
|---|---|---|
| `policy0.scaling_governor` | `powersave` | min-lock, eksplisit |
| `policy0.scaling_min_freq` | `1612800` | floor screen-on |
| `policy0.scaling_max_freq` | `1612800` | cap = floor; tidak bisa lebih rendah |
| `policy6.scaling_governor` | `powersave` | min-lock |
| `policy6.scaling_min_freq` | `1843200` | floor |
| `policy6.scaling_max_freq` | `1843200` | cap = floor |
| `policy0.core_ctl.min_cpus` | `2` | park little cores (max 6) |
| `policy0.core_ctl.max_cpus` | `4` | batasi core aktif |
| `policy0.core_ctl.busy_up_thres` | `75` | lebih malas menyalakan core |
| `policy0.core_ctl.busy_down_thres` | `55` | |
| `policy0.core_ctl.offline_delay_ms` | `60` | |
| `policy0.core_ctl.task_thres` | `8` | >= 6 (kernel) |
| `gpu.max_pwrlevel` | `5` | 267 MHz |
| `gpu.min_pwrlevel` | `6` | 180 MHz |
| `gpu.default_pwrlevel` | `6` | |
| `io.scheduler` | `cfq` | |
| `io.nr_requests` | `64` | antrean kecil |
| `io.read_ahead_kb` | `128` | |
| `io.nomerges` | `1` | |
| `io.iostats` | `0` | |
| `io.rq_affinity` | `1` | stock |
| `io.cfq.fifo_expire_async` | `500` | batching |
| `io.cfq.slice_async` | `60` | |
| `io.cfq.low_latency` | `0` | |
| `vm.dirty_ratio` | `25` | writeback lebih santai |
| `vm.dirty_background_ratio` | `10` | |
| `vm.dirty_expire_centisecs` | `400` | |
| `vm.dirty_writeback_centisecs` | `1000` | |
| `vm.stat_interval` | `10` | |
| `kernel.sched_upmigrate` | `90` | migrasi big lebih sulit |
| `kernel.sched_downmigrate` | `50` | |
| `kernel.sched_latency_ns` | `20000000` | |
| `kernel.sched_min_granularity_ns` | `6000000` | |
| `kernel.sched_wakeup_granularity_ns` | `4000000` | |
| `kernel.sched_migration_cost_ns` | `50000000` | |
| `kernel.sched_nr_migrate` | `16` | |
| `stune.top-app.boost` | `0` | |
| `stune.top-app.prefer_idle` | `0` | |
| `cpuset.background.cpus` | `0-2` | stock |
| `cpuset.system-background.cpus` | `0-3` | stock |
| `cpuset.foreground.cpus` | `0-2,4-7` | stock |
| `cpuset.top-app.cpus` | `0-7` | stock |
| `cpu_boost.input_boost_freq` | `0:0 1:0 2:0 3:0 4:0 5:0 6:0 7:0` | matikan touch boost (floor tetap) |
| `devfreq.cpu0_l3_lat.min_freq` | `300000000` | stock |
| `devfreq.cpu6_l3_lat.min_freq` | `300000000` | stock |
| `policy0.schedutil.pl` | `0` | |
| `policy6.schedutil.pl` | `0` | |
| `kernel.sched_conservative_pl` | `0` | |

Catatan: karena floor, profil ini tidak akan membuat CPU <1.6/1.8 GHz saat
screen-on. Keuntungan utama: cap beban puncak, GPU/IO, core parking, dan
touch boost off.

### Balance

| Key | Nilai draft | Alasan |
|---|---|---|
| `policy0.scaling_governor` | `schedutil` | |
| `policy0.scaling_min_freq` | `1612800` | floor |
| `policy0.scaling_max_freq` | `1708800` | cap sedikit di bawah stock |
| `policy0.schedutil.hispeed_freq` | `1612800` | di floor |
| `policy0.schedutil.hispeed_load` | `90` | |
| `policy6.scaling_governor` | `schedutil` | |
| `policy6.scaling_min_freq` | `1843200` | floor |
| `policy6.scaling_max_freq` | `1939200` | cap hemat; **bukan 1324800** |
| `policy6.schedutil.hispeed_freq` | `1843200` | |
| `policy6.schedutil.hispeed_load` | `90` | |
| `policy0.schedutil.pl` | `0` | |
| `policy6.schedutil.pl` | `0` | |
| `kernel.sched_conservative_pl` | `0` | |
| `policy0.core_ctl.min_cpus` | `4` | stock |
| `policy0.core_ctl.max_cpus` | `6` | stock |
| `policy0.core_ctl.busy_up_thres` | `55` | |
| `policy0.core_ctl.busy_down_thres` | `35` | |
| `policy0.core_ctl.offline_delay_ms` | `120` | |
| `gpu.max_pwrlevel` | `1` | 650 MHz |
| `gpu.min_pwrlevel` | `6` | |
| `io.scheduler` | `cfq` | |
| `io.nr_requests` | `128` | |
| `io.read_ahead_kb` | `512` | |
| `io.rq_affinity` | `2` | |
| `kernel.sched_upmigrate` | `80` | |
| `kernel.sched_downmigrate` | `70` | |
| `kernel.sched_group_upmigrate` | `100` | stock |
| `kernel.sched_group_downmigrate` | `85` | stock |
| `kernel.sched_little_cluster_coloc_fmin_khz` | `1248000` | |
| `kernel.sched_latency_ns` | `10000000` | |
| `kernel.sched_min_granularity_ns` | `3000000` | |
| `kernel.sched_wakeup_granularity_ns` | `3000000` | |
| `kernel.sched_migration_cost_ns` | `2000000` | |
| `kernel.sched_nr_migrate` | `32` | |
| `stune.top-app.boost` | `0` | |
| `stune.top-app.prefer_idle` | `0` | |
| cpuset | stock MIUI 14 | jangan copy MIUI 12 |
| `cpu_boost.input_boost_freq` | stock `0:1324800 ...` | |
| `devfreq l3 min` | `300000000` | |

### Game

> **Thermal reality (2026-11-20):** the screen-on big-cluster floor is
> **1 843 200 kHz**; sub-floor thermal targets are clamped. v0.4.0 uses
> floor-aware custom steps (`2 169 600`/`1 939 200`/`1 843 200`) and direct
> `nvm-therm-adc` sensing. The 2 304 000 target is only available above the
> custom trigger; see [THERMAL-MIUI14.md](THERMAL-MIUI14.md).

| Key | Nilai draft | Alasan |
|---|---|---|
| `policy0.scaling_governor` | `schedutil` | |
| `policy0.scaling_min_freq` | `1612800` | floor |
| `policy0.scaling_max_freq` | `1804800` | max stock |
| `policy0.schedutil.hispeed_freq` | `1804800` | ramp ke max |
| `policy0.schedutil.hispeed_load` | `75` | |
| `policy0.schedutil.pl` | `1` | predicted load |
| `policy6.scaling_governor` | `schedutil` | |
| `policy6.scaling_min_freq` | `1843200` | floor |
| `policy6.scaling_max_freq` | `2304000` | max stock |
| `policy6.schedutil.hispeed_freq` | `2304000` | |
| `policy6.schedutil.hispeed_load` | `70` | |
| `policy6.schedutil.pl` | `1` | |
| `kernel.sched_conservative_pl` | `1` | |
| `policy0.core_ctl.min_cpus` | `6` | semua little online |
| `policy0.core_ctl.max_cpus` | `6` | |
| `policy0.core_ctl.busy_up_thres` | `35` | |
| `policy0.core_ctl.busy_down_thres` | `15` | |
| `policy0.core_ctl.offline_delay_ms` | `400` | |
| `policy0.core_ctl.task_thres` | `12` | legal (>=6) |
| `gpu.max_pwrlevel` | `0` | 800 MHz |
| `gpu.min_pwrlevel` | `3` | 430 MHz |
| `gpu.default_pwrlevel` | `4` | 355 MHz |
| `io.scheduler` | `deadline` | |
| `io.nr_requests` | `256` | |
| `io.read_ahead_kb` | `1024` | |
| `io.rq_affinity` | `2` | |
| `cpuset.background.cpus` | `0-1` | |
| `cpuset.system-background.cpus` | `0-1` | |
| `cpuset.foreground.cpus` | `0-7` | uji dulu; stock 0-2,4-7 |
| `cpuset.top-app.cpus` | `0-7` | |
| `stune.top-app.boost` | `10` | |
| `stune.top-app.prefer_idle` | `1` | |
| `kernel.sched_upmigrate` | `60` | |
| `kernel.sched_downmigrate` | `50` | |
| `kernel.sched_group_upmigrate` | `140` | |
| `kernel.sched_group_downmigrate` | `120` | |
| `kernel.sched_little_cluster_coloc_fmin_khz` | `1017600` | |
| `kernel.sched_latency_ns` | `6000000` | |
| `kernel.sched_min_granularity_ns` | `1000000` | |
| `kernel.sched_wakeup_granularity_ns` | `500000` | |
| `kernel.sched_migration_cost_ns` | `500000` | |
| `kernel.sched_nr_migrate` | `8` | |
| `kernel.sched_walt_rotate_big_tasks` | `0` | |
| `vm.watermark_scale_factor` | `10` | |
| `vm.vfs_cache_pressure` | `50` | |
| `vm.dirty_ratio` | `30` | |
| `vm.dirty_expire_centisecs` | `600` | |
| `vm.dirty_writeback_centisecs` | `1500` | |
| `devfreq l3 min` | `940800000` | |
| `cpu_boost.input_boost_freq` | `0:1804800 1:0 ...` | uji; hanya freq yang boleh |
| `net.tcp_slow_start_after_idle` | `0` | |

### Sleep (screen off)

| Key | Nilai draft | Alasan |
|---|---|---|
| `policy0.scaling_governor` | `schedutil` | |
| `policy0.scaling_min_freq` | `576000` | floor screen-off |
| `policy0.scaling_max_freq` | `1017600` | cap |
| `policy0.schedutil.hispeed_freq` | `768000` | |
| `policy0.schedutil.hispeed_load` | `95` | |
| `policy0.core_ctl.min_cpus` | `1` | |
| `policy0.core_ctl.max_cpus` | `2` | |
| `policy0.core_ctl.busy_up_thres` | `85` | |
| `policy0.core_ctl.busy_down_thres` | `65` | |
| `policy0.core_ctl.offline_delay_ms` | `40` | |
| `policy6.scaling_governor` | `schedutil` | |
| `policy6.scaling_min_freq` | `652800` | floor off |
| `policy6.scaling_max_freq` | `1324800` | cap |
| `policy6.schedutil.hispeed_freq` | `1094400` | |
| `policy6.schedutil.hispeed_load` | `95` | |
| `gpu.max_pwrlevel` | `5` | |
| `gpu.min_pwrlevel` | `6` | |
| `io.scheduler` | `cfq` | |
| `io.nr_requests` | `32` | |
| `io.read_ahead_kb` | `128` | |
| `io.nomerges` | `1` | |
| `io.iostats` | `0` | |
| `io.cfq.fifo_expire_async` | `500` | |
| `io.cfq.slice_async` | `60` | |
| `io.cfq.low_latency` | `0` | |
| `vm.dirty_ratio` | `25` | |
| `vm.dirty_background_ratio` | `10` | |
| `vm.dirty_expire_centisecs` | `400` | |
| `vm.dirty_writeback_centisecs` | `1000` | |
| `vm.stat_interval` | `10` | |
| `kernel.sched_upmigrate` | `90` | |
| `kernel.sched_downmigrate` | `50` | |
| `kernel.sched_latency_ns` | `16000000` | |
| `kernel.sched_min_granularity_ns` | `4000000` | |
| `kernel.sched_wakeup_granularity_ns` | `3000000` | |
| `kernel.sched_migration_cost_ns` | `100000000` | |
| `kernel.sched_nr_migrate` | `8` | |
| cpuset | stock MIUI 14 | |
| `cpu_boost.input_boost_freq` | all 0 | |
| `devfreq l3 min` | `300000000` | |

### Boost (transient) — removed in 0.2

| Key | Nilai draft | Alasan |
|---|---|---|
| `policy0.scaling_min_freq` | `1612800` | floor |
| `policy0.scaling_max_freq` | `1804800` | |
| `policy0.schedutil.hispeed_freq` | `1804800` | |
| `policy0.schedutil.hispeed_load` | `75` | |
| `policy0.schedutil.pl` | `1` | |
| `policy0.core_ctl.min_cpus` | `6` | |
| `policy6.scaling_min_freq` | `1843200` | floor |
| `policy6.scaling_max_freq` | `2304000` | |
| `policy6.schedutil.hispeed_freq` | `2304000` | |
| `policy6.schedutil.hispeed_load` | `70` | |
| `gpu.max_pwrlevel` | `0` | |
| `gpu.min_pwrlevel` | `3` | |
| `stune.top-app.boost` | `10` | |
| `stune.top-app.prefer_idle` | `1` | |
| `cpuset.top-app.cpus` | `0-7` | |
| `kernel.sched_group_upmigrate` | `140` | |
| `kernel.sched_group_downmigrate` | `120` | |
| `kernel.sched_little_cluster_coloc_fmin_khz` | `1017600` | |
| `policy0/6.schedutil.up/down_rate_limit_us` | `0` | |

Boost pada MIUI 14 tidak bisa menaikkan floor di atas 1.6/1.8; efeknya lebih
kecil daripada di MIUI 12. Pertimbangkan untuk menonaktifkan fitur ini di v1
atau menjadikannya eksperimen.

## Aturan validasi baru

- `FreqMax`: `resolved >= live_min` wajib; jika tidak, `Locked("max < effective min")`.
- `FreqMin`: readback `>= wanted` = framework wins (bukan drift); readback
  `< wanted` = drift.
- `core_ctl.task_thres` >= 6 untuk little; jangan tambahkan big core_ctl.
- `gpu.max_pwrlevel <= gpu.min_pwrlevel` tetap.
- `io.scheduler` readback hanya bracket; write token.
- `cpu_boost.input_boost_freq` boleh; `powerkey_*`/`input_boost_ms` forbidden.

## Yang harus diuji di device sebelum final

1. Setiap profil penuh: `plan` -> `apply` -> `verify` -> `restore`.
2. Cek apakah `scaling_max_freq` gagal saat transisi screen-on/off.
3. Cek drift setelah app launch/game/thermal; berapa lama sampai watchdog.
4. Cek cpuset game (0-7 foreground) apakah memengaruhi stabilitas/thermal.
5. Cek `cpu_boost.input_boost_freq` game/powersave effect.
6. Cek refresh SF 1035 + settings key; apakah MIUI UI konsisten.
7. Cek profil sleep saat screen off benar-benar turun dan tidak bentrok
   dengan `msm_performance` release.
