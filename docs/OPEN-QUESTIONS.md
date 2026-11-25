# Open questions / R&D lanjutan

Hal yang belum cukup bukti dan perlu diputuskan sebelum implementasi final.

> **Slim build (2026-11-20):** the product scope is now Device Profile +
> Apps Profile only. DND, refresh-rate follow, charge limit, bypass policy,
> battery/thermal guards, quiet hours, backup, jank boost, maintenance, MIUI
> performance mirror and game checker were removed. Open questions about
> those features are archived/out of scope.
>
> **Resolved in the 2026-11-19/20 E2E:** app install + APatch root grant,
> binary deploy, FGS/daemon start, 3 profile applies + sleep, per-app
> mapping, saver follow, automation API, cold-buffer startup fallback,
> multi-window detection, R&D input suspend, and true Service OFF stock
> restore (no reboot). See
> [DEVICE-E2E-2026-11-19.md](HISTORY.md).

## 1. Multi-window detection — resolved 2026-11-20

- Split screen sintetis (Chrome atas + Settings bawah) dibuat lewat
  `am stack move-task`; screenshot membuktikan dua pane benar-benar tampil.
- `ProcessMonitor` tetap diam untuk split ini (konfirmasi bahwa jalur MIUI
  tidak cukup), tetapi `dumpsys window` memuat kedua task:
  `visible=true mode=split-screen-primary/secondary` + `A=<uid>:<pkg>`.
- Implementasi: daemon melakukan probe `dumpsys window` saat foreground
  berubah dan setiap 5 tick supervisor (15 s); parser hanya menerima satu
  pane primary + satu pane secondary yang keduanya visible.
- Device-verified: `bridge: multi-window ON (com.android.settings)` →
  `-> balance (multi-window)`; keluar split → `multi-window off` → base
  kembali. Unit test parser ditambahkan.
- Catatan: uji dengan gesture Recents milik user tetap disarankan setelah
  update MIUI; struktur `dumpsys window` bisa berubah.

## 2. Jank detection — removed from scope (2026-11-20)

- `Choreographer: Skipped N frames!` belum muncul pada uji swipe Chrome.
- Perlu app/game yang benar-benar jank, atau `dumpsys gfxinfo` polling.
- Keputusan 0.2: jank boost dihapus dari produk.

## 3. Power mode / floor

- `PowerKeeper` `mPowerMode=2`; mode 1/3 mapping internal. Belum diuji apakah
  power mode 3 (saving) menurunkan floor `msm_performance`.
- Jika ya, apakah kita boleh menawarkan tombol "Power mode" (MIUI-owned) atau
  hanya menampilkan status?
- Untuk sekarang: jangan menulis `msm_performance`/power mode. Profil hanya
  bekerja di dalam floor.
- **Update 2026-11-24:** model floor diperbarui — pin screen-on MIUI itu
  overlay berbatas (wake ~10 s, launch ~40 s; MIUI memulihkan floor
  sebelumnya saat release). Yang belum teridentifikasi: komponen persis
  yang menulis pin 1248000/979200 (perfd via PowerKeeper kemungkinan
  besar) — audit 0.8.0 mencatatnya di log, dan sentinel menegakkan floor
  hanya saat screen-off.

## 4. Refresh helper — removed from scope (2026-11-20)

- SF 1035 terbukti; per-app 60 Hz applied live (`mActiveModeId=3`,
  `user_refresh_rate=60`) and restored to 90 on exit.
- Dex `setScreenEffect(24)` masih mencetak error theme_config dan tidak
  terlihat di log HAL; SF transaction is sufficient for the panel.
- Masih perlu dicek: apakah MIUI Settings UI tetap konsisten setelah SF 1035,
  dan apakah `peak_refresh_rate`/`miui_refresh_rate` perlu ikut di-capture/
  restore (saat ini hanya `user_refresh_rate` yang di-hold).

## 5. Game mode / thermal tgame — resolved 2026-11-20

- Tidak ada Joyose/GameTurbo package; `sconfig=0` (normal).
- Launch Epic Seven tidak mengubah `sconfig`; Game Turbo tidak memilih
  `tgame` di build ini.
- Semua config vendor berhasil didekripsi dengan `mi-thermal-crypt`
  (AES-128-CBC, key/IV `thermalopenssl.h`); hasilnya identik dengan vendor.
- `thermal-tgame` bukan upgrade CPU sustained: cap cpu6 tetap 1209600 @50 °C,
  ditambah cap cpu0 1324800 @53 °C dan LCD lebih awal.
- `sconfig=10` (nolimits) melepas cpu6 ke 1843200, tetapi menambah cap cpu7/
  GPU dan menghapus CCC/LCD; bukan daily driver.
- Runtime override `/data/vendor/thermal/config` bisa (map+config harus
  terenkripsi), tetapi **tidak dipakai app**.
- Keputusan 0.3: app hanya membaca telemetry.
- 0.4.0: custom takeover per profile (private `sconfig` 20–23), dengan stock
  fallback, yield ke camera/phone/ARVR, dan Service OFF restore. Detail:
  [THERMAL-MIUI14.md](EVIDENCE-THERMAL.md).

## 6. Profile values

- Semua nilai di `PROFILE-DESIGN.md` masih draft.
- Perlu apply/verify penuh dan cek:
  - `scaling_max_freq` saat transisi screen on/off;
  - drift setelah app launch/thermal;
  - efek cpuset `0-7` untuk game (stock foreground 0-2,4-7);
  - efek `cpu_boost.input_boost_freq`;
  - efek `schedutil.hispeed` di atas floor.
- Perlu benchmark baterai/thermal sederhana (10 menit A/B) setelah kandidat
  stabil.

## 7. App/Android 12 lifecycle

- FGS start dari background: `ForegroundServiceStartNotAllowedException`.
- `PendingIntent` mutability (API 31).
- Notification trampoline (API 31).
- MIUI Autostart + battery saver "No restrictions" + phantom process killer.
- Perlu uji app shell nyata (belum ada kode app).

## 8. Root/APatch lifecycle

- Daemon app-spawned dengan stdin pipe vs detached root process.
- Perlu uji:
  - app FGS -> `su -c miui-ft serve` -> EOF saat app mati;
  - MIUI memfreeze app (millet) saat background;
  - APatch prompt/deny;
  - restart storm.

## 9. Catalog additions

- Perf HAL 2.2 punya node baru (`uclamp`, `sched_prefer_idle`,
  `sched_task_unfilter_period`, dll). Belum dievaluasi.
- Untuk v1, jangan tambah node baru tanpa evidence + nilai profil yang jelas.
- Perlu audit owner map lengkap (boot + runtime) untuk build MIUI 14 ini.

## 10. Naming / packaging decisions — resolved for v0.1.0

- Package: `com.suryaprofiler`
- Binary: `surya-prof`
- State dir: `/data/adb/suryaprofiler`
- App label: `Surya Profiler`
- Deploy path: `/data/local/tmp/suryaprofiler/surya-prof`

## 11. Thermal index collision — resolved v0.5.0

- PowerKeeper `ThermalSwitchHandler` uses base IDs 0–25 plus offsets
  `+30/+100/+200/+500`; its package map maps YouTube to 21 and camera to 12.
- Custom indices moved from 20–23 to **60–63**; v0.4.x is migrated on the
  first sync. The map is generated from `/vendor/etc/thermal-map.conf`.
- Yield policy: camera/phone/ARVR/performance yield; video/game/benchmark/
  unknown keep the active Device Profile.
- Open: confirm on device that MIUI actually writes 21/121 for YouTube on
  this boot, and that `thermal-global-mode` changes with the MIUI performance
  tile. Host tests cover the state machine.

## 12. Scheduler / WALT — RESOLVED 2026-11-21 (absent)

- See [SCHEDULER-MIUI14.md](EVIDENCE-DEVICE.md) and
  `scripts/audit_walt.sh`. Resolved by the 2026-11-21 device probe: the
  WALT nodes (`sched_window_stats_policy`, `sched_ravg_hist_size`,
  `sched_restrict_tasks_spread`, `sched_small_task`, per-CPU
  `sched_mostly_idle_*` / `sched_prefer_idle`) **do not exist** on kernel
  4.14.190-perf-g75201eeb4293. Only `sched_load_boost` exists (perf-HAL/
  ROM-owned, forbidden) and `sched_time_avg_ms` (already cataloged).
- The draft candidate matrix in SCHEDULER-MIUI14.md is dead for this
  build; no profile values will ship from that track.

## 13. New findings from the 2026-11-21 battery/perf R&D

- The screen-on CPU floor (`msm_performance` 1612800/1843200) is
  **interaction-scoped**, not a constant screen-on property — adb-driven
  sessions run floor-less. RUNTIME-BEHAVIOR.md carries the correction.
- `charge_counter` quantizes (~51 mAh steps); gfxinfo cannot see
  Chrome's renderer or game GL surfaces. Full details, harness patterns
  and per-profile verdicts:
  [RND-BATTERY-PERF-2026-11-21.md](HISTORY.md).

## 14. Thread placement (Fase 6) — implemented 0.9.0, opt-in

- **Implemented:** `bridge/affinity/` + `thread_affinity` config key
  (default off). While a Game-mapped app is in front: `UnityMain`/
  `GameThread` → cpu6–7 (`c0`), `UnityGfx*`/`RenderThread`/`RHIThread` →
  cpu6 (`40`). Captured per-tid masks in `affinity.json`, restored on leave,
  screen off, lock, Service OFF and daemon exit; orphans reconciled at
  startup. Never widens a mask; if the app re-pins a thread, our record is
  dropped and the thread is yielded for that app session (no re-pin fight).
- **Device evidence (2026-11-24, Seven Knights):** `RenderThread` and
  `UnityGfxDeviceW` moved to cpu6; `UnityMain` was already on 6-7 without
  our write, so it is left alone. Leave/exit restored 4/4 captured masks and
  cleared `affinity.json`.
- **Open:** per-app presets/masks (the current rule table is global for
  Game-mapped apps); an A/B power/fps run for the pin (the 0.7.0 Game
  profile numbers still stand); games that self-pin render threads may see
  little benefit. No new catalog/sysfs surface.

## 15. Render scale / display scale — implemented 0.10.0

- **Per-app render scale:** `cmd game downscale` hold, prior capture from
  `dumpsys platform_compat`, restored on unmap/Service OFF/exit. Device
  verified 0.8 on Seven Knights (`mCompatFrame=864×1920`), 0.65 layout test
  on Chrome (`inner` unchanged, DPR 2.5→1.625).
- **Global display scale:** exact 20:9 presets; `hd` = 720×1600 @320
  (vendor 720p density), `hd-compact` = 720×1600 @267 (same 431dp layout as
  native 400/1080); exact original size/density restored on Off/Service OFF.
  Device verified `hd` and combined 576×1280; restore to 1080×2400 @400.
- **Power R&D (60 s):** GPU-bound Chrome stress −29% at native+0.65 and −41%
  at 720p+0.8; Seven Knights −27% at native+0.8 and −36% at 720p; TikTok
  neutral (GPU ~8%→4%, decode/network/panel dominated).
- **Open:** 3–5 min A/B for the shipped ratios; MIUI Game Turbo can write a
  different downscale (foreground re-check re-asserts); global display
  reboot/persistence needs a device run; per-app 65/70 choice for video apps
  is user preference, not a guaranteed battery win.

## 16. Power usage — implemented 0.11.0 (host-verified)

- **Read-only:** 3 s screen-on / 30 s screen-off sampler; one-minute buckets;
  7-day `power-history.jsonl`; all-time `power-peaks.json`; clear; clean-exit
  flush. No charge/limit/guard writes.
- **Units:** battery-side `current_now × voltage_now`; negative current =
  charging. While charging this is net into the battery, not wall power.
  `usb`/`main` input rails are shown only when the kernel exposes them and
  are labelled as reported/negotiated, not measured wall power.
- **Attribution:** profile every sample (including Sleep); app only when
  screen-on and unlocked. Multi-window buckets use the daemon's primary
  foreground package; this is an approximation.
- **Device-verified 2026-11-26:** live 797 mA / 3.51 W at 85% matched
  `current_now=-798645 µA` / `voltage_now=4403272 µV`; `charge_counter`
  exists (3,731,030 µAh) and is quantized, so it is only a rough estimate;
  `usb` has no `voltage_now`/`current_now` files — its `uevent` reports
  `VOLTAGE_NOW=4748096`, `INPUT_CURRENT_NOW=1317690`,
  `REAL_TYPE=USB_CDP`, and `main` has `input_*_settled`; the probe uses
  those fallbacks. `input_suspend=1` flips to discharging, the warning
  appears, and the switch restores 0. Screen-off produced a 60 s bucket
  with `screen_on:false`, `profile:"sleep"`, 2 samples (deep-sleep cadence).
  Per-app/profile rows (2-minute filter), peaks, Clear, and history across
  Service OFF/ON all verified. `docs/HISTORY.md` has the full table.
- **Open:** the 7-day trim has not been run for a week yet (unit-tested);
  `battery_charging_enabled=0` was not exercised on device (read-only
  detection path); a different fuel gauge may use the opposite current sign
  and needs a fresh audit. Short charge/usage switches inside one minute are
  averaged in the chart (live card + peaks keep the extremes).
