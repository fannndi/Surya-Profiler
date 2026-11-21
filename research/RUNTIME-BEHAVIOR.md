# Runtime behavior — hasil eksperimen terkontrol

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
> regimes. See `research/RND-BATTERY-PERF-2026-11-21.md`.

## 1. Screen on/off cycle

> (See the 2026-11-21 correction note above — the "Screen ON" row below was
> measured during manual operator use; adb-driven sessions show the
> floor-less regime instead.)

Sampler 1 detik (`research/raw/state_screen_cycle.csv`), 120 detik,
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

## 2. Power mode / PowerKeeper

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

## 3. Battery saver (`low_power`)

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

## 4. Refresh rate

### Settings key saja tidak cukup

Uji `settings put system user_refresh_rate 120`, lalu `peak_refresh_rate 120`,
lalu `settings put secure miui_refresh_rate 120`; `dumpsys display` tetap
`mActiveModeId=2` (90 Hz). Disimpulkan settings key hanya bookkeeping.

### SurfaceFlinger transaction 1035 bekerja

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

## 5. Charging / bypass

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

## 6. Thermal

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
  [THERMAL-MIUI14.md](THERMAL-MIUI14.md).

## 7. Root / daemon

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

## 8. Foreground / Dynamic Profile (v0.4.2)

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
