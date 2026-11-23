# Device evidence — surya / MIUI 14

Fakta device & perilaku runtime terkonsolidasi (dari sesi R&D
2026-10 → 2026-11; sumber asli per section). Pembacaan wajib sebelum
mengubah engine/katalog — lihat juga `AGENTS.md`.


---

## DEVICE-FACTS.md

## Device facts — Surya / MIUI 14 / Android 12

Dikumpulkan dari device via adb root. Semua nilai adalah observasi langsung
(2026-11-19), bukan asumsi dari MiFineTune.

### Identitas

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

### CPU

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

### Runtime CPU floor (sangat penting)

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

### Cgroup / scheduler

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

### GPU (Adreno 618 / kgsl)

- `devfreq` freqs: `180 267 355 430 565 650 800 MHz` (pwrlevel 0=800, 6=180).
- Stock `max_pwrlevel=0`, `min_pwrlevel=6`, `default_pwrlevel=6`,
  governor `msm-adreno-tz`.
- `idle_timer=80`, `force_no_nap=1`, `force_clk_on=0`, `force_rail_on=0`
  (perf HAL-owned; jangan ditulis).

### Storage / IO

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

### Battery / charging

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

### Thermal

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
  dengan vendor. Detail: [THERMAL-MIUI14.md](EVIDENCE-THERMAL.md).
- v0.4.0: custom takeover per Device Profile via encrypted `/data` configs
  (private `sconfig` 60–63; v0.4.x used 20–23), direct `nvm-therm-adc` SS sensor, stock charging
  ladder preserved, Service OFF restore.

### MIUI stack

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


---

## MIUI14-DELTAS.md

## MIUI 14 / Android 12 vs MiFineTune (MIUI 12 / Android 10)

Dokumen ini hanya mencatat perbedaan yang **berdampak pada desain**.
Referensi MiFineTune tetap dipakai untuk filosofi, bukan nilai/path mentah.

### 1. Runtime & framework

| Area | MiFineTune (MIUI 12/A10) | MIUI 14/A12 (device ini) | Dampak |
|---|---|---|---|
| Activity resume tags | `am_resume_activity`, `am_set_resumed_activity` | **`wm_resume_activity`, `wm_set_resumed_activity`** | watcher foreground harus ganti tag |
| Foreground rich info | tidak ada | `ProcessMonitor: onForegroundInfoChanged: ForegroundInfo{...}` (main buffer) | bisa dipakai untuk label + multiwindow, tapi **tidak selalu log saat pindah ke launcher** |
| Multi-window | `GameBoosterService` log `mMultiWindowForegroundPackageName` | `MultiScreenObserver` ada, tetapi log `splitScreenMode:false` pada split sintetis; `ProcessMonitor` punya `mMultiWindowForegroundPackageName` tetapi tidak berubah pada split sintetis | **belum ada sinyal yang terbukti; perlu riset lanjutan / fallback `dumpsys`** |
| FGS | Android 10 | Android 12: restriksi start FGS dari background, `PendingIntent` mutability, notification trampoline | app shell harus menangani `ForegroundServiceStartNotAllowedException`, flag PendingIntent |
| Phantom process killer | tidak ada | Android 12 punya phantom process killer; uji daemon root detached bertahan (PPID 1) | risiko rendah, tetapi harus diuji dengan daemon yang di-spawn app + stdin pipe |
| Cgroup | v1 | hybrid v1 + **cgroup2 freezer** (`/sys/fs/cgroup/frozen|unfrozen`, dibuat `millet_monitor`) | app bisa dibekukan; FGS + whitelist battery optimization penting |
| Root | APatch | APatch; `su` uid 0, transisi `magisk:s0`, tulis sysfs berhasil | tetap single-writer root daemon |

### 2. CPU / scheduler

| Area | MiFineTune | MIUI 14 | Dampak |
|---|---|---|---|
| Screen-on floor | 576000/652800 (bisa turun) | **boost 1612800/1843200** via `msm_performance.cpu_min_freq` (interaction-scoped) | profil tidak bisa menurunkan floor saat boost; `scaling_min_freq` di bawah floor live di-clamp |
| Screen-off floor | 576000/652800 | 576000/652800 | sleep profile tetap bisa cap max |
| `sched_boost` | bisa 0/1; forbidden | screen-on 1, screen-off 0; forbidden | jangan ditulis; framework punya kontrol |
| OPP little | 300k..1804800 (tanpa 1612800/1708800?) | + `1612800`, `1708800` | validator OPP harus pakai list live |
| OPP big | 300k..2304000 | + `1843200`, `1939200`, `2169600`, `2208000` | balance big cap minimal 1843200 saat screen-on |
| core_ctl | `policy0/core_ctl` (referensi) | `cpu0/core_ctl`, `cpu6/core_ctl`; big `enable=0` | path & validasi beda |
| `schedutil` | hispeed 1248000/1324600, load 90/85 | sama; tidak ada `schedutil/conservative_pl` | `kernel.sched_conservative_pl` tetap ada |
| `cpu_boost` | input 1324800@120ms | sama, powerkey 1804800/2208000@400ms | `input_boost_freq` tetap satu-satunya yang diizinkan |
| cpuset stock | 0-3/0-5/0-5/0-7 | **0-2 / 0-3 / 0-2,4-7 / 0-7**; `foreground/boost=4-7` | profile tidak boleh copy cpuset MIUI 12 |

### 3. GPU / IO / VM

| Area | MiFineTune | MIUI 14 | Dampak |
|---|---|---|---|
| GPU levels | 7 level 180..800 | sama | pwrlevel mapping sama |
| GPU stock | max 0/min 6/default 6 | sama | powersave max 5, game min 3 |
| IO scheduler | cfq/deadline | cfq stock, deadline tersedia | sama |
| CFQ stock | beda | quantum 8, fifo 125/250, slice 100/40, low_latency 1 | nilai profile harus pakai baseline MIUI 14 |
| VM stock | dirty 25/10/500/1000 (asumsi) | **20/5/200/500**, stat 1, watermark 1 | jangan pakai angka referensi tanpa cek |
| f2fs | sda16 | sda16 | sama |

### 4. Display / refresh

| Area | MiFineTune | MIUI 14 | Dampak |
|---|---|---|---|
| Settings key | `user_refresh_rate` | `user_refresh_rate`, `peak_refresh_rate`, `miui_refresh_rate` (secure); settings alone **tidak** mengubah panel | tetap butuh jalur SF |
| SF transaction | `1035 i32 <idx>` | **`1035 i32 <idx>` masih bekerja** (0=120,1=90,2=60,3=50,4=30) | bisa reuse |
| MIUI internal code | 1035 | `ScreenEffectService` punya `31035`/`31037`; `service call` 31035 gagal `Not a data message` | gunakan 1035 |
| DFPS list | `[120,90,60,50,30]` | sama, dari panel `nt36672c_huaxing` | mapping sama |
| HAL helper | dex `setScreenEffect(24,hz)` | dex jalan tetapi mencetak error `theme_compatibility.xml`; log HAL `HandleFpsSwitch` tidak terlihat pada uji | SF transaksi jadi sumber utama; helper opsional |
| Panel | 1080x2400 | sama; modes 120/90/60/50/30 | per-app refresh memungkinkan |

### 5. Charging

| Area | MiFineTune | MIUI 14 | Dampak |
|---|---|---|---|
| `input_suspend` | tersedia, 0777 | tersedia, 0777; `1` -> Discharging | per-app bypass tetap bisa |
| `battery_charging_enabled` | tersedia | tersedia; `0` -> current discharge walau status masih Charging | charge guard pakai current sign sebagai verifikasi |
| `charge_full_design` | rusak/negatif | rusak/negatif; bms 5216 mAh | wear estimate pakai 5216 (atau spec 5160) |
| MIUI charge limit UI | tidak ada | tidak terlihat di build ini | fitur charge limit kita tetap relevan |

### 6. Thermal

| Area | MiFineTune | MIUI 14 | Dampak |
|---|---|---|---|
| Config | encrypted | encrypted, tapi `decrypt.txt` plaintext | bisa baca config aktif |
| Normal | big 1209600 @50 °C | sama | balance big cap 1.32 GHz tidak mungkin saat boost floor 1.8432 aktif |
| Sensor | CPU thermal zone by type | banyak `cpu-0-*-usr`/`cpu-1-*-usr`; SS memakai `nvm-therm-adc` (VIRTUAL-SENSOR) | thermal telemetry baca nvm + quiet |
| tgame | ada | ada (encrypted); teruji: cap cpu6 sama, tambah cap cpu0 + LCD awal | bukan upgrade CPU sustained |
| `thermal_message` | ada | `sconfig=0`; game launch tidak mengubahnya | app hanya baca, tidak menulis |
| Override | — | `/data/vendor/thermal/config` (map+config terenkripsi) | R&D only; tidak dipakai app |

### 7. MIUI stack

| Area | MiFineTune | MIUI 14 | Dampak |
|---|---|---|---|
| PowerKeeper | ada | ada 4.2.00; `mPowerMode=2`, `PowerModeHandler` mengatur floor | jangan lawan; monitor saja |
| `mcd` | ada | ada; config plaintext | framework-owned |
| `millet_monitor` | tidak ada | ada (SIG/BINDER/PKG; cgroup2 freezer) | app freezing; FGS/whitelist |
| Joyose | ada (per-game data) | **tidak ada** | jangan bergantung pada Joyose |
| GameTurbo | ada | package tidak ada; game launch tidak memilih `sconfig=9` | game checker lama tidak berlaku |
| perf HAL | 2.0 | **2.2**, 166 resource path | overlap catalog 22 path; baseline/forbidden diperbarui |
| QS tile performance | tidak | `PerformanceModeTileService` (securitycenter) | MIUI-owned, jangan ditulis |

### 8. Ringkasan aturan baru

1. **Jangan pernah** berharap bisa menurunkan CPU floor saat boost aktif; floor itu milik MIUI power policy (`msm_performance`), bukan katalog kita. Boost bersifat interaction-scoped (wake/touch), bukan konstanta screen-on.
2. Validasi `scaling_max_freq >= live_min` (bukan hanya OPP); kalau tidak, tulis gagal.
3. Profile `balance`/`game` tidak boleh memakai cap big < 1843200 saat screen-on.
4. Refresh: **SF transaksi 1035** adalah lever utama; settings key hanya bookkeeping.
5. Foreground watcher: `wm_resume_activity`/`wm_set_resumed_activity`; `ProcessMonitor` sebagai pelengkap.
6. Multi-window: sinyal lama tidak ada; perlu fallback `dumpsys`/riset.
7. Joyose/GameTurbo checker tidak dipakai; bridge fokus ke saver, refresh, DND, charge/bypass.
8. Perf HAL 2.2 overlap 22 node -> Baseline; `msm_performance`, `millet`, `mcd`, thermal, `cpu_boost` prefix tetap forbidden.


---

## RUNTIME-BEHAVIOR.md

## Runtime behavior — hasil eksperimen terkontrol

Semua uji dilakukan di device dengan root APatch. Setiap perubahan
dikembalikan ke nilai awal; tidak ada state tuning yang ditinggalkan.

> **Koreksi 2026-11-21 (RND battery/perf):** the "screen-on floor" below is
> **not a constant screen-on state**. Re-verified after a clean reboot:
> wake + swipe produces a ~3 s wake boost (`0:1804800 6:2304000`), a swipe
> produces a ~1 s touch blip (`0:1248000 6:1094400`), and
> `kernel.sched_boost` stays `0` through screen-on in adb-driven sessions.
> The sustained `1612800/1843200` floor was observed during manual device
> use (charging + operator interaction) on 2026-11-19 and could not be
> reproduced from automation, even with real injected input and 3 min of
> normal use. Treat the floor as **interaction-scoped**: MIUI decides the
> regime. ADB-driven screen-on sessions run floor-less (`cpu_min_freq=0`,
> `scaling_min` = the katalog's own values). Profiles must be sane in both
> regimes. See `docs/HISTORY.md`.

### 1. Screen on/off cycle

> (See the 2026-11-21 correction note above — the "Screen ON" row below was
> measured during manual operator use; adb-driven sessions show the
> floor-less regime instead.)

Sampler 1 detik (CSV di arsip lokal), 120 detik,
melewati screen-on -> screen-off -> wake -> sleep.

| Fase | `scaling_min_freq` p0/p6 | `msm_performance.cpu_min_freq` | `sched_boost` | core_ctl active |
|---|---|---|---|---|
| Screen ON | **1612800 / 1843200** | 1612800 / 1843200 | **1** | 6 / 2 |
| Screen OFF | **576000 / 652800** | 0 / 0 | **0** | 6 / 2 |

Kesimpulan:

- Floor CPU screen-on berasal dari `msm_performance` (PowerKeeper/perf stack),
  bukan dari `scaling_min_freq` katalog.
- Menulis `scaling_min_freq=576000` saat screen-on menghasilkan readback
  **1612800** (write rc=0, tetapi di-clamp). Saat screen-off, write yang sama
  berhasil dan readback 576000.
- `scaling_max_freq` bisa diturunkan sampai **sama dengan floor**:
  - p0 max 1612800: berhasil (readback 1612800)
  - p6 max 1843200: berhasil (readback 1843200)
  - max di bawah floor (mis. p0 1248000 saat floor 1612800, p6 1324800
    saat floor 1843200) **di-clamp ke floor** dengan write rc=0
    (readback = floor, bukan nilai yang diminta). Verifikasi readback wajib.
- `sched_boost` 1 saat screen-on dan 0 saat screen-off; **forbidden**, jangan
  ditulis.

### 2. Power mode / PowerKeeper

- `dumpsys activity service com.miui.powerkeeper`:
  - `mPowerModeHandler enabled`
  - `powerMode = 2` (mapping internal: 1=performance, 2=equilibria/balance,
    3=saving menurut `PowerModeHandler.setPowerModeState`)
  - `mPowerMode:2` juga muncul di dump KillProcessController.
- `persist.sys.power.default.powermode = enhance`
- `settings get system power_mode` = null (tidak ada key user-facing).
- `Settings.Global low_power` tetap ada dan berfungsi (lihat bawah).
- MIUI 14 juga punya QS tile `PerformanceModeTileService`
  (`com.miui.securitycenter`) dan `power_performance_tile_enabled=1`.
- **Rekomendasi**: perlakukan power mode sebagai milik MIUI. Aplikasi boleh
  menampilkan statusnya, tetapi jangan menulis `msm_performance` atau
  memaksa power policy.

### 3. Battery saver (`low_power`)

Uji dengan `dumpsys battery unplug` agar tidak sedang charging:

```
dumpsys battery unplug
settings put global low_power 1
-> Battery Saver is currently: ON
settings put global low_power 0
dumpsys battery reset
-> OFF
```

- `low_power=1` berhasil; status saver ON.
- Floor CPU tetap 1612800/1843200; max tetap 1804800.

**Battery percentage mismatch (2026-11-20):** Diagnostics membaca
`/sys/class/power_supply/battery/capacity` langsung; status bar membaca
Android `BatteryService`. Kalau `dumpsys battery` menampilkan
`UPDATES STOPPED`, berarti service masih memegang override dari R&D
(`dumpsys battery unplug` / `set ac`). Jalankan `dumpsys battery reset`;
setelah itu level BatteryService dan status bar kembali sama dengan sysfs.
`surya-prof doctor` (0.4.1) menambahkan check `battery_service` untuk
mendeteksi kondisi ini.
- Jadi MIUI battery saver **tidak** menurunkan floor CPU. Bridge saver tetap
  berguna untuk perilaku framework (sync/animation/background), bukan CPU.

### 4. Refresh rate

#### Settings key saja tidak cukup

Uji `settings put system user_refresh_rate 120`, lalu `peak_refresh_rate 120`,
lalu `settings put secure miui_refresh_rate 120`; `dumpsys display` tetap
`mActiveModeId=2` (90 Hz). Disimpulkan settings key hanya bookkeeping.

#### SurfaceFlinger transaction 1035 bekerja

```
service call SurfaceFlinger 1035 i32 <idx>
```

| idx | fps | `mActiveModeId` |
|---|---|---|
| 0 | 120 | 1 |
| 1 | 90 | 2 |
| 2 | 60 | 3 |
| 3 | 50 | 4 |
| 4 | 30 | 5 |

- `service call SurfaceFlinger 31035 i32 0` -> error `Not a data message`
  (kode internal MIUI, bukan jalur `service call`).
- Kode MIUI `ScreenEffectService` di `miui-services.jar`:
  `SURFACE_FLINGER_TRANSACTION_DISPLAY_FEATURE_DFPS = 31035` (dan
  `SMART_DFPS=31037`, `SET_MODE=31023`, `PCC=31101`, `DISPLAY_FEATURE=31100`),
  tetapi jalur shell yang terbukti bekerja adalah **1035**.
- Panel: `dsi_nt36672c_huaxing_fhd_video_display`; DFPS list
  `[120, 90, 60, 50, 30]`; path panel_info dan device-tree sama seperti
  referensi.
- Dex helper referensi (`setScreenEffect(24, hz)`) berjalan tetapi mencetak
  error `theme_compatibility.xml` dan tidak terlihat memicu log
  `DisplayFeatureHal: HandleFpsSwitch`; transaksi SF adalah lever utama.
- State settings saat audit: `user_refresh_rate=90`, `peak_refresh_rate=90`,
  `miui_refresh_rate=90` (secure).

**Desain**: per-app refresh memakai SF 1035 + update settings key (capture/
restore). Jangan menulis props dfps.

### 5. Charging / bypass

Uji langsung:

| Aksi | Status | `current_now` | Node |
|---|---|---|---|
| Awal | Charging | -420837 (negatif = charging) | `input_suspend=0`, `battery_charging_enabled=1` |
| `input_suspend=1` | **Discharging** | +768738 | 1 |
| `input_suspend=0` | Charging | -444031 | 0 |
| `battery_charging_enabled=0` | Charging (status tidak berubah) | **+362854** (discharge) | 0 |
| `battery_charging_enabled=1` | Charging | -449219 | 1 |

- `input_suspend` cocok untuk per-app bypass; efek langsung terlihat di status.
- `battery_charging_enabled` cocok untuk charge limit; status bisa tetap
  "Charging" walau current sudah positif, jadi verifikasi harus memakai
  `current_now`/arah arus, bukan hanya status.
- Battery health: `charge_full=5111000` (5111 mAh), `cycle_count=34`,
  `bms.charge_full_design=5216000`; `battery.charge_full_design` negatif/rusak.

### 6. Thermal

- `thermal_message/board_sensor_temp` = VIRTUAL-SENSOR (nvm-therm) ~38.5 °C
  saat idle; `quiet-therm-adc` ~35.7 °C; `battery` ~29.9 °C.
- Zone CPU `cpu-0-*-usr` / `cpu-1-*-usr` 36.3..38.5 °C; `-step` sedikit lebih
  tinggi; `lmh-dcvs-*` 75 °C (bukan suhu aktual, limit).
- `sconfig=0` (normal); game launch tidak mengubahnya.
- Config normal aktif:
  - big cluster cap 1209600 di VIRTUAL-SENSOR (`nvm-therm-adc`) 50 °C
    (clear 48 °C); terlihat live: `policy6 max=1209600`, cdev8=8
  - battery current target 201..1515 mA pada quiet-therm 38..45 °C
  - backlight 819/1230 pada 44/47 °C
  - temp_state 4 pada 60 °C
  - CCC hotplug pada 61 °C
- `sconfig=10` (nolimits) melepas cpu6 ke 1843200, tetapi menambah cap cpu7
  dan GPU; `tgame` tetap cap cpu6 1209600 dan menambah cap little core.
- `temp_state` adalah status output (dibaca PowerKeeper), bukan selector.
- v0.4.0 mengambil alih empat profile lewat encrypted `/data` map/config
  (`sconfig` 60–63; v0.4.x used 20–23), memakai `nvm-therm-adc` langsung untuk SS/GPU; MIUI
  camera/phone/ARVR di-yield. Service OFF mengembalikan `sconfig` dan
  menghapus file custom.
- Untuk telemetry, gunakan `nvm-therm` (SS), `quiet-therm` (battery/LCD), dan
  cooling device `thermal-cpufreq-6/7`. Detail + tabel threshold:
  [THERMAL-MIUI14.md](EVIDENCE-THERMAL.md).

### 7. Root / daemon

- `su -c id` -> uid=0, context shell/magisk; write ke sysfs/proc berhasil
  (diuji: scaling min/max, governor, stune, cpuset, sched, gpu pwrlevel,
  io scheduler, vm stat, charge nodes, SF transaction).
- Write `io.scheduler`: harus menulis token (`cfq`/`deadline`), bukan string
  readback `noop deadline [cfq]`; readback hanya token dalam bracket yang
  aktif.
- Daemon root detached (`nohup sh ... &`) bertahan, PPID 1, melewati screen
  off, tidak dibunuh phantom killer dalam uji 120 detik.
- `su -c cat` dengan stdin pipe + EOF bekerja (lease model JSON-lines layak).
- `logcat` dari root bisa membaca main/events; `settings` CLI bisa; `service
  call` bisa; `app_process` bisa (dengan quirk theme_config).

### 8. Foreground / Dynamic Profile (v0.4.2)

Aturan yang dipakai daemon:

- `last_fg` = foreground mentah terakhir (termasuk overlay).
- `last_real` = app non-transient terakhir di bawah overlay.
- Keputusan memakai `last_fg` kalau itu app nyata; hanya SystemUI, IME, dan
  permission dialog yang jatuh ke `last_real`.
- Surya Profiler **bukan** transient. App ini tidak pernah muncul di daftar
  Apps Profile, jadi membukanya selalu berarti kembali ke base universal.
- Base hanya berubah lewat tap Device Profile / `SET_PROFILE`; mapping app
  hanya override selama app tersebut di depan.
- `config_changed` (dikirim app setiap resume) memakai aturan foreground yang
  sama, jadi mapping YouTube tidak bisa hidup lagi saat Surya di depan.
- Watcher membuang event logcat yang lebih lama dari waktu spawn, sehingga
  restart service tidak memutar ulang resume event lama.

Bukti device (base Balance, 2026-11-20):

```
YouTube     -> active powersave, sconfig 22, MIUI saver ON
Surya       -> active balance,   sconfig 20, MIUI saver restored
WhatsApp    -> active balance,   sconfig 20
Home        -> active balance,   sconfig 20
Epic Seven  -> active game,      sconfig 21
Surya       -> active balance,   sconfig 20
Dynamic OFF -> YouTube tetap balance
Service restart saat Surya di depan -> balance (tanpa replay powersave)
```


---

## SCHEDULER-MIUI14.md

## Scheduler / WALT — MIUI 14 surya

Status: **R&D track for v0.5.x; no profile values shipped yet.**
Source: `/vendor/bin/init.qcom.post_boot.sh` (sm6150 / SDMMAGPIE branch),
`vendor/etc/perf/*`, and the perf-HAL overlap audit.

### Why this track

The current profiles tune cpufreq, core_ctl, cpuset, stune, GPU, IO and VM.
MIUI's own boot script also writes WALT task-packing and migration nodes that
are not in the 95-entry catalog. They can improve responsiveness or battery
without touching the framework-owned CPU floor. We only add them after a
device audit and an A/B run.

### ROM baseline (from `init.qcom.post_boot.sh`)

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
adb push tools/research-scripts/audit_walt.sh /data/local/tmp/
adb shell su -c 'sh /data/local/tmp/audit_walt.sh'
```

### Node inventory / ownership

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

### Draft candidate matrix (not shipped)

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

### A/B protocol

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

### Open questions

- Does the perf HAL rewrite `sched_prefer_idle`/`sched_mostly_idle_*` at
  runtime, or only at boot?
- Are per-CPU `sched_mostly_idle_*` nodes writable by root on this kernel?
- Does `sched_restrict_tasks_spread=1` interact with cpuset `game`/`gamelite`?
- Does PowerKeeper's `mPowerMode` change any of these nodes when the user
  switches MIUI power mode?

Until these are answered, the scheduler track stays read-only.


---

## LOG-SIGNALS.md

## Logcat signals — MIUI 14 / Android 12

Semua contoh dari device (epoch seconds, `-v epoch`). Untuk daemon, gunakan
stream terpisah agar parsing sederhana dan tidak membanjiri buffer.

### 1. Foreground: events buffer (`wm_*`)

**Tag yang terbukti bekerja:**

```text
wm_resume_activity: [0,<activityId>,<taskId>,<pkg>/<activity>]
wm_set_resumed_activity: [0,<pkg>/<activity>,<reason>]
wm_task_to_front: [0,<taskId>]
wm_focused_root_task: [0,0,<rootTaskId>,<lastRootTaskId>,<reason>]
```

Contoh nyata:

```text
1795100618.869  1732  2942 I wm_resume_activity: [0,117144395,177,com.whatsapp/.home.ui.HomeActivity]
1795100621.031  1732  1767 I wm_set_resumed_activity: [0,com.miui.home/.launcher.Launcher,bringingFoundTaskToFront]
```

- Launcher **hanya** muncul di `wm_set_resumed_activity` pada kasus ini; karena
  itu parser harus mendukung kedua tag (seperti referensi).
- Package = token sebelum `/` pada field package/activity.
- Filter server-side: `logcat -b events -v epoch -s wm_resume_activity:I wm_set_resumed_activity:I`
  (format `TAG:LEVEL`; gunakan `*:S` untuk silence tag lain bila perlu).

### 2. Foreground rich info: main buffer (`ProcessMonitor`)

```text
ProcessMonitor: onForegroundInfoChanged: ForegroundInfo{
  mForegroundPackageName='com.whatsapp',
  mForegroundUid=10186, mForegroundPid=10736,
  mLastForegroundPackageName='com.android.settings',
  mLastForegroundUid=1000, mLastForegroundPid=6417,
  mMultiWindowForegroundPackageName='com.whatsapp',
  mMultiWindowForegroundUid=10186,
  mFlags=0
}
```

- Muncul saat app berubah (WhatsApp -> Chrome -> Settings).
- **Tidak muncul** saat pindah ke launcher pada uji `am start HOME`.
- `mMultiWindowForegroundPackageName` masih sama dengan foreground pada split
  screen sintetis; belum bisa diandalkan sebagai sinyal multi-window.
- Tag duplikat: `PerfEngineController` (PowerKeeper) dan `AppObserver`
  (launcher) mencetak objek yang sama.
- Berguna sebagai sinyal sekunder / label, bukan pengganti `wm_*`.

### 3. Multi-window

Belum ada sinyal yang terbukti.

- `MultiScreenObserver: onTaskStackChanged, splitScreenMode: false isCameraFloat: false isSmallScreen: false`
  muncul saat split screen sintetis dibuat (Chrome primary + Settings
  secondary, keduanya `visible=true` di `dumpsys activity activities`).
- `ProcessMonitor` tidak berubah pada split tersebut.
- `GameBoosterService` / `mMultiWindowForegroundPackageName` ala MIUI 12 tidak
  ada (package Joyose/GameBooster tidak terpasang).
- Kandidat fallback: polling `dumpsys activity activities` dan cari
  `mode=split-screen` + `visible=true`. Biaya perlu diukur; root `dumpsys`
  bekerja.
- Kandidat lain: `dumpsys window displays` / `dumpsys activity` events; belum
  diverifikasi.

### 4. Jank

- `Choreographer` masih mencetak `Frame time is ... in the future!` pada MIUI 14.
- Uji swipe di Chrome tidak menghasilkan `Skipped N frames!`; sinyal jank
  `Choreographer: Skipped N frames!` belum terbukti.
- Alternatif: `dumpsys gfxinfo` (janky frames) atau frame stats; perlu
  eksperimen dengan app/game yang benar-benar jank.
- Untuk v1, jank boost sebaiknya **opsional/experimental** dan tidak menjadi
  blocker.

### 5. Screen / power / thermal (dari app)

- App dapat memakai broadcast `ACTION_SCREEN_ON/OFF` + `KeyguardManager`.
- `low_power` bisa dibaca via `settings get global low_power`.
- `thermal_message/board_sensor_temp` bisa dibaca langsung (root) atau via
  `PowerManager` thermal API untuk app.
- `DisplayFeatureHal` logs (`HandleFpsSwitch`, `HandleDisplayState`) berguna
  untuk debug refresh; jangan dijadikan input keputusan.

### 6. Contoh filter daemon yang disarankan

```text
## foreground (events)
logcat -b events -v epoch -s wm_resume_activity:I wm_set_resumed_activity:I *:S

## rich foreground (main)
logcat -b main -v epoch -s ProcessMonitor:I *:S

## jank (main; experimental)
logcat -b main -v epoch -s Choreographer:W *:S
```

Catatan: `logcat -s` dengan `*:S` harus diuji di device; jika tidak didukung
oleh toybox logcat versi ini, gunakan `logcat -b events -v epoch` lalu filter
di proses daemon. Stream mentah events buffer relatif kecil.


---

## analysis/catalog_miui14.md

## Catalog probe — MIUI 14 V14.0.2.0.SJGIDXM / Android 12

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


---

## analysis/catalog_missing.md

## Catalog paths missing on MIUI 14

Tidak ada. Semua 95 path ada.
