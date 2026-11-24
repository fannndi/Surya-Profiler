# AGENTS.md — Surya Profiler

Device: **POCO X3 NFC, surya / sm6150 / SDMMAGPIE, MIUI 14 V14.0.2.0.SJGIDXM,
Android 12 / SDK 31, kernel 4.14.190, root APatch**.

Read the R&D evidence before changing the engine/catalog (consolidated
2026-11-24 — the old `research/` tree now lives under `docs/`):

- `docs/EVIDENCE-DEVICE.md` — device facts, MIUI deltas, runtime behavior,
  scheduler R&D, log signals, the full 95-entry catalog probe
- `docs/EVIDENCE-THERMAL.md` — thermal evidence, catalog status, profile
  design, the reference value matrix and the perf-HAL resource overlap
- `docs/ROM-HARMONY.md` — who owns every node (writer chains, shared
  surface, boost hint inventory)
- `docs/HISTORY.md` — dated device-E2E and RE-session evidence (full text)
- `docs/OPEN-QUESTIONS.md` — the live backlog

## Doc map

| File | What it is |
|---|---|
| `README.md` | project landing (GitHub) |
| `AGENTS.md` | this file — the agent/LLM entry point |
| `CHANGELOG.md` | version history |
| `docs/ARCHITECTURE.md` | layering rules and data flow |
| `docs/PROTOCOL.md` | daemon ↔ app JSON-lines protocol |
| `docs/THERMAL.md` | thermal takeover design + REA-confirmed internals |
| `docs/ROM-HARMONY.md` | node ownership map (shared/forbidden) |
| `docs/DEVICE-E2E.md` | the device E2E checklist to run per release |
| `docs/EVIDENCE-*.md`, `docs/HISTORY.md`, `docs/OPEN-QUESTIONS.md` | R&D evidence (read-only; do not edit without adding a dated section) |
| `tools/` | one-off R&D scripts + `perf-hal-runtime-writers.txt` |

## Architecture in one paragraph

The Rust daemon (`core/src/daemon/`, started as `surya-prof serve`) owns every
decision and every write. The Kotlin app is a thin client: Compose UI,
foreground service + notification, `su` process plumbing, and Android-only
signals (screen/keyguard). They talk JSON-lines over stdio; `config.json`
(app-owned, in `filesDir`) is the only shared file for user intent. The engine
(`core/src/engine/`) is the only code that writes parameters, always through
the catalog guard.

## Hard rules

1. **MIUI 14 floor** — never write `msm_performance`, `sched_boost`, or any
   `cpu_boost` parameter other than `input_boost_freq`. The live screen-on CPU
   floor (typically 1612800 little / 1843200 big) belongs to MIUI's power
   policy. It is **interaction-scoped**, not a constant screen state: a wake
   boost lasts ~3–10 s and an app-launch boost ~40 s (measured 2026-11-24;
   MIUI *restores the previous floor* on release), while adb-driven sessions
   can run floor-less. Always use the live `scaling_min_freq`, never a
   hardcoded floor.
2. **Max ≥ floor** — a `scaling_max_freq` below the live `scaling_min_freq`
   is silently clamped by the kernel. The engine plans it as `Locked` and
   retries once after a transient boost decays; do not write a value that can
   never read back during a persistent floor.
3. **Single writer** — all writes go through the daemon (or the CLI while the
   service is off). The app never writes sysfs/proc directly.
4. **Harmony guard** — only Free/Baseline catalog nodes; `guard_path` rejects
   any path that is not a catalog entry (plus the documented `ALLOWED_EXACT`
   audit list) and framework-owned prefixes on every profile write. Thermal
   takeover uses its own explicit allowlist. Never touch SELinux,
   `persist.sys.aries.*`, or vendor dfps props.
5. **Watchers** — foreground tags are `wm_resume_activity` /
   `wm_set_resumed_activity`. A cold/cleared event buffer falls back to
   `dumpsys activity activities`; the first decision of a run applies the base
   even when the foreground is transient. Surya Profiler itself is **not**
   transient: it is a normal unmapped foreground app, so leaving a mapped app
   for the profiler/launcher returns to the universal base. Only SystemUI,
   IMEs and permission dialogs are overlays. A real `last_fg` always wins over
   `last_real` for config/unlock decisions.
6. **Multi-window** — `ProcessMonitor` is silent for a real split on this
   build. Use the `dumpsys window` probe (`visible=true
   mode=split-screen-primary|secondary` + `A=<uid>:<pkg>`), on foreground
   changes and every 5 supervisor ticks (15 s).
7. **MIUI saver follow** — only for a Power Save-mapped app in front, visible
   and unlocked, and never in split screen. Capture the user's `low_power`
   value and restore it on release.
8. **R&D input suspend** — the only charge node the app may write is
   `input_suspend`, through the single global Settings switch. Capture/restore
   the node; never touch `battery_charging_enabled`.
9. **Service OFF = true stock restore** — release holds, write `stock.json`
   back, verify, then stop. A failed/unconfirmed restore keeps the service ON.
   No reboot. The daemon also does this on a clean exit while a profile is
   active.
10. **Kernel invariants** — keep `ordered_pairs_with` (migrate pairs) and the
   pass-2 re-plan; `core_ctl` big cluster is disabled on this device.
11. **Asset sync** — `assembleDebug` runs `syncCore`; keep
   `core/profiles.json` and the release binary in `app/src/main/assets/` in
   lockstep.
12. **Device testing is mandatory** for engine/daemon changes.
13. **Thermal takeover is profile-scoped and reversible** — the daemon may
    install its four encrypted configs under `/data/vendor/thermal/config`
    with private `sconfig` **60–63**, and select them in lockstep with the
    active Device Profile. The custom map is generated from
    `/vendor/etc/thermal-map.conf` (stock entries preserved, 4K/8K fallbacks
    added when absent). `mi_thermald` rejects map IDs >64; do not move the
    private range above 64. Never modify `/vendor`, never fight MIUI camera
    (12/17/18), phone (8), ARVR (15) or performance mode (1 /
    `thermal-global-mode` / `persist.sys.thermal.config`); video/game/
    benchmark keep the active custom profile. Keep the stock `MONITOR-BAT`
    charging ladder, and remove the marker-owned files + restore `sconfig` on
    Service OFF. `temp_state` is an output, never a selector. Screen-on
    floors (big 1.8432 GHz, little 1.6128 GHz) clamp lower thermal targets;
    use floor-aware steps.
14. **Thread placement is process-level, opt-in, and never widens** — the
    only non-sysfs write is `taskset` through `bridge/affinity/` (default
    off). Target only a Game-mapped app's own main process (uid ≥ 10000),
    only `UnityMain`/`GameThread` → cpu6–7 and `UnityGfx*`/`RenderThread`/
    `RHIThread` → cpu6, only when the mask is a subset of the live
    `Cpus_allowed_list`. Capture the per-tid mask in `affinity.json`, restore
    on leave/screen-off/lock/Service OFF/exit, and yield sticky when the app
    re-pins a thread. Never touch system processes or widen a mask.

## File map (keep current)

### Rust (`core/`)

| Path | Responsibility |
|---|---|
| `src/lib.rs` | module index + `DEFAULT_PROFILES_JSON` |
| `src/logging.rs` | levelled logger: ring buffer + rotating file + stderr mirror |
| `src/main.rs` | binary entry (delegates to `cli.rs`) |
| `src/cli.rs` + `src/cli/commands.rs` | CLI: probe/profiles/plan/apply/restore/clean/verify/status/serve/doctor/thermal-clean/catalog |
| `profiles.json` | 3 user profiles + hidden sleep |
| `src/engine/catalog/mod.rs` | tiers, kinds, lookup, `guard_path` |
| `src/engine/catalog/entries/` | parameter registry split by cpu/gpu-io/kernel/misc |
| `src/engine/catalog/forbidden.rs` | framework-owned paths / ALLOWED_EXACT |
| `src/engine/probe.rs` | read-only capture + option lists |
| `src/engine/env/` | telemetry model/parsers/sampler/scenario/sysfs |
| `src/engine/thermal_scenario/` | PowerKeeper `sconfig` ID/offset decoder + SP-game list |
| `src/engine/thermal_log.rs` | `mi_thermald` dump parser |
| `src/engine/crypt.rs` | AES-128-CBC encrypt/decrypt for thermal configs |
| `src/platform.rs` | process runner with timeout/backpressure |
| `src/engine/plan.rs` + `plan/` | plan, statuses, kernel-invariant pair checks, write order |
| `src/engine/validate.rs` | per-kind validation; MIUI 14 max-vs-floor lock |
| `src/engine/readback.rs` | read-back rules + snapshot normalization |
| `src/engine/apply/` | apply/restore/verify + atomic store |
| `src/daemon/` | arbiter, worker, watchers, bridge, settings, env, stats |
| `src/daemon/freq_audit.rs` | freq floor audit sentinel (external floor drift detection) |
| `src/daemon/bridge/` | saver, R&D input-suspend, thermal takeover + holds |
| `src/daemon/commands.rs` | command dispatch + foreground/peek plumbing |
| `src/daemon/evaluate.rs` | decision point + 3 s supervisor |
| `src/daemon/worker.rs` | coalescing apply worker |
| `src/daemon/bridge/thermal/` | profile/paths/io/perf/map/install/select/sync/release/files |
| `src/daemon/bridge/affinity/` | opt-in Game thread placement (rules, `/proc`, `taskset`, restore) |
| `src/diagnose/` | `doctor` self-check (battery/miui/env/thermal/thermal-files) + `debug env` |
| `assets/thermal/` | plaintext balance/game/powersave/sleep configs (encrypted at runtime) |
| `tests/` | host E2E: protocol, foreground watcher, multi-window probe |

### Kotlin (`app/`)

| Path | Responsibility |
|---|---|
| `MainActivity.kt` | single ComponentActivity Compose host |
| `core/RootBridge.kt` | `su` exec + asset deploy |
| `core/FtClient.kt` / `Tuner.kt` | CLI client for service-off/restore paths |
| `protocol/` | ProfileId, commands, typed events, diagnostics parsers |
| `config/` | AppConfig + pure ConfigCodec |
| `dynamic/DynamicProfileService.kt` | FGS lifecycle + Android signals + restore-on-stop |
| `dynamic/DaemonClient.kt` | daemon spawn, JSON-lines, stderr relay |
| `dynamic/DynamicProfileConfig.kt` | atomic schema-2 `config.json` writer |
| `dynamic/DynamicProfileState.kt` | StateFlows mirrored from events |
| `dynamic/DaemonNotifications.kt` | channel + notification builder |
| `dynamic/AutomationReceiver.kt` | `SET_PROFILE` only |
| `ui/` | Home, Apps, Settings, Diagnostics, cards/dialogs, theme |

## Build

```bash
./scripts/check.sh     # fmt, clippy, Rust tests, Kotlin tests, APK
./scripts/build.sh     # cross-build arm64 + APK
./scripts/deploy.sh    # adb install + device health summary
```

Architecture: `docs/ARCHITECTURE.md`; protocol: `docs/PROTOCOL.md`; thermal:
`docs/THERMAL.md`; device matrix: `docs/DEVICE-E2E.md`.

## Testing conventions (LLM: follow these exactly)

- **Production files contain zero test code.** Every module's unit tests
  live in a sibling `<name>/tests.rs` (`daemon/proto.rs` + `proto/tests.rs`)
  or, for directory modules, in the module dir (`env/tests.rs`). The
  arch-checker only exempts `*/tests.rs` from the 300-line cap — keep it
  that way instead of moving logic into test files.
- `testutil.rs` / `test_util.rs` are `cfg(test)`-gated fixtures INSIDE the
  crate (unit tests need crate-private access; they cannot move to
  `core/tests/`). They build the one fake surya device.
- Host E2E lives in `core/tests/` (`daemon_smoke.rs`, `daemon_watchers.rs`,
  `common/`, `fixtures/`) and runs without a device using the env vars in
  the table below.
- Kotlin tests are JVM-only in `app/src/test/`; no tests inside
  `app/src/main`.
- Run everything with `./scripts/check.sh`; it also enforces asset
  lockstep and the architecture rules.

### Adding a catalog node (checklist)

1. Confirm nobody else writes it: check `docs/ROM-HARMONY.md` (writer
   chains + shared list) and `docs/EVIDENCE-THERMAL.md` (perf-HAL
   overlap). Framework-shared nodes stay shared (Baseline + reconcile),
   never forbidden.
2. Live-verify on device: existence, permissions, owner, fresh mtimes
   (`adb shell su -c ...`), and add the evidence to `docs/HISTORY.md`
   (dated section).
3. Add the entry to `core/src/engine/catalog/entries/` (correct file by
   kind) — `guard_path` rejects anything not cataloged, so this IS the
   allowlist.
4. Add kind-specific validation/plan/readback handling only if the value
   shape is new.
5. `cargo test --lib`, then device E2E (`docs/DEVICE-E2E.md`).

### Device E2E quickstart

`./scripts/build.sh && ./scripts/deploy.sh`, then follow
`docs/DEVICE-E2E.md`. Mandatory for engine/daemon changes (hard rule 12).

## Host simulation env vars

| Env var | Replaces |
|---|---|
| `SURYAPROFILER_LOGCAT_BIN` | `/system/bin/logcat` |
| `SURYAPROFILER_DUMPSYS_BIN` | `/system/bin/dumpsys` (peek + split probe) |
| `SURYAPROFILER_SETTINGS_BIN` | `/system/bin/settings` (saver flag) |
| `SURYAPROFILER_SYSFS_ROOT` | `/` (fake tree) |
| `SURYAPROFILER_LOG` | daemon log level (default `info`; config `log_level` overrides live) |
| `SURYAPROFILER_ENV_SAMPLE_MS` | 30 s sample cadence |
| `SURYAPROFILER_WATCHDOG_SECS` | 1800 s reconcile cadence |
| `SURYAPROFILER_SETTLE_MS` | 400 ms worker coalescing window |
| `SURYAPROFILER_SLEEP_DELAY_MS` | 10 s screen-off grace |
| `SURYAPROFILER_DEBOUNCE_MS` | 300 ms foreground debounce |
| `SURYAPROFILER_FLOOR_RETRY_MS` | 3500 ms one-shot retry after a transient MIUI boost (`0` disables) |
| `SURYAPROFILER_AUDIT_INTERVAL_MS` | 15000 ms freq floor audit sampling cadence |
| `SURYAPROFILER_AUDIT_GRACE_MS` | 30000 ms how long a high floor persists before the audit acts |

## Device E2E status

Passed 2026-11-19/20 (surya, MIUI 14): root/deploy/daemon, profile apply,
sleep/wake, per-app mapping, saver follow, automation, cold-buffer startup
fallback, multi-window probe, R&D input suspend, and true Service OFF stock
restore (no reboot). 0.4.2 additionally verified the universal base on return
from mapped apps (YouTube → Surya/launcher/WhatsApp) and on service restart.
0.5.0 device validation (2026-11-20) covers the 60–63 index migration,
YouTube video keep-custom, camera yield, performance-mode yield, thermal
event log/doctor, and daemon exit restore. 0.6.0 (2026-11-22) re-verified
the full Service OFF UI path (18/18 written, 0 failed, `sconfig=0`), the
sleep→game `core_ctl` 6/6 wake fix, force-stop/stdin-EOF restore, the CLI
single-writer gate, and the deployed-pack/stock `doctor` checks. 0.7.0
(2026-11-22) verified the efficient Game profile through the real daemon:
Epic Seven mapping → `active=game`, `sconfig=61` with the raised thermal
trips; Service OFF → `active=null / last_mode=restore`, `sconfig=0`, stock
CPU, all holds released. 0.7.1/0.7.2 (2026-11-23/24) verified the
foreign-file scanner E2E (`doctor --fix-thermal`), the REA-derived
ownership map, and a full real-game run with Seven Knights
(`sconfig=61`, no MIUI override, base re-assert, stock restore). 0.9.0
(2026-11-24) verified opt-in thread placement (Seven Knights: RenderThread/
UnityGfxDeviceW → cpu6, leave/daemon-exit restore, `affinity.json` empty),
the corrupt-config thermal self-repair, and doctor UFS health. Reboot
remains covered by the 2026-11-21 run.
Evidence: `docs/HISTORY.md`.

## Known open items

- Multi-window uses a `dumpsys window` probe; re-check after MIUI updates
  (task-line shape may change).
- The `FolkPatch/APatch` superuser must grant the app once; before that the
  first launch shows a deploy error and `ON_RESUME` retries after the grant.
- QS tile placement is not exercised on device (the tile's STOP action path
  is tested through the service).
- A daemon crash (SIGKILL) cannot run the exit restore. Holds persist in
  `holds.json` and are recovered on the next start; `surya-prof restore` is
  the repair path for leftover tuning.
- A Service OFF restore always re-writes three representation-normalized
  keys (`io.scheduler`, `core_ctl.busy_up_thres`, `core_ctl.busy_down_thres`);
  this is idempotent and verified, not drift.
- Thermal takeover is opt-in via `custom_thermal` (default on). A failed
  cleanup keeps the service ON; `surya-prof thermal-clean` repairs a crash
  leftover. Thresholds/evidence: `docs/EVIDENCE-THERMAL.md`.
- Reference-repo audit (2026-11-23): new `doctor --fix-thermal` removes
  foreign ships in `/data/vendor/thermal/config` (other tools' files shadow
  the vendor config at `sconfig=0`) — CLI-only, daemon must be stopped.
  Evidence: `docs/HISTORY.md`.
- Thread placement is implemented (0.9.0) as an **opt-in, default-off**
  bridge: Game-mapped apps only, `UnityMain`/`GameThread` → cpu6–7,
  `UnityGfx*`/`RenderThread`/`RHIThread` → cpu6, captured affinity in
  `affinity.json`, reconciled at startup. It is not per-app configurable yet
  and must stay within the active cpuset (never widen a mask).
