# Surya-Profiler R&D — index

Device: **POCO X3 NFC (surya / sm6150-AC / SDMMAGPIE)**
ROM: **MIUI 14 V14.0.2.0.SJGIDXM / Android 12 / SKQ1.211019.001**
Kernel: **4.14.190-perf-g75201eeb4293**
Root: **APatch** (su works; root uid 0)
Status: **R&D complete for the slim surface; app + daemon built and device
E2E passed 2026-11-19/20** (3 Device Profiles + Sleep, Apps Profile mapping,
multi-window, MIUI saver follow, R&D input suspend, true Service OFF restore).
0.3.0 adds MIUI thermal telemetry (sconfig/caps/cooling); 0.4.0 adds
per-profile custom thermal takeover (balance/game/powersave/sleep) with stock
fallback and Service OFF restore. 0.4.2 makes the Device Profile base
universal when returning to any unmapped app, including Surya Profiler.
0.5.0 moves the custom thermal indices to 60–63 (PowerKeeper uses 20–25 plus
offsets; mi_thermald rejects map IDs >64), generates the map from the vendor
blob, decodes MIUI scenarios, adds the thermal event log and MIUI background
checks. Device E2E passed 2026-11-20. See `../`,
[DEVICE-E2E-2026-11-19.md](DEVICE-E2E-2026-11-19.md) and
[THERMAL-MIUI14.md](THERMAL-MIUI14.md).

## Dokumen

| File | Isi |
|---|---|
| [DEVICE-FACTS.md](DEVICE-FACTS.md) | identitas device, root, kernel, cgroup, CPU/GPU, storage, battery |
| [MIUI14-DELTAS.md](MIUI14-DELTAS.md) | perbedaan penting vs MiFineTune (MIUI 12/Android 10) |
| [RUNTIME-BEHAVIOR.md](RUNTIME-BEHAVIOR.md) | perilaku screen on/off, power-mode floor, refresh, charging, thermal |
| [THERMAL-MIUI14.md](THERMAL-MIUI14.md) | decrypt `mi_thermald`, peta `sconfig`, threshold per config, bukti device |
| [SCHEDULER-MIUI14.md](SCHEDULER-MIUI14.md) | audit WALT/post_boot, kandidat tuning, protokol A/B |
| [LOG-SIGNALS.md](LOG-SIGNALS.md) | tag logcat MIUI 14 untuk foreground/multi-window/jank |
| [CATALOG-STATUS.md](CATALOG-STATUS.md) | status 95 entri referensi (92 aktif di 0.2) + overlap perf HAL |
| [PROFILE-DESIGN.md](PROFILE-DESIGN.md) | constraint desain profil dan kandidat nilai awal |
| [RND-BATTERY-PERF-2026-11-21.md](RND-BATTERY-PERF-2026-11-21.md) | R&D battery/perf: audit node baru, harness bypass-charging, A/B per skenario, temuan floor interaction-scoped |
| [OPEN-QUESTIONS.md](OPEN-QUESTIONS.md) | hal yang belum terverifikasi / keputusan slim build |
| [DEVICE-E2E-2026-11-19.md](DEVICE-E2E-2026-11-19.md) | hasil E2E app/daemon di device (apply, sleep/wake, mapping, saver, multi-window, R&D input suspend, true service off) |

## Raw data

- `research/raw/` — dump getprop, settings, thermal, power supply, logcat, catalog probe, state sampler.
- `research/rom/` — file ROM yang di-pull: `init.qcom.post_boot.sh`, perf XML, powerhint, thermal decrypt/dump, mcd config, perfd default values, framework jars.
- `research/apk/` — PowerKeeper, MISettings, PerformanceMode APK (untuk analisis lanjutan).
- `research/analysis/` — tabel hasil olahan (catalog, profil, overlap perf HAL).
- `research/scripts/` — script audit/probe/eksperimen read-only atau restore-safe.

## Kesimpulan awal

1. Arsitektur MiFineTune (Rust daemon + Kotlin thin client, JSON-lines stdio, single writer) tetap cocok.
2. **Bukan fork buta**: banyak asumsi MIUI 12 berubah, terutama:
   - tag logcat `am_*` -> `wm_*`;
   - `msm_performance` dapat menaikkan floor CPU saat wake/touch
     (interaction-scoped: wake boost ~3 dtk, touch blip ~1 dtk; sesi adb
     bisa floor-less) — lihat koreksi 2026-11-21;
   - `scaling_min_freq` di bawah floor live akan di-clamp (bukan error tulis);
   - refresh rate tetap butuh transaksi SurfaceFlinger (1035), bukan settings key saja;
   - tidak ada Joyose/GameTurbo package di build ini; ada `millet_monitor` + perf HAL 2.2;
   - `core_ctl` tetap di `cpu0/core_ctl` (bukan `policy0/core_ctl`), big cluster core_ctl disabled.
3. Nilai profil referensi tidak bisa disalin: contoh `balance` MiFineTune v0.16 memakai big cap 1324800; saat floor 1843200 aktif nilai itu di-clamp, dan engine merencanakannya `Locked`/retry setelah boost reda.
4. Kandidat profil harus dibangun dari constraint baru dan diverifikasi di device.

> Historical one-shot probes live in `scripts/historical-dangerous/`; some write forbidden charge nodes and must not be run on a daily-use device.
