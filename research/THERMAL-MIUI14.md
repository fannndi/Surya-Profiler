# Thermal — MIUI 14 `mi_thermald` (surya)

Device: **POCO X3 NFC / surya / sm6150**, ROM **V14.0.2.0.SJGIDXM**
(Android 12, SDK 31), kernel `4.14.190-perf-g75201eeb4293`, root APatch.

Reference ROM: `/home/fannndi/Downloads/MIO-KITCHEN-4.2.1-linux/MIUI14-ID-07`
(extracted). Build fingerprint:
`POCO/surya_id/surya:12/RKQ1.211019.001/V14.0.2.0.SJGIDXM:user/release-keys`.

`thermal-normal.conf`, `thermal-tgame.conf`, `thermal-nolimits.conf` and
`thermal-map.conf` in the extracted ROM are **md5-identical** to `/vendor/etc`
on the device. This ROM is the exact reference for the installed build.

> Scope: **v0.4.0** uses the runtime override below for four custom profiles.
> `temp_state` is never written. Stock MIUI remains the fallback and MIUI
> camera/phone/ARVR scenarios are yielded to.

## 1. Thermal stack

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

## 2. Encryption / decryption

`mi_thermald` uses **AES-128-CBC** with a fixed key and IV:

```
key = first 16 bytes of "thermalopenssl.h"
iv  = first 16 bytes of "thermalopenssl.h"
```

Tool: `/home/fannndi/Documents/Surya-Profiler/Reference/mi-thermal-crypt/mi-thermal-crypt`
(the app implementation is `core/src/engine/crypt.rs`; the reference
`Reference/vtools_en/mi-thermal-config` uses the same path/format).

```bash
# decrypt (no -e)
mi-thermal-crypt -i thermal-normal.conf -o thermal-normal.txt

# encrypt (mi_thermald only accepts encrypted configs)
mi-thermal-crypt -i thermal-normal.txt -o thermal-normal.conf -e
```

Verified 2026-11-20:

- decrypt → edit → encrypt → decrypt round-trips byte-for-byte;
- encrypting the plaintext of `thermal-normal.conf` produces a file
  **byte-identical** to the vendor blob (`md5 ab7e18d1…`).

## 3. Config inventory

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

## 4. `sconfig` map (verified live)

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

## 5. Threshold tables

All temperatures are millidegrees Celsius. `trig` = throttle threshold,
`clr` = clear threshold (hysteresis).

### `thermal-normal.conf` (default)

| Algorithm | Sensor | Device | Thresholds | Target |
|---|---|---|---|---|
| `SS-CPU6` | `VIRTUAL-SENSOR` (`nvm-therm-adc`) | `cpu6` | 50 °C / clear 48 °C | 1 209 600 kHz |
| `MONITOR-BAT` | `quiet-therm-adc` | `battery` | 38…45 °C ladder | 201…1515 |
| `MONITOR-LCD` | `backlight_therm` | `backlight` | 44 / 47 °C | 819 / 1230 |
| `MONITOR-TEMP_STATE` | `quiet-therm-adc` | `temp_state` | 60 °C / clear 58 °C | 4 |
| `MONITOR-BCL` | `bat_soc` | `hotplug_cpu0+hotplug_cpu1` | 5 % (clear 6 %) | 1+1 |
| `MONITOR-CCC_CTRL` | `quiet-therm-adc` | `hotplug_cpu6+hotplug_cpu7+backlight` | 61 °C / clear 58 °C | 1+1+2048 |

### `thermal-tgame.conf`

Same `SS-CPU6` as normal (50 °C → 1 209 600), plus:

| Algorithm | Sensor | Device | Thresholds | Target |
|---|---|---|---|---|
| `SS-CPU0` | `VIRTUAL-SENSOR` | `cpu0` | 53 °C / clear 51 °C | 1 324 800 kHz |
| `MONITOR-LCD` | `backlight_therm` | `backlight` | 40 / 44 / 47 °C | 410 / 819 / 1230 |

Battery, `temp_state`, BCL and CCC are identical to normal.

### `thermal-nolimits.conf`

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

### Other scenarios

| Config | CPU caps | GPU | LCD | Notes |
|---|---|---|---|---|
| `phone` | cpu6 33 °C → 1 324 800; 54 °C → 979 200 | 43/45/48 °C → 0/1/2 | — | battery 25 °C → 1414, 60 °C → 1515; CCC 55 °C |
| `camera` | cpu0 52 °C → 1 209 600; cpu6 50 °C → 979 200 | 50 °C → 2 | 40/44/47 °C | hotplug cpu6/7 at 52 °C |
| `4k` | cpu6 38 °C → 1 843 200; 44 °C → 1 555 200; 51 °C → 1 094 400 | 53 °C → 2 | — | CCC 55 °C offlines cpu5/6/7 |
| `arvr` | cpu6 11 °C → 1 497 600 | 11 °C → 2 | — | very aggressive; not used by the app |
| `chg-only` | cpu6 51 °C → 979 200 | — | — | plaintext; not in `sconfig` map |

## 6. Live device observations (2026-11-20)

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

### Runtime override (shipped in v0.4.0)

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

## 7. What this means for the profiles

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

## 8. Reproduction commands

```bash
# decrypt all reference configs
for f in thermal-normal thermal-tgame thermal-phone thermal-camera \
         thermal-4k thermal-arvr thermal-nolimits; do
  mi-thermal-crypt -i "$f.conf" -o "$f.txt"
done

# live scenario / caps (root)
adb shell su -c 'cat /sys/class/thermal/thermal_message/sconfig'
adb shell su -c 'cat /sys/class/thermal/thermal_message/temp_state'
adb shell su -c 'cat /data/vendor/thermal/decrypt.txt'
adb shell su -c 'cat /sys/devices/system/cpu/cpufreq/policy6/scaling_max_freq'
adb shell su -c 'for c in /sys/class/thermal/cooling_device*; do
  [ "$(cat $c/type)" = thermal-cpufreq-6 ] && cat $c/cur_state $c/max_state; done'
```

## 9. Manual R&D cleanup state (before app takeover)

- Service OFF, daemon not running.
- `sconfig=0`, `temp_state=0`, `screen_state=1`.
- `/data/vendor/thermal/decrypt.txt` = 1016 bytes, `thermal-normal`.
- `/data/vendor/thermal/config/` empty; no override files left.
- `mi_thermald` running; vendor configs untouched.
- No reboot required.

## 10. App telemetry mapping (v0.5.0)

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

## 11. Surya custom takeover (v0.5.0)

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

### Why 60–63, not 20–23

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

### Yield policy (v0.5.0)

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

### Other constraints

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

## 12. Runtime telemetry (v0.5.0)

- `env` now includes `thermal_scenario` (decoded base + flags),
  `thermal_global_mode` (`/data/vendor/thermal/thermal-global-mode`) and
  `thermal_perf_config` (`/data/property/persist.sys.thermal.config`).
- `diag` includes the newest 12 events parsed from
  `/data/vendor/thermal/thermal.dump` and `last_thermal.dump`. Format:
  `MM-DD HH:MM:SS[ALGO][sensor value][device value]`.
- `doctor` reports `thermal_takeover`, `miui_thermal_mode`, `thermal_log`,
  and informational `battery_whitelist`, `appops_background`,
  `millet_frozen` checks.

