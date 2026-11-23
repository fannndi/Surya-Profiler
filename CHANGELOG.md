# Changelog — Surya Profiler

> 2026-11-24: the `research/` tree was consolidated into `docs/`
> (`EVIDENCE-DEVICE`, `EVIDENCE-THERMAL`, `HISTORY`, `OPEN-QUESTIONS`);
> older entries below still name the original paths.

## 0.7.2 — 2026-11-24 (REA ownership map)

- Reverse-engineered the shipped MIUI/QTI components with REA (Ghidra for
  ELF, JADX for APK): `mi_thermald` (inotify on `sconfig` + the config
  dir, data-dir shadowing, map IDs < 65), `com.miui.powerkeeper`
  (ID space 0–24, offsets +30/+100/+200, performance-mode chain,
  per-game −2 °C virtual-sensor compensation list), `perfservice`
  (pure binder proxy), `power-hal` (reads `/vendor/etc/powerhint.xml`).
- Decoded the `perfboostsconfig.xml` hint inventory (sdmmagpie) and
  classified **26 catalog nodes as SHARED** with the perf HAL;
  `/dev/cpuctl` added to the forbidden guard.
- `thermal_scenario`: base 0–24, REA-confirmed offsets, `is_miui_sp_game`;
  diag gains `miui_sp_game` and the Thermal card explains the compensation.
- `docs/ROM-HARMONY.md` — the node-ownership map. E2E with a real game
  (Seven Knights): `sconfig=61`, no MIUI override, base re-assert, stock
  restore.

## 0.7.1 — 2026-11-23 (reference-repo audit)

- Forbidden guard +5 (live-verified): `sched_boost_top_app`,
  `sched_prefer_sync_wakee_to_waker`, UFS
  `1d84000.ufshc/{clkscale,clkgate,hibern8}_enable`.
- `crypt.rs`: differential vectors against the reference
  `mi-thermal-crypt` binary; `is_encrypted` + `decrypt_or_plain`
  (plaintext vendor maps tolerated).
- `doctor --fix-thermal`: foreign-file scanner/repair in
  `/data/vendor/thermal/config` (classification, module ships,
  `chattr -R -i` → remove → `restorecon -DFR`); CLI-only, daemon stopped.


## 0.7.0 — 2026-11-22 (SoC value optimization)

- **Game is now efficiency-first.** Measured on the Epic Seven title screen
  (60 fps, GPU-bound): the old aggressive CPU/sched/VM/cpuset profile spent
  **+2.6 % power for no fps gain** and made a CPU/GPU canvas stress page worse
  (p95 33 ms vs 17 ms). The new Game keeps full CPU/GPU caps, game touch boost
  (`input_boost_freq=0:1804800`), `top-app.prefer_idle` and the deadline I/O
  values, with stock CPU/sched/VM/cpuset values. Result: **−1.8 % power vs
  stock** at identical 60 fps, and the stress-page regression is gone.
- **Balance verified better than stock.** Chrome scroll, paired 3-minute
  runs: **−4.4 % power** at identical 60 fps and p95 (17.0 ms), because the
  profile caps CPU/GPU without touching frame pacing.
- **Game thermal headroom raised** (user-approved, kernel-limited):
  SS-CPU6 56/60 → 58/62 °C, SS-CPU0 58 → 60 °C, GPU 58/62 → 60/64 °C,
  CCC 61 → 63 °C. `MONITOR-BAT`, `MONITOR-BCL`, `MONITOR-TEMP_STATE` and LCD
  remain; kernel LMH/BCL is still the hard limiter.
- **R&D:** added a SurfaceFlinger frame-timing harness (real p50/p95/p99/jank
  for Chrome and GL game surfaces), a 3–5 minute A/B/B/A runner, and an
  SoC-node write/readback audit. Bus-DCVS/cgroup/sched/popp nodes are
  writable but produced no measurable win on this build, so they stay
  documented and uncataloged (slim-build rule).
- **Version:** crate/app 0.7.0 (versionCode 9).

## 0.6.0 — 2026-11-22 (correctness + guardrails)

- **Engine:** `core_ctl.min_cpus` now clamps against the `max_cpus` the same
  plan writes (sleep→wake no longer under-applies 2/6); caps locked by a
  transient MIUI boost are retried once after decay; `guard_path` rejects
  every non-catalog path (lexically normalized); `verify` fails on plan
  errors/all-locked; `restore` converges internally, skips stale stock keys,
  and fails closed on a corrupt baseline.
- **Daemon:** the worker owns exit cleanup and is joined before the daemon
  exits (no restore/apply race); stderr writes cannot abort the exit path;
  worker PID lock is create-new + cmdline-checked; CLI `apply` refuses to run
  beside the daemon; config parse/schema failures fail closed; bridge syncs
  are gated by `enabled`; stale peeks are ignored; terminal replies are not
  dropped by stdout backpressure.
- **Bridge/thermal:** saver/input holds re-assert on live drift and persist
  their restore point before writing; release is read-back verified; thermal
  follows the applied profile (not the decision), re-reads `sconfig` for
  MIUI preemption, yields before backoff, and treats 17/18 as camera
  scenarios; orphan `input_suspend=1` is released at startup.
- **App:** single-flight daemon start with a `hello` gate, synchronous
  restart stop, OFF→ON generation guard, late-event generation guard,
  Result-based config writes (fail closed, no silent overwrite), full
  diagnostics share, runtime labels in the notification, lazy app icons,
  and ~390 unused imports removed.
- **Guardrails:** `arch-check` fails oversized files and checks Kotlin
  protocol/config purity + protocol version parity; `check-assets` enforces
  binary version/lockstep; CI runs arch-check and the asset check; build.sh
  verifies assets; deploy doctor uses the app config path.
- **Device E2E (2026-11-22):** sleep→wake core_ctl 6/6, full Service OFF UI
  restore 18/18 (sconfig 0), force-stop restore, CLI single-writer gate,
  doctor deployed pack + stock baseline. Reboot remains from the 2026-11-21
  pass.
- **Version:** crate/app 0.6.0 (versionCode 8).

## Unreleased — modularization (2026-11-20)

- **Service OFF is gated on every hold.** A failed MIUI saver, input-suspend
  or thermal release now keeps the service ON (`ReleaseReport`); the
  `restored` event carries per-surface results.
- **Crash recovery:** `restore_pending` is persisted before the service is
  disabled; `surya-prof clean` and CLI `restore` release saver/input-suspend/
  thermal holds without a daemon.
- **Restore race closed:** the bridge gate closes at the IPC command and the
  blocked check is inside the state lock; `set_base` is rejected while
  restoring/disabled.
- **Thermal:** cleanup removes the map before configs, adopts marker-less
  owned maps, and uses an explicit write allowlist. Stale `sconfig=1` after
  performance mode off now reasserts the custom profile.
- **Structure:** Rust split into `platform`-style engine modules, `diagnose`,
  `daemon/run`, thermal/env/catalog/watcher/plan submodules. Kotlin gained
  pure `protocol` + `config` packages, typed events, one `ProfileId`, one
  `DaemonCommands`, `StockRestore`, and unit tests.
- **Guardrails:** `scripts/check.sh`, `build.sh`, `deploy.sh`, `arch-check.sh`,
  docs, LICENSE, top-level workspace guide. Host tests: **156 unit + 10 daemon
  E2E + 1 watcher**; Kotlin **15 JVM tests**.
- **Runtime hardening:** bounded stdout writer (backpressure cannot freeze the
  daemon), process timeouts for settings/getprop/dumpsys/logcat peeks, a
  `daemon.pid` single-writer guard, cached saver reads, and thermal select
  backoff + write-if-changed.
- **No background haptic:** the profile-apply `EFFECT_CLICK` vibration and
  the `VIBRATE` permission were removed. It fired on every `applied` event,
  including the 15 s in-place periodic confirmation, so the phone vibrated
  even with MIUI touch-vibration disabled.
- **Protocol v3:** `bypass_*` wire keys renamed to `input_suspend_*` (old
  `holds.json` is accepted via serde aliases).


## 0.5.0 — 2026-11-20 (MIUI thermal harmony)

- **Private thermal indices 60–63.** MIUI's PowerKeeper uses `sconfig` base
  IDs 20–25 (game-normal/video/demo/yuanshen) plus offsets +30/+100/+200/+500;
  its package map maps YouTube to 21, for example. The old custom indices
  20–23 could therefore be mistaken for MIUI scenarios (or vice versa). The
  four custom profiles moved to 60–63, with automatic migration of a v0.4.x
  takeover on the first sync. `mi_thermald` rejects map IDs above 64
  (verified on device), so 60–63 is the highest safe private range.
- **Vendor map generation.** The daemon decrypts
  `/vendor/etc/thermal-map.conf` (AES-128-CBC, same key as the configs),
  preserves every stock entry, adds 4K/8K fallbacks (`17` → `thermal-4k`,
  `18` → `thermal-camera`) when absent, and appends the four private entries.
  A future ROM entry is no longer lost by a hardcoded map.
- **MIUI scenario decoding.** `sconfig` is decoded into base scenario +
  performance/foreground/IEC flags. Yield policy: camera (12/17/18), phone
  (8), ARVR (15), and performance mode (1 or `thermal-global-mode` /
  `persist.sys.thermal.config`). Video/game/benchmark/navigation/unknown stay
  on the active Device Profile, so YouTube→Power Save and Game→Game remain
  effective.
- **Unknown-ID safety.** `mi_thermald` keeps the last config for an unknown
  `sconfig`; Service OFF now forces stock normal before removing the map.
  Cleanup accepts both legacy 20–23 and new 60–63 maps.
- **Diagnostics:** MIUI scenario label, performance/global mode, and the
  newest `mi_thermald` events (`thermal.dump` / `last_thermal.dump`) in a new
  card.
- **Doctor:** `thermal_takeover`, `miui_thermal_mode`, `thermal_log`,
  `battery_whitelist`, `appops_background`, and `millet_frozen` (the last
  three informational).
- **Settings:** live “Battery saver: no restrictions” check and request
  button (`REQUEST_IGNORE_BATTERY_OPTIMIZATIONS`).
- **Scheduler R&D:** `research/scripts/audit_walt.sh` and
  `research/SCHEDULER-MIUI14.md`; no scheduler profile values shipped until
  the device audit + A/B run.
- **Device E2E (2026-11-20):** 60–63 map/decrypt verified, YouTube `sconfig
  21` kept custom powersave 62, camera 12 yielded, Epic Seven 61, MIUI
  performance mode yielded to 1, daemon exit restored `sconfig=0` and stock
  normal. Evidence: `research/DEVICE-E2E-2026-11-19.md`.
- **Host tests**: 153 unit + 10 E2E; clippy/fmt clean.

## 0.4.2 — 2026-11-20 (universal base on app return)

- Surya Profiler is no longer treated as a transient foreground package. It is
  a normal, unmapped app, so returning to it from a mapped app (e.g. YouTube)
  applies the universal Device Profile base again.
- `config_changed` (sent on every app resume) and the mtime reload now use the
  effective foreground: a real current app wins; only SystemUI/IME/permission
  overlays fall back to the app underneath. A stale YouTube mapping can no
  longer re-apply `powersave` while the profiler is in front.
- The foreground watcher drops logcat ring-buffer history from before its
  spawn, so a service restart cannot replay an old mapped-app resume event.
- Dynamic OFF still forces the base for every app; sleep, multi-window,
  MIUI-saver follow, thermal takeover and charging behavior are unchanged.
- **Host tests**: 130 unit + 9 E2E; clippy/fmt clean.

## 0.4.1 — 2026-11-20 (doctor battery-service check)

- `surya-prof doctor` adds `battery_service`: detects Android
  `BatteryService` overrides (`UPDATES STOPPED` from `dumpsys battery
  unplug/set ac`) and compares its level with the sysfs fuel gauge. A stale
  status bar is reported with `dumpsys battery reset` as the hint.
- No tuning/charging behavior change; informational check only.
- **Host tests**: 126 unit + 8 E2E; clippy/fmt clean.

## 0.4.0 — 2026-11-20 (custom thermal takeover)

Per-profile MIUI thermal takeover, driven by the extracted ROM and the
decrypted `mi_thermald` configs.

- **Custom thermal configs**: balance/game/powersave/sleep, installed as
  encrypted files under `/data/vendor/thermal/config` with a custom
  `thermal-map.conf` (private `sconfig` indices 20–23). Vendor files are never
  modified; stock entries remain the fallback.
- **Automatic**: the active Device Profile selects its thermal config; sleep
  selects the sleep config. Unknown/stock states fall back to MIUI.
- **MIUI scenario respect**: camera/phone/ARVR and any unknown `sconfig` are
  yielded to; the daemon reasserts the profile when MIUI returns to normal.
- **Service OFF**: removes marker-owned files, restores the captured
  `sconfig`, and gates the stop on cleanup success. `surya-prof thermal-clean`
  handles crash leftovers; a boot-time `0` always maps to stock normal.
- **Direct sensor**: CPU/GPU steps read `nvm-therm-adc` directly (the MIUI
  `VIRTUAL-SENSOR` is rejected on this boot); thresholds are floor-aware
  (big 1.8432 GHz / little 1.6128 GHz screen-on floors).
- **No charging changes**: every config keeps the stock `MONITOR-BAT`
  charging-current ladder unchanged. `input_suspend` remains R&D-only.
- **UI**: Settings → Advanced → “Custom thermal profiles” (default on);
  Diagnostics shows takeover state, active custom profile and yield reason.
- **Host tests**: 123 unit + 8 E2E; clippy/fmt clean.

## 0.3.0 — 2026-11-20 (thermal awareness)

Read-only visibility into MIUI's thermal policy, based on the extracted
`V14.0.2.0.SJGIDXM` ROM and the decrypted `mi_thermald` configs.

- **Telemetry**: `EnvSnapshot` now carries `thermal_sconfig` + resolved mode
  (`normal`/`phone`/`tgame`/`nolimits`/`camera`/`arvr`), `temp_state`,
  `screen_state`, quiet/NVM temperatures, CPU6/CPU7 cooling states and the
  effective little/big policy caps.
- **Diagnostics**: new "Thermal (MIUI)" card; the Home diagnostics row shows
  the current thermal mode.
- **Doctor**: `env_thermal_config` + `env_thermal_caps` checks.
- **Research**: `research/THERMAL-MIUI14.md` documents the encryption key,
  `sconfig` map, thresholds per config and the live device evidence.
- **No tuning change**: thermal nodes remain MIUI-owned; the app never writes
  `sconfig`, `temp_state` or `thermal-*.conf`.
- **Host tests**: 115 unit + 8 E2E.

## 0.2.0 — 2026-11-20 (slim build)

Intent: keep only Device Profile + Apps Profile, remove the experimental
surface, and make Service OFF a true stock restore.

- **Profiles**: 3 user profiles (Power Save, Balance, Game) + hidden Sleep.
  `boost` removed from the pack, CLI, config and UI.
- **Apps Profile**: one mapping per app (Default / Power Save / Balance /
  Game); `app_profiles`, bypass/DND/refresh fields and the detail screen are
  gone.
- **Config schema 2**: `enabled`, `dynamic`, `base_profile`, `app_map`,
  `rnd_input_suspend`. A schema-1 file is normalized on first load
  (`app_profiles[*].profile` is folded into `app_map`).
- **Service OFF = true stock restore**: release MIUI saver + R&D input
  suspend, write `stock.json` back, verify, then stop. A failed/unconfirmed
  restore keeps the service ON. No reboot. The daemon also restores on a
  clean exit while a profile is active.
- **MIUI saver follow** kept (Power Save-mapped app, visible/unlocked, not in
  split screen); it now restores the user's `low_power` value on release.
- **Multi-window** kept, simplified to the `dumpsys window` probe (the
  ProcessMonitor stream was silent for a real split).
- **R&D `input_suspend`**: one global Settings switch, per-session, released
  and cleared on Service OFF. `battery_charging_enabled` is forbidden.
- **Removed**: DND, refresh-rate follow + SF 1035 + dex helper, charge limit /
  charge-once, bypass policy + floor, battery/thermal guards, quiet hours,
  backup/import, jank boost, storage maintenance, MIUI performance mirror,
  game-mode checker, ultra-saver retire, app-side drift guard, biometric gate,
  usage-stats foreground fallback.
- **Host tests**: 114 unit + 8 E2E; clippy/fmt clean; Android debug APK
  builds.
- **Device E2E 2026-11-20**: 3 profiles, sleep/wake, app mapping, saver
  follow, multi-window (Balance, no saver), automation `SET_PROFILE`, R&D
  input suspend, true Service OFF restore, no reboot. Evidence:
  `research/DEVICE-E2E-2026-11-19.md`.

## 0.1.0 — 2026-11-19

First MIUI 14 / Android 12 port, derived from the MiFineTune architecture but
re-audited on the device.

- Rust `surya-prof` engine + daemon: 95-node catalog, MIUI 14 `FreqMax`
  floor lock, guard, snapshot/stock, apply/verify/restore, JSON-lines daemon.
- Profiles: Power Save, Balance, Game, Sleep, Boost.
- Android app: Compose Home/Apps/Settings/Diagnostics, FGS lifecycle,
  daemon client, QS tile, automation receiver, backup, biometric gate.
- MIUI 14 watchers: `wm_resume_activity` / `wm_set_resumed_activity`.
- Bridge: saver follow, per-app bypass/DND/refresh, charge guard, holds.
- Startup seed: logcat `wm_*` peek falls back to `dumpsys activity
  activities`; a cold event buffer applies the base instead of idling.
- Device E2E: profiles, sleep/wake, per-app mapping, refresh 60→90, DND,
  charge limit, automation, tile/notification stop. See
  `research/DEVICE-E2E-2026-11-19.md`.
- Host tests at the time: 148 unit + 16 E2E.
