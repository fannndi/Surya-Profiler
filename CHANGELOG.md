# Changelog — Surya Profiler

> 2026-11-24: the `research/` tree was consolidated into `docs/`
> (`EVIDENCE-DEVICE`, `EVIDENCE-THERMAL`, `HISTORY`, `OPEN-QUESTIONS`);
> older entries below still name the original paths.

## 0.11.0 — 2026-11-26 (Power usage + Phase 0 safety/telemetry integrity)

- Startup foreground peek keeps mapped apps active across daemon restarts;
  peek sequencing prevents older wake/unlock results from winning.
- Periodic foreground fallback repairs a hung or cleared logcat stream.
- Lock screen returns to the universal base instead of retaining a Game
  mapping; bridge saver attribution is updated before slow thermal work.
- Forced reconciliation preserves stricter external thermal caps.
- Decoded MIUI offset scenarios (`112`, `108`, `117`, `118`, …) survive thermal
  release/cleanup and are no longer replaced with stock normal.
- Unowned `input_suspend=1` is released before a new R&D hold can capture it;
  hold persistence is fail-closed and atomically fsynced.
- PID-file create/unlink race replaced by an OS-backed `flock` on
  `daemon.lock`; concurrent startups have exactly one winner.
- Power sampling uses actual elapsed time, wakes on screen changes, joins
  before final flush, merges same-minute partials without data loss, dedupes
  segment replays, reconciles peaks, and makes Clear fail-closed. Legacy
  duplicate-minute files are healed on load; a zero-duration first reading
  contributes extrema/peaks without fabricating represented time.
- Power protocol adds backward-compatible segment bounds/energy/ID; the app
  resubscribes after daemon reconnect and rejects malformed snapshots.

- **Power usage screen (read-only).** Home → Power shows live battery-side
  current, watt, voltage, temperature, charger input (when the kernel exposes
  a rail), battery health and warnings for `input_suspend=1`,
  `battery_charging_enabled=0`, or a status/current mismatch. No charge
  control is added; the fast sampler never writes a sysfs/proc node.
- **Daemon-owned history.** A dedicated sampler reads battery/charger sysfs
  every **3 s screen-on / 30 s screen-off** and aggregates **one-minute
  buckets** into append-only `power-history.jsonl` (7-day retention,
  all-time `power-peaks.json`). A clean exit closes the in-progress bucket;
  SIGKILL loses at most one minute. Clear deletes both files and resets the
  in-memory ring.
- **Attribution.** Buckets carry the active Device Profile (including Sleep)
  and the last real foreground package only while the screen is on and
  unlocked. The app derives time-weighted per-profile and per-app averages
  (apps need 2+ minutes), charge sessions (added mAh, duration, peak) and the
  all-time peak tiles.
- **Protocol (v3, additive).** `power {watch, window_s?}`, `power_clear`;
  events `power` (snapshot: `window_s`, `live`, `current_bucket`, `history`,
  `peaks`) and `power_live` (`sample`, in-progress `bucket`). Window clamped
  10 min–7 d; `power` replies use the reliable path.
- **UI.** Live card, charge-session card, signed power chart (1h/6h/24h,
  positive = usage / negative = charging, screen-off strip), peak record,
  by-profile and by-app lists, clear confirmation, and a service-off CTA.
  Formatting/aggregation lives in pure Kotlin (`PowerFormat`/`PowerAggregate`)
  with JVM tests.
- **Internal split.** `daemon/proto.rs`'s stdout `Publisher` moved to
  `daemon/publisher.rs` and the PID guard to `daemon/instance.rs` to keep the
  file budget (no behaviour change).
- Host: **264 Rust unit tests**, **21 daemon E2E** and **54 Kotlin JVM
  tests** pass; fmt, clippy, architecture guardrails and APK build are clean.
  Coverage includes startup/peek sequencing, thermal offset ownership,
  write-ahead hold persistence, the single-writer lock, elapsed-time power
  aggregation, legacy history healing, segment replay deduplication,
  reconnect/resubscribe and fail-closed Clear.
- Device E2E (surya, 2026-11-26): live 797 mA / 3.51 W at 85% matched sysfs
  (`current_now=-798645 µA`, `voltage_now=4403272 µV`); `usb (USB_CDP)`
  reported via `uevent`; `input_suspend=1` showed the bypass warning and
  restored to 0; screen-off Sleep bucket (2 samples/60 s), per-app/profile
  rows, peaks, Clear (files deleted, live continues), history survived
  Service OFF/ON, and the full 19/19 stock restore kept the Power CTA while
  off. Phase 0 final build (`b14e4fe1…` / APK `6db01e77…`) additionally
  verified legacy history healing (70 rows → 67 minutes), Power reconnect
  after SIGKILL, crash-hold recovery, camera yield and preservation of MIUI
  `sconfig=12` through Service OFF, all holds released and `doctor ok:true`.
  Evidence: `docs/HISTORY.md`.

## 0.10.0 — 2026-11-24 (per-app render scale + optional display scale)

- **Per-app render scale (opt-in, default OFF).** Uses the Android 12
  GameManagerService shell interface (`cmd game downscale <ratio> <pkg>`)
  through `bridge/downscale/`: the app renders into a uniformly smaller
  buffer while its dp layout, aspect ratio and density stay unchanged (only
  DPR shrinks; verified with Chrome: `inner=432x815`, DPR 2.5→1.25 at 0.5,
  `mCompatFrame=540x1200`, `mGlobalScale=2.0`). Captures the prior
  PlatformCompat override from `dumpsys platform_compat` and restores it on
  unmap/Service OFF/exit (`downscale.json`, `downscale_ok`). Persistent while
  Service ON so apps start already scaled (a live change can relaunch
  Chrome); a foreground re-check re-asserts MIUI Game Turbo drift.
- **Optional global display scale (default OFF).** `bridge/display/` sets
  exact 20:9 `wm size` + density presets (80/75/70/HD 720p standard/HD
  compact/60/50), captures the user's exact size/density overrides (this
  device: no size override, density 400), and restores them on Off/Service
  OFF/exit (`display.json`, `display_ok`). `hd` = 720x1600 @320 (vendor 720p
  density, larger/readable UI), `hd-compact` = 720x1600 @267 (same 431dp
  layout as native 400/1080).
- **R&D evidence (60 s samples, brightness 400, 60Hz, input_suspend):**
  Chrome WebGL stress 2209.6→1564.0 mW (−29%) with native+0.65, and
  2209.6→1309.8 mW (−41%) with 720p+0.8; Seven Knights
  2781.1→2039.3 mW (−27%, native+0.8) and 2781.1→1789.3 mW (−36%, 720p).
  TikTok was neutral (1911.8→1912.8 mW; GPU ~8%→4.4%): video decode/network/
  panel dominate, so render scale is not a doomscrolling power fix. Density
  is a UI-scale knob, not a performance knob (720p buffer is 720x1600 at any
  density).
- **UI:** Apps Profile bottom sheet gains “Render scale” (Off/90/80/75/70/
  65/60/55/50) with the effective buffer size and app-row badge; Settings
  gains “Display scale (system-wide)” with the exact presets and a warning.
- **Config:** schema-2 additive `app_downscale` map and `display_scale`
  string; `downscale_ok`/`display_ok` in `restored`; diag holds expose
  downscale/display state.
- Host: unit tests (ratio canonicalization, PlatformCompat parser, 20:9
  presets, wm parser) + 2 daemon E2E (downscale capture/apply/restore with
  prior 0.8; display apply/Off exact density restore).
- Device E2E (surya, 2026-11-24): Seven Knights with per-app 0.8 reports
  `mCompatFrame=864x1920` / `mGlobalScale=1.25` while `mFrame` stays
  `1080x2400`; display `hd` sets `wm size 720x1600` + density 320 and the
  combo reports `mFrame=720x1600` / `mCompatFrame=576x1280`; force-stop exit
  restores `wm` 1080x2400 @400, `cmd game downscale disable`, empty state
  files and `downscale_ok/display_ok=true`.

## 0.9.0 — 2026-11-24 (opt-in thread placement + reference hardening)

- **Thread placement (Fase 6, default OFF).** While an app mapped to
  **Game** is in front, the daemon pins the critical render-path threads to
  the gold cores and restores the captured affinity when the app leaves, the
  screen turns off, the phone locks, or the service stops:
  `UnityMain`/`GameThread` -> cpu6–7 (`c0`), `UnityGfx*`/`RenderThread`/
  `RHIThread` -> cpu6 (`40`); worker/audio threads are left alone. Threads
  are matched by kernel `comm`, the target must be the package's own main
  process (uid >= 10000), and a mask is only written when it is a subset of
  the thread's live `Cpus_allowed_list` (never widen). Per-tid restore points
  live in `affinity.json`; a SIGKILL is repaired at the next daemon start.
  New Settings switch, config key `thread_affinity` (schema 2, additive),
  `restored.affinity_ok`, and a Diagnostics row. Host E2E fakes `/proc` +
  `taskset`; device E2E below.
- **Thermal repair after a failed select.** A corrupted owned config (marker
  survived, bytes did not) now triggers one `install_all()` rewrite before
  the normal backoff; `mi_thermald` reloads through its config-dir inotify.
- **Doctor: config-path corruption + UFS health.** A
  `/data/vendor/thermal/config` that became a file is reported
  (`config_is_dir=false`) and `--fix-thermal` rebuilds it (`chattr -i` →
  remove → mkdir). `diag`/`doctor` gain read-only UFS health
  (`life_time_estimation_a`, `eol_info`); Diagnostics shows the decoded
  values.
- **Docs corrected.** The thermal README no longer claims MIUI's
  `VIRTUAL-SENSOR` is unusable: vendor defines it as a weight-1 alias of
  `nvm-therm-adc`, which our configs address directly. `mi-thermal-crypt`
  has no explicit license file; our implementation is independent and
  verified with differential vectors.
- Host: Rust unit tests + 2 new daemon E2E (placement on/off, restore on
  leave + Service OFF); Kotlin config/diag tests extended.
- Device E2E (surya, 2026-11-24): Seven Knights mapped to Game → `sconfig=61`
  and `RenderThread`/`UnityGfxDeviceW` pinned to cpu6 (`UnityMain` was
  already on 6-7 without our write); leave → 4/4 released, `affinity.json`
  empty, `sconfig=60`; daemon exit (force-stop) → `affinity_ok=true`, same
  restore. Corrupt `thermal-game.conf` → select failure logged, configs
  reinstalled, game loaded on the retry. Doctor `storage_health` =
  `0x06`/`0x01`, overall `ok=true`.

## 0.8.0 — 2026-11-24 (freq floor audit + observability)

- **Freq floor audit sentinel (default ON, screen-off scope).** Reports of
  an idle CPU pinned at a high frequency traced back to
  `scaling_min_freq`: the profiles write the floor low, so a high floor
  means an external writer. Device measurement (2026-11-24) settled the
  model: MIUI's screen-on floor is a bounded overlay (wake ~10 s, app
  launch ~40 s) that releases on its own — fighting it on screen would
  cut legitimate boosts. So the audit acts only while the screen is
  OFF (no interaction is possible there): a floor above the active
  sleep profile for >30 s is logged with full context and re-asserted
  through the normal forced apply (closing the real gap: in-place
  applies skip drift and the watchdog only runs every 30 min). Yields
  to MIUI performance mode; a non-sticking re-assert backs off
  (30 s→300 s cap) and reports "external floor holder suspected".
  Screen-on drift is logged once per episode, never fought. Heals flow
  into the auto-revive stats.
- **Levelled logging infra** (`core/src/logging.rs`): ring buffer (512
  lines) + rotating file sink (`<state_dir>/logs/daemon.log`, 512 KB,
  one generation) + stderr mirror for the app's logcat relay. Levels via
  `SURYAPROFILER_LOG` and the config `log_level` (applied live on
  reload); components tag every line (`daemon`, `apply`, `floor`, ...).
  New diagnostics: worker per-key failures (`apply` at debug/warn).
- **`diag` gains `recent_logs`** (~120 newest lines) — the app seeds its
  Diagnostics log view from the durable ring. Settings gains a
  "Freq floor audit" switch (ConfigCodec schema-2 additive keys:
  `freq_floor_audit`, `log_level`).
- **CLI:** `surya-prof log [--last N] [--level L] [--follow]` (durable
  log tail) and `surya-prof debug env` (read-only live freq/thermal
  report: live vs profile floors, stock values, governors, thermal
  mode — the one-shot "why is my CPU frequency high" answer).
- Sampler: `EnvSnapshot` gains `little/big_min_mhz` (the floor was
  invisible before) and the active governors.
- Host tests: floor-audit E2E (sleep drift → heal → stats; screen-on
  drift noted, never fought), logging rotation/filtering, freq-audit
  state machine unit tests, `recent_logs` in diag.
- Device E2E (surya): injected a stuck sleep floor (1017600/979200) —
  healed in ~40 s with the full log story; `log`/`debug env` verified
  on device; force-stop → stock restore (36 keys, 0 failed).
  Evidence: `docs/HISTORY.md` (2026-11-24).

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
