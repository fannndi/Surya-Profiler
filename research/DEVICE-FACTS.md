# Device facts — Surya / MIUI 14 / Android 12

Dikumpulkan dari device via adb root. Semua nilai adalah observasi langsung
(2026-11-19), bukan asumsi dari MiFineTune.

## Identitas

| Item | Nilai |
|---|---|
| Device | POCO X3 NFC, `surya`, model `M2007J20CG` |
| SoC | Qualcomm `sm6150` / platform `SDMMAGPIE`, soc_id `365` |
| ROM | MIUI 14 (`ro.miui.ui.version.name=V140`, code 14) |
| Build | `V14.0.2.0.SJGIDXM`, fingerprint `POCO/surya_id/surya:12/...` |
| Android | 12 / SDK 31 / security patch 2023-06-01 |
| Kernel | `4.14.190-perf-g75201eeb4293` (aarch64, clang 10.0.7) |
| SELinux | Enforcing |
| Root | APatch: `/data/adb/apd`, `/data/adb/ap`; `su -c id` -> uid=0 |
| su context | tampak `u:r:shell:s0` / transisi APatch `u:r:magisk:s0`; write sysfs berhasil |

## CPU

- 8 core, 2 cluster: **6 little (cpu0-5) + 2 big (cpu6-7)**.
- `policy0` (little): governor `schedutil`, driver `osm-cpufreq`.
  - OPP: `300000 576000 768000 1017600 1248000 1324800 1497600 1612800 1708800 1804800`
  - `cpuinfo_min/max`: 300000 / 1804800
- `policy6` (big): governor `schedutil`, driver `osm-cpufreq`.
  - OPP: `300000 652800 806400 979200 1094400 1209600 1324800 1555200 1708800 1843200 1939200 2169600 2208000 2304000`
  - `cpuinfo_min/max`: 300000 / 2304000
- `schedutil` stock: hispeed little `1248000@90`, big `1324600@85`; `pl=0`,
  `up/down_rate_limit_us=0`. Tidak ada file `conservative_pl` di schedutil;
  `kernel.sched_conservative_pl` ada di `/proc/sys/kernel`.
- `core_ctl` ada di `/sys/devices/system/cpu/cpu0/core_ctl` dan
  `/sys/devices/system/cpu/cpu6/core_ctl` (bukan di bawah `cpufreq/policyN`).
  - little: `min=4 max=6 busy_up=60 busy_down=40 offline_delay=100 task_thres=8 enable=1`
  - big: `enable=0`; `task_thres=4294967295` (praktis disabled)
- `cpu_boost` stock: `input_boost_freq=0:1324800 ...`, `input_boost_ms=120`,
  `powerkey_input_boost_freq=4:1804800 7:2208000`, `powerkey_input_boost_ms=400`.

## Runtime CPU floor (sangat penting)

> **Koreksi 2026-11-21:** nilai 1.6128/1.8432 GHz di bawah adalah kondisi
> saat **wake/touch boost** (wake ~3 dtk, touch blip ~1 dtk), bukan konstanta
> screen-on. Sesi adb-driven bisa floor-less. Selalu baca
> `scaling_min_freq` live; jangan hardcode floor.

Saat boost aktif, `msm_performance` menulis:

```
cpu_min_freq = 0:1612800 1:1612800 ... 5:1612800 6:1843200 7:1843200
```

sehingga `scaling_min_freq` efektif = **1612800 / 1843200**. Saat screen off:

```
cpu_min_freq = 0:0 ... 7:0
scaling_min_freq = 576000 / 652800
```

`msm_performance` adalah prefix forbidden (perf HAL/PowerKeeper); jangan ditulis.
Konsekuensi: menulis `scaling_min_freq` di bawah floor saat screen-on akan
dibaca kembali sebagai floor (bukan error; framework menang).

## Cgroup / scheduler

- Hybrid cgroup v1 + cgroup2:
  - v1: `/dev/cpuset` (cpuset), `/dev/cpuctl` (cpu), `/dev/stune` (schedtune),
    `/dev/blkio`, `/dev/memcg`.
  - v2: `/sys/fs/cgroup` (freezer; `frozen/` + `unfrozen/` dibuat oleh
    `millet_monitor` saat boot).
- `/dev/cpuset`: `background`, `system-background`, `foreground`, `top-app`,
  `game`, `gamelite`, `vr`, `audio-app`, `camera-daemon`, `restricted`.
- Stock cpuset:
  - `background=0-2`
  - `system-background=0-3`
  - `foreground=0-2,4-7`
  - `foreground/boost=4-7`
  - `top-app=0-7`
- `/dev/stune`: `top-app`, `foreground`, `background`, `rt`, `audio-app`.
  Stock `top-app.boost=0`, `prefer_idle=0`, `foreground.boost=0`,
  `foreground.prefer_idle=0`.

## GPU (Adreno 618 / kgsl)

- `devfreq` freqs: `180 267 355 430 565 650 800 MHz` (pwrlevel 0=800, 6=180).
- Stock `max_pwrlevel=0`, `min_pwrlevel=6`, `default_pwrlevel=6`,
  governor `msm-adreno-tz`.
- `idle_timer=80`, `force_no_nap=1`, `force_clk_on=0`, `force_rail_on=0`
  (perf HAL-owned; jangan ditulis).

## Storage / IO

- Block utama: `/sys/block/sda`; scheduler stock `cfq`
  (`noop deadline [cfq]`).
- `nr_requests=128`, `nomerges=0`, `iostats=1`, `rq_affinity=1`,
  `read_ahead_kb=512`.
- CFQ stock: `quantum=8`, `fifo_expire_sync=125`, `fifo_expire_async=250`,
  `slice_sync=100`, `slice_async=40`, `slice_idle_us=0`,
  `target_latency_us=300000`, `group_idle=8`, `low_latency=1`.
- f2fs userdata: `/sys/fs/f2fs/sda16`; `gc_urgent=0`,
  `gc_urgent_sleep_time=500`.
- `vfs_cache_pressure=100`, `dirty_ratio=20`,
  `dirty_background_ratio=5`, `dirty_expire_centisecs=200`,
  `dirty_writeback_centisecs=500`, `stat_interval=1`,
  `watermark_scale_factor=1`, `extra_free_kbytes=30375`.

## Battery / charging

- `battery`: status Charging (selama audit), `capacity=88..99`,
  `current_now` negatif = charging, positif = discharging.
- `charge_full=5111000` µAh (5111 mAh), `cycle_count=34`,
  `health=Good`, `temp=324` (0.1 °C).
- `charge_full_design` rusak/negatif (`-414902457`); `bms` melaporkan
  `charge_full_design=5216000` (5216 mAh).
- Node user-facing yang bisa ditulis:
  - `battery/input_suspend` (0/1) — bypass charging; terverifikasi:
    `1` -> status Discharging, current positif; `0` -> Charging.
  - `battery/battery_charging_enabled` (0/1) — charge guard; terverifikasi:
    `0` -> current positif walau status masih "Charging"; `1` -> charging.
  - `charge_control_limit` (0..16), `constant_charge_current`,
    `constant_charge_current_max`, `charge_term_current`, dll. — tidak dipakai.
- MIUI 14 build ini **tidak** menunjukkan UI charge-limit bawaan; tidak ada
  setting key charge limit yang terlihat.

## Thermal

- 82 thermal zone; yang relevan:
  - `thermal_message/board_sensor_temp` (VIRTUAL-SENSOR; stock ~38.5 °C saat idle)
  - `quiet-therm-adc` (battery/LCD monitor; ~35.7 °C)
  - `cpu-0-0-usr` .. `cpu-0-5-usr`, `cpu-1-0-usr` .. `cpu-1-3-usr`
  - `gpuss-0-usr`, `gpuss-1-usr`
  - `battery` (29.9 °C), `backlight_therm`
- `thermal_message/sconfig = 0` (normal); `temp_state=0`, `screen_state=1`.
  Launch game tidak mengubah `sconfig` (Game Turbo tidak memilih tgame).
- Config aktif (`/data/vendor/thermal/decrypt.txt`, plaintext):
  - `SS-CPU6`: sensor VIRTUAL-SENSOR (`nvm-therm-adc`), trig 50 °C,
    clr 48 °C, target 1209600
  - `MONITOR-BAT`: quiet-therm 38..45 °C -> battery current 201..1515 mA
  - `MONITOR-LCD`: backlight_therm 44/47 °C -> backlight 819/1230
  - `MONITOR-TEMP_STATE`: 60 °C -> 4
  - `MONITOR-BCL`: bat_soc 5% -> hotplug
  - `MONITOR-CCC_CTRL`: 61 °C -> hotplug cpu6/7 + backlight 2048
- `sconfig` map (verified): 0/1/2/11/14 -> normal (file high/extreme/class0/
  youtube tidak ada), 8 phone, 9/13/16 tgame, 10 nolimits, 12 camera,
  15 arvr.
- Live: `policy6 max` pernah 1209600 pada quiet ~49.7 °C (cdev8=8);
  `sconfig=10` melepas cpu6 ke 1843200.
- `/vendor/etc/thermal-*.conf` terenkripsi; tool `mi-thermal-crypt`
  (AES-128-CBC, key/IV `thermalopenssl.h`) menghasilkan ciphertext identik
  dengan vendor. Detail: [THERMAL-MIUI14.md](THERMAL-MIUI14.md).
- v0.4.0: custom takeover per Device Profile via encrypted `/data` configs
  (private `sconfig` 60–63; v0.4.x used 20–23), direct `nvm-therm-adc` SS sensor, stock charging
  ladder preserved, Service OFF restore.

## MIUI stack

| Komponen | Status |
|---|---|
| `vendor.qti.hardware.perf@2.2-service` | running (perf HAL 2.2) |
| `perfservice` / `vendor.perfservice` | running |
| `mi_thermald` | running |
| `mcd` | running (`/system/bin/mcd`, config `/system/etc/mcd_default.conf`) |
| `millet_monitor` | 3 proses: `SIG`, `BINDER`, `PKG`; mengelola cgroup2 freezer |
| `miuibooster` | running (`com.miui.performance.IMiuiBoosterManager`) |
| `com.miui.powerkeeper` | running; PowerKeeper 4.2.00; `mPowerMode=2` (balanced) |
| `com.qualcomm.qti.performancemode` | package ada, tidak launchable |
| `com.xiaomi.joyose` | **tidak ada** |
| `com.xiaomi.gamecenter` / `glgm` | **tidak ada** |
| `com.xiaomi.misettings` | 2.9.9.17 |
| `com.miui.securitycenter` | 11.1.6 |

PowerKeeper dump menunjukkan `mPowerModeHandler enabled`, `powerMode=2`,
dan `msm_performance cpu_min_freq` berubah mengikuti screen state. Jadi floor
screen-on adalah bagian dari power policy MIUI, bukan sesuatu yang boleh
ditimpa oleh aplikasi tuning.
