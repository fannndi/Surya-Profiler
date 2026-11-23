# Thermal, katalog & desain profil — evidence

Bukti thermal MIUI 14, status katalog, dan desain profil, plus
matriks nilai referensi dan overlap resource perf HAL.


---

## THERMAL-MIUI14.md

## Thermal — MIUI 14 `mi_thermald` (surya)

Device: **POCO X3 NFC / surya / sm6150**, ROM **V14.0.2.0.SJGIDXM**
(Android 12, SDK 31), kernel `4.14.190-perf-g75201eeb4293`, root APatch.

Reference ROM: `/home/<user>/Downloads/MIO-KITCHEN-4.2.1-linux/MIUI14-ID-07`
(extracted). Build fingerprint:
`POCO/surya_id/surya:12/RKQ1.211019.001/V14.0.2.0.SJGIDXM:user/release-keys`.

`thermal-normal.conf`, `thermal-tgame.conf`, `thermal-nolimits.conf` and
`thermal-map.conf` in the extracted ROM are **md5-identical** to `/vendor/etc`
on the device. This ROM is the exact reference for the installed build.

> Scope: **v0.4.0** uses the runtime override below for four custom profiles.
> `temp_state` is never written. Stock MIUI remains the fallback and MIUI
> camera/phone/ARVR scenarios are yielded to.

### 1. Thermal stack

- `/vendor/bin/mi_thermald` runs as root (`u:r:mi_thermald:s0`), service
  defined in `vendor/etc/init/hw/init.target.rc` lines 91–95.
- `thermal-engine` is **not used**: `vendor/etc/thermal-engine.conf` is a
  comment-only file and the service block is commented out
  (`init.target.rc` line 310).
- `mi_thermald` reads encrypted configs from `/vendor/etc/thermal-*.conf`;
  `/data/vendor/thermal/decrypt.txt` is its decrypted working copy.
- Permissions (init.target.rc lines 65–68):
  - `thermal_message/sconfig` → `0664 system:system`
  - `thermal_message/temp_state` → `0666 system:system`

### 2. Encryption / decryption

`mi_thermald` uses **AES-128-CBC** with a fixed key and IV:

```
key = first 16 bytes of "thermalopenssl.h"
iv  = first 16 bytes of "thermalopenssl.h"
```

Tool: `/home/<user>/Documents/Surya-Profiler/Reference/mi-thermal-crypt/mi-thermal-crypt`
(the app implementation is `core/src/engine/crypt.rs`; the reference
`Reference/vtools_en/mi-thermal-config` uses the same path/format).

```bash
## decrypt (no -e)
mi-thermal-crypt -i thermal-normal.conf -o thermal-normal.txt

## encrypt (mi_thermald only accepts encrypted configs)
mi-thermal-crypt -i thermal-normal.txt -o thermal-normal.conf -e
```

Verified 2026-11-20:

- decrypt → edit → encrypt → decrypt round-trips byte-for-byte;
- encrypting the plaintext of `thermal-normal.conf` produces a file
  **byte-identical** to the vendor blob (`md5 ab7e18d1…`).

### 3. Config inventory

| File | Size (encrypted) | Role |
|---|---:|---|
| `thermal-normal.conf` | 1024 B | default screen-on scenario |
| `thermal-tgame.conf` | 1152 B | MIUI game scenario |
| `thermal-phone.conf` | 1232 B | call / phone scenario |
| `thermal-camera.conf` | 1408 B | camera scenario |
| `thermal-4k.conf` | 1184 B | 4K recording |
| `thermal-arvr.conf` | 976 B | AR/VR |
| `thermal-nolimits.conf` | 960 B | benchmark / no-limits |
| `thermal-chg-only.conf` | 419 B | **plaintext**, charging-only fallback |
| `thermal-map.conf` | 304 B | `sconfig` → config map |
| `thermald-devices.conf` | 384 B | cooling-device name map |

`thermald-devices.conf` maps:

| Thermal name | Cooling device |
|---|---|
| `gpu` | `thermal-devfreq-0` |
| `battery` | `battery` |
| `backlight` | `panel0-backlight` |
| `temp_state` | `/sys/class/thermal/thermal_message/temp_state` |

### 4. `sconfig` map (verified live)

`cat /sys/class/thermal/thermal_message/sconfig` selects the scenario. Writing
each value and reading `/data/vendor/thermal/decrypt.txt` gave:

| `sconfig` | Config loaded | Notes |
|---:|---|---|
| 0 | `thermal-normal` | current device value |
| 1 | `thermal-normal` | `thermal-high.conf` missing → fallback |
| 2 | `thermal-normal` | `thermal-extreme.conf` missing → fallback |
| 8 | `thermal-phone` | |
| 9 | `thermal-tgame` | |
| 10 | `thermal-nolimits` | |
| 11 | `thermal-normal` | `thermal-class0.conf` missing → fallback |
| 12 | `thermal-camera` | |
| 13 | `thermal-tgame` | |
| 14 | `thermal-normal` | `thermal-youtube.conf` missing → fallback |
| 15 | `thermal-arvr` | |
| 16 | `thermal-tgame` | |

The daemon resolves the same map in
`core/src/engine/env/::thermal_mode_name`.

### 5. Threshold tables

All temperatures are millidegrees Celsius. `trig` = throttle threshold,
`clr` = clear threshold (hysteresis).

#### `thermal-normal.conf` (default)

| Algorithm | Sensor | Device | Thresholds | Target |
|---|---|---|---|---|
| `SS-CPU6` | `VIRTUAL-SENSOR` (`nvm-therm-adc`) | `cpu6` | 50 °C / clear 48 °C | 1 209 600 kHz |
| `MONITOR-BAT` | `quiet-therm-adc` | `battery` | 38…45 °C ladder | 201…1515 |
| `MONITOR-LCD` | `backlight_therm` | `backlight` | 44 / 47 °C | 819 / 1230 |
| `MONITOR-TEMP_STATE` | `quiet-therm-adc` | `temp_state` | 60 °C / clear 58 °C | 4 |
| `MONITOR-BCL` | `bat_soc` | `hotplug_cpu0+hotplug_cpu1` | 5 % (clear 6 %) | 1+1 |
| `MONITOR-CCC_CTRL` | `quiet-therm-adc` | `hotplug_cpu6+hotplug_cpu7+backlight` | 61 °C / clear 58 °C | 1+1+2048 |

#### `thermal-tgame.conf`

Same `SS-CPU6` as normal (50 °C → 1 209 600), plus:

| Algorithm | Sensor | Device | Thresholds | Target |
|---|---|---|---|---|
| `SS-CPU0` | `VIRTUAL-SENSOR` | `cpu0` | 53 °C / clear 51 °C | 1 324 800 kHz |
| `MONITOR-LCD` | `backlight_therm` | `backlight` | 40 / 44 / 47 °C | 410 / 819 / 1230 |

Battery, `temp_state`, BCL and CCC are identical to normal.

#### `thermal-nolimits.conf`

| Algorithm | Sensor | Device | Thresholds | Target |
|---|---|---|---|---|
| `NL-SS-CPU6` | `VIRTUAL-SENSOR` | **`cpu7`** | 51 °C / clear 50 °C | 1 094 400 kHz |
| `NL-MONITOR-GPU` | `VIRTUAL-SENSOR` | `gpu` | 51 °C / clear 48 °C | 2 |
| `NL-MONITOR-BOOST_LIMIT` | `VIRTUAL-SENSOR` | `boost_limit` | 51 °C / clear 48 °C | 1 |
| `NL-MONITOR-TEMP_STATE` | `VIRTUAL-SENSOR` | `temp_state` | 60 °C / clear 50 °C | 5 |
| `NL-MONITOR-BAT` | `VIRTUAL-SENSOR` | `battery` | 35 / 39 / 42 / 60 °C | 900 / 1002 / 1408 / 1515 |
| `NL-MONITOR-BCL` | `bat_soc` | `hotplug_cpu6+hotplug_cpu7` | 5 % | 1+1 |

There is **no LCD monitor and no CCC** in this config. The kernel LMH/BCL
zones (`lmh-dcvs-*`, `pm6150-bcl-*`) and battery/`temp_state` limits remain,
but the userspace backlight/core-offline safety net is absent.

#### Other scenarios

| Config | CPU caps | GPU | LCD | Notes |
|---|---|---|---|---|
| `phone` | cpu6 33 °C → 1 324 800; 54 °C → 979 200 | 43/45/48 °C → 0/1/2 | — | battery 25 °C → 1414, 60 °C → 1515; CCC 55 °C |
| `camera` | cpu0 52 °C → 1 209 600; cpu6 50 °C → 979 200 | 50 °C → 2 | 40/44/47 °C | hotplug cpu6/7 at 52 °C |
| `4k` | cpu6 38 °C → 1 843 200; 44 °C → 1 555 200; 51 °C → 1 094 400 | 53 °C → 2 | — | CCC 55 °C offlines cpu5/6/7 |
| `arvr` | cpu6 11 °C → 1 497 600 | 11 °C → 2 | — | very aggressive; not used by the app |
| `chg-only` | cpu6 51 °C → 979 200 | — | — | plaintext; not in `sconfig` map |

### 6. Live device observations (2026-11-20)

All observations were taken with the service **OFF** and `sconfig=0` unless
stated. Values move with load/perf HAL; treat them as snapshots, not constants.

- `sconfig=0`, `temp_state=0`, `screen_state=1`.
- Launching Epic Seven (`com.stove.epic7.google`) did **not** change
  `sconfig`; it stayed 0. MIUI Game Turbo did not select `tgame` on this build.
- Under warmth (`quiet-therm-adc ≈ 49.7 °C`):
  - `policy6/scaling_max_freq = 1 209 600`
  - `cooling_device8` (`thermal-cpufreq-6`) `cur_state = 8`
  This is the normal `SS-CPU6` cap in action.
- Writing `sconfig=10` (`nolimits`):
  - `policy6/scaling_max_freq` rose to **1 843 200**
  - `cooling_device8 cur_state` → 0
  - `cooling_device9` (`thermal-cpufreq-7`) → 4
  - `decrypt.txt` size/content switched to `thermal-nolimits`
- Writing `sconfig=9` (`tgame`):
  - `policy0/scaling_max_freq = 1 324 800`
  - `policy6` released to 1 843 200 in that snapshot
- Reverting `sconfig=0` restored `thermal-normal` and the original caps.

#### Runtime override (shipped in v0.4.0)

`mi_thermald` also reads overrides from:

```
/data/vendor/thermal/config/thermal-map.conf
/data/vendor/thermal/config/<config-name>
```

Both files must be **encrypted**. Verified with a harmless change
(`polling 1000 → 1200`):

1. encrypt map + config;
2. atomic `mv` into `/data/vendor/thermal/config/`;
3. write `sconfig=1`, then `sconfig=0` (forces a reload);
4. `decrypt.txt` showed `polling 1200`;
5. remove the files, repeat the `sconfig` toggle → `polling 1000` back.

Caveats:

- plaintext map/config produced a 0-byte `decrypt.txt` (parse failure);
- `mi_thermald` re-reads on `sconfig` change, not instantly on file creation;
- v0.4.0 installs these files only with its `.suryaprofiler` marker, removes
  them on Service OFF, and repairs leftovers with `surya-prof thermal-clean`;
- the custom map keeps every stock entry; `0` always resolves to vendor
  `thermal-normal.conf`, so a leftover map cannot silently apply a custom
  profile after a crash/reboot.

### 7. What this means for the profiles

- `thermal-normal` targets **1 209 600 kHz** once its virtual sensor crosses
  50 °C, but the screen-on big-cluster floor is **1 843 200 kHz**; the kernel
  clamps the target to the floor. The lower target is therefore not a usable
  sustained frequency while the screen is on.
- The Game profile's 2 304 000 kHz target is sustainable only above the
  thermal trigger; below it the floor/thermal state decides the effective
  cap. v0.4.0 therefore uses floor-aware custom steps (1.9392/2.1696 GHz),
  not sub-floor targets that can never read back.
- `tgame` is not a performance upgrade for sustained CPU: it keeps the same
  cpu6 cap and adds a little-core cap plus earlier LCD dimming.
- `nolimits` releases cpu6 but caps cpu7/GPU and removes the CCC/LCD safety
  net; it is MIUI's benchmark scenario, not a recommended daily driver.
- `temp_state` is an **output** (thermal severity for PowerKeeper/UI), not a
  scenario selector. Do not write it.
- The app's role is to tune within MIUI's thermal policy; the ROM's thresholds
  are the ceiling, not something Surya Profiler overrides.

### 8. Reproduction commands

```bash
## decrypt all reference configs
for f in thermal-normal thermal-tgame thermal-phone thermal-camera \
         thermal-4k thermal-arvr thermal-nolimits; do
  mi-thermal-crypt -i "$f.conf" -o "$f.txt"
done

## live scenario / caps (root)
adb shell su -c 'cat /sys/class/thermal/thermal_message/sconfig'
adb shell su -c 'cat /sys/class/thermal/thermal_message/temp_state'
adb shell su -c 'cat /data/vendor/thermal/decrypt.txt'
adb shell su -c 'cat /sys/devices/system/cpu/cpufreq/policy6/scaling_max_freq'
adb shell su -c 'for c in /sys/class/thermal/cooling_device*; do
  [ "$(cat $c/type)" = thermal-cpufreq-6 ] && cat $c/cur_state $c/max_state; done'
```

### 9. Manual R&D cleanup state (before app takeover)

- Service OFF, daemon not running.
- `sconfig=0`, `temp_state=0`, `screen_state=1`.
- `/data/vendor/thermal/decrypt.txt` = 1016 bytes, `thermal-normal`.
- `/data/vendor/thermal/config/` empty; no override files left.
- `mi_thermald` running; vendor configs untouched.
- No reboot required.

### 10. App telemetry mapping (v0.5.0)

`core/src/engine/env/::EnvSnapshot` (read-only) exposes:

| JSON field | Source | Diagnostics label |
|---|---|---|
| `thermal_sconfig` | `thermal_message/sconfig` | Mode |
| `thermal_mode` | resolved map above | Mode |
| `thermal_scenario` | PowerKeeper ID/offset decoder | Scenario |
| `thermal_global_mode` / `thermal_perf_config` | `/data/vendor/thermal/*`, `/data/property/*` | MIUI perf |
| `thermal_temp_state` / `thermal_screen_state` | `thermal_message/*` | State |
| `quiet_temp_c` / `nvm_temp_c` | thermal zones | Quiet / NVM |
| `cpu6_cooling` / `cpu7_cooling` (+ max) | `cooling_device*` | CPU6/CPU7 cooling |
| `little_max_mhz` / `big_max_mhz` | `policy0/6/scaling_max_freq` | Policy max |

`diag` adds the newest 12 parsed events from
`/data/vendor/thermal/thermal.dump` and `last_thermal.dump`; the Diagnostics
screen renders them in the “MIUI thermal events” card.

`doctor` adds `env_thermal_config`, `env_thermal_caps`, `thermal_takeover`,
`miui_thermal_mode`, `thermal_log`, `battery_whitelist`,
`appops_background`, and `millet_frozen`. The values are sampled on the normal
30 s env cadence; telemetry itself is read-only. The takeover path writes only
`sconfig` + marker-owned config files, never `temp_state`.

### 11. Surya custom takeover (v0.5.0)

The daemon (`core/src/daemon/bridge/thermal/`) installs four encrypted
configs and a custom map under `/data/vendor/thermal/config` only while the
service is on:

| Profile | `sconfig` | Config |
|---|---:|---|
| balance | 60 | `thermal-balance.conf` |
| game | 61 | `thermal-game.conf` |
| powersave | 62 | `thermal-powersave.conf` |
| sleep | 63 | `thermal-sleep.conf` |

The map is **generated** from `/vendor/etc/thermal-map.conf` (decrypted with
the same AES key), so new stock entries survive MIUI updates. Stock entries
(`0` normal, `8` phone, `9/13/16` tgame, `10` nolimits, `12` camera,
`15` arvr) are preserved; `17`/`18` are added as 4K/camera fallbacks when the
vendor map lacks them. The four private entries are appended last.

#### Why 60–63, not 20–23

PowerKeeper's `ThermalSwitchHandler` uses base IDs 0–25 and adds offsets:

| Offset | Meaning |
|---:|---|
| `+30` | power-mode performance |
| `+100` | foreground / display not folded |
| `+200` | IEC-All48 |
| `+500` | IEC monitor |

Its `THERMAL_ID_*` constants include:

| ID | Scenario | ID | Scenario |
|---:|---|---:|---|
| 0 | default | 13 | pubg (tgame) |
| 1 | performance | 14 | youtube |
| 2 | extreme | 15 | arvr |
| 3 | huanji | 16 | game2 |
| 4 | videochat | 17 | camera-4k |
| 5 | abnormal | 18 | camera-8k |
| 6 | nightvideo | 19 | navigation |
| 7 | dolbyvision | 20 | game-normal |
| 8 | incall | 21 | video |
| 9 | game | 22 | demo |
| 10 | benchmark | 23 | yuanshen |
| 11 | class0 | 24 | sptm |
| 12 | camera | 25 | max |

The device's PowerKeeper dump maps `com.google.android.youtube=21`,
`com.android.camera=12`, `com.xiaomi.mihomemanager=22`, benchmark apps=10.
Using 20–23 for our custom profiles could therefore apply the wrong custom
thermal (e.g. YouTube → game) or mask a MIUI yield. 60–63 is outside every
base ID and `+30`/`+100`/`+200`/`+500` offset, and it is below
`mi_thermald`'s map parser limit (`ID > 64` is rejected — verified on device
2026-11-20). The daemon refuses to install if a future vendor map already
uses 60–63.

#### Yield policy (v0.5.0)

- **Yield:** camera `12/17/18`, phone `8`, ARVR `15`, performance mode `1`
  (or `thermal-global-mode=1` / `persist.sys.thermal.config=1`).
- **Keep custom:** video `21`, game `9/13/16`, benchmark `10`, navigation
  `19`, game-normal `20`, yuanshen `23`, unknown/offset values. This keeps
  YouTube→Power Save and Game→Game effective.
- **Unknown IDs:** `mi_thermald` keeps the last config (`use config file
  which last set`). Service OFF therefore writes `0` before removing the
  map when the live ID has no vendor map entry.
- **Default:** `0` and `100` (default + foreground) re-assert the active
  custom profile.

#### Other constraints

- **Direct sensor**: `SS`/GPU steps use `sensor nvm-therm-adc` directly.
  `VIRTUAL-SENSOR` sections are rejected by `check_sensor_valid` on this
  boot (`remove [...]`), so they cannot drive the cpufreq cooling device.
- **Screen-on floors**: big `1 843 200` kHz and little `1 612 800` kHz. A
  thermal target below the floor is clamped; final steps use
  `2 169 600 / 1 939 200 / 1 843 200` (big) and floor-level little caps.
- **Stock charging ladder**: every config copies `MONITOR-BAT` exactly
  (`201..1515`). This is MIUI's thermal charging behavior, not a new charge
  control; the app never writes `battery_charging_enabled` or
  `input_suspend` for thermal.
- **Safety net**: `MONITOR-TEMP_STATE`, `MONITOR-BCL` and
  `MONITOR-CCC_CTRL` are kept. `CCC` at 61 °C is untouched for
  balance/game; powersave/sleep use earlier CCC thresholds.
- **Cleanup**: marker-owned files only. Service OFF restores the captured
  `sconfig` and removes the files; a failure gates the stop. Cleanup accepts
  both the 20–23 legacy map and the 60–63 map.
- **Migration**: a v0.4.x takeover (`sconfig 20–23`, marker present) is
  rewritten to 60–63 on the first sync after the daemon starts.

### 12. Runtime telemetry (v0.5.0)

- `env` now includes `thermal_scenario` (decoded base + flags),
  `thermal_global_mode` (`/data/vendor/thermal/thermal-global-mode`) and
  `thermal_perf_config` (`/data/property/persist.sys.thermal.config`).
- `diag` includes the newest 12 events parsed from
  `/data/vendor/thermal/thermal.dump` and `last_thermal.dump`. Format:
  `MM-DD HH:MM:SS[ALGO][sensor value][device value]`.
- `doctor` reports `thermal_takeover`, `miui_thermal_mode`, `thermal_log`,
  and informational `battery_whitelist`, `appops_background`,
  `millet_frozen` checks.


---

## CATALOG-STATUS.md

## Catalog status — 95 entri referensi di MIUI 14

> **0.2 slim build:** 3 entries were removed from the active catalog
> (`charge.battery_charging_enabled` and the two f2fs GC nodes), leaving
> **92**. The remaining charge node is `charge.input_suspend` (R&D only).
> This document stays as the 95-entry R&D reference.
>
> **2026-11-23 guard additions (live-verified):** no new catalog nodes; three
> forbidden guards added from the reference-repo audit
> ([REFERENCES-2026-11-23.md](HISTORY.md)) —
> `sched_boost_top_app` + `sched_prefer_sync_wakee_to_waker` (absent on
> kernel 4.14.190, future-proof) and UFS
> `1d84000.ufshc/{clkscale,clkgate,hibern8}_enable` (vendor
> `init.qcom.rc`-owned, fresh mtimes = active writer). The vendor thermal-map
> reader now tolerates plaintext maps (`decrypt_or_plain`).
>
> **2026-11-21 R&D:** additional SoC nodes were audited (write/readback/
> restore verified on device) but **not cataloged** — stock values are
> already optimal and no profile uses them: kgsl `ifpc`/`bus_split`/
> `gpu_llc_slice_enable`/`gpuhtw_llc_slice_enable`/`preempt_level`,
> `devfreq/cpu*-cpu-l3-lat/max_freq`, `stune/background/*`,
> `cpu0/core_ctl/enable`. WALT nodes are absent on this kernel. Evidence
> and bounds: [RND-BATTERY-PERF-2026-11-21.md](HISTORY.md).
>
> **2026-11-22 R&D:** bus-DCVS floors (`cpu-cpu-llcc-bw`, `cpu-llcc-ddr-bw`,
> `cpu*-ddr-latfloor`, `cpu*-cpu-llcc-lat`, `cpu*-llcc-ddr-lat`, `gpubw`),
> `cpu.shares`, `sched_cstate_aware` and kgsl `popp` are writable and held
> for 62 s, but produced no measurable scroll/frame win; they stay
> documented and **uncataloged** (slim-build rule). The Game profile was
> replaced with a stock-CPU/sched/VM/cpuset variant (full GPU + touch boost
> + deadline I/O): −1.8 % power vs stock at 60 fps. Balance verified −4.4 %
> vs stock in Chrome scroll. Evidence:
> [RND-SOC-2026-11-22.md](HISTORY.md).

Sumber: bagian *catalog_miui14* di file ini (diprobe ulang di device),
bagian *perf_overlap* di file ini, ekstrak `init.qcom.post_boot.sh`
(arm moorea soc 365/366), dan `perfd_default_values` (arsip lokal).

### Ringkasan

- **95/95 path ada** di MIUI 14; tidak ada yang hilang.
- Permission: mayoritas `-rw-r--r-- root:root`; beberapa `system:system`
  (scaling min/max, charge nodes) dan `root:system` (cpuset foreground/top-app).
- Root APatch bisa menulis semua path yang diuji.
- Readback `io.scheduler` hanya token aktif dalam bracket; restore harus
  menulis token, bukan string readback.
- `core_ctl` ada di `cpu0/core_ctl` dan `cpu6/core_ctl`; big cluster
  `enable=0`, `task_thres=4294967295`.

### Perbedaan nilai baseline MIUI 14 vs asumsi referensi

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

### Tier / ownership

#### Perf HAL 2.2 overlap (Baseline)

22 path persis overlap dengan katalog (bagian *perf_overlap* di file ini):

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

#### Forbidden (framework-owned)

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

#### Free / boot baseline

- Semua path katalog lain dianggap Free atau boot-Baseline sesuai evidence
  `post_boot` arm moorea. Tidak ada runtime writer tambahan yang ditemukan
  pada audit ini selain perf HAL/PowerKeeper/thermal/mcd/millet.

### Catatan path

- `post_boot` menulis melalui alias `cpu0/cpufreq/...` dan `cpu6/cpufreq/...`;
  path efektif sama dengan `policy0`/`policy6`.
- `cpuset.foreground` stock **0-2,4-7** (cpu3 tidak dipakai foreground).
  Jangan menganggap ini typo tanpa bukti; profile yang mengubahnya harus diuji.
- `policy6.core_ctl.enable=0`; menulis min/max big tidak akan menghidupkan
  hotplug. Jangan masukkan core_ctl big ke profile.
- `charge_full_design` rusak; gunakan `bms.charge_full_design` (5216 mAh) atau
  spec 5160 mAh untuk wear estimate.


---

## PROFILE-DESIGN.md

> **0.2 slim build:** the shipped pack is Power Save, Balance, Game and the
> hidden Sleep profile. The Boost section below is kept as R&D history only.

## Profile design — constraint MIUI 14 dan kandidat awal

Status: **draft untuk R&D**; belum final dan belum diuji sebagai satu set
profil utuh. Nilai harus divalidasi lewat `plan/apply/verify` di device.

### Constraint baru yang mengikat semua profil

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

### Kandidat profil (draft)

Nilai di bawah adalah titik awal. Kolom "Alasan" menjelaskan intent, bukan
jaminan hasil.

#### Power Save

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

#### Balance

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

#### Game

> **Thermal reality (2026-11-20):** the screen-on big-cluster floor is
> **1 843 200 kHz**; sub-floor thermal targets are clamped. v0.4.0 uses
> floor-aware custom steps (`2 169 600`/`1 939 200`/`1 843 200`) and direct
> `nvm-therm-adc` sensing. The 2 304 000 target is only available above the
> custom trigger; see [THERMAL-MIUI14.md](EVIDENCE-THERMAL.md).

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

#### Sleep (screen off)

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

#### Boost (transient) — removed in 0.2

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

### Aturan validasi baru

- `FreqMax`: `resolved >= live_min` wajib; jika tidak, `Locked("max < effective min")`.
- `FreqMin`: readback `>= wanted` = framework wins (bukan drift); readback
  `< wanted` = drift.
- `core_ctl.task_thres` >= 6 untuk little; jangan tambahkan big core_ctl.
- `gpu.max_pwrlevel <= gpu.min_pwrlevel` tetap.
- `io.scheduler` readback hanya bracket; write token.
- `cpu_boost.input_boost_freq` boleh; `powerkey_*`/`input_boost_ms` forbidden.

### Yang harus diuji di device sebelum final

1. Setiap profil penuh: `plan` -> `apply` -> `verify` -> `restore`.
2. Cek apakah `scaling_max_freq` gagal saat transisi screen-on/off.
3. Cek drift setelah app launch/game/thermal; berapa lama sampai watchdog.
4. Cek cpuset game (0-7 foreground) apakah memengaruhi stabilitas/thermal.
5. Cek `cpu_boost.input_boost_freq` game/powersave effect.
6. Cek refresh SF 1035 + settings key; apakah MIUI UI konsisten.
7. Cek profil sleep saat screen off benar-benar turun dan tidak bentrok
   dengan `msm_performance` release.


---

## analysis/profile_matrix.md

## Matriks nilai profil MiFineTune (referensi) vs default MIUI 14

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


---

## analysis/perf_overlap.md

## Perf HAL 2.2 resource overlap

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

### Semua path perf HAL

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
