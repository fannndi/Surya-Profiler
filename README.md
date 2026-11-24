# Surya Profiler

Performance profiler/tuner for the **POCO X3 NFC** (`surya` / `sm6150` /
`SDMMAGPIE`) on **MIUI 14 / Android 12** with root (APatch).

Derived from the architecture and safety rules of MiFineTune (the MIUI 12 /
Android 10 reference, not bundled in this repository), with every device
assumption re-audited on:

- ROM `V14.0.2.0.SJGIDXM` (MIUI 14, Android 12, SDK 31)
- kernel `4.14.190-perf-g75201eeb4293`
- APatch root

## Status

- Rust engine + daemon: **198 unit tests + 17 daemon E2E pass**, clippy/fmt clean.
- Android app: **24 JVM tests**, builds (`app-debug.apk`) and is device-tested.
- Device E2E: **passed 2026-11-24** for v0.9.0 (thread placement on/off,
  restore on leave and daemon exit; thermal corrupt-config self-repair;
  doctor UFS health) plus the v0.8.0 floor-audit/logging run and the earlier
  v0.7.0–0.4.2 suites. Evidence: [docs/HISTORY.md](docs/HISTORY.md).

## Features

| Surface | Behaviour |
|---|---|
| Device Profile | 3 cards: Power Save, Balance, Game. Tap = apply + become the universal base. |
| Sleep | Hidden screen-off profile after a 10 s grace period. |
| Apps Profile | Map an app to Default / Power Save / Balance / Game while it is in front. |
| Dynamic Profile | ON = a mapped app overrides the base while it is in front; any unmapped app (including the launcher and Surya Profiler) returns to the base. OFF = base only. |
| Thread placement | Opt-in (default OFF): while a Game-mapped app is in front, pins the Unity/Unreal main+render threads to the gold cores; restores the captured affinity on leave, screen off, lock and Service OFF. |
| Multi-window | Split screen is forced to Balance (dumpsys window probe). |
| MIUI saver follow | A Power Save-mapped app turns MIUI Battery Saver on; leaving restores the user value. |
| Service | FGS + QS tile + boot receiver. ON starts the daemon; OFF restores stock. |
| Automation | `SET_PROFILE` only (Tasker/MacroDroid/adb). |
| R&D | One global `input_suspend` switch in Settings → Advanced; auto-released on Service OFF. |
| Custom thermal | Active Device Profile selects a custom encrypted `mi_thermald` config (balance/game/powersave/sleep). The map is generated from the vendor map plus private `sconfig` 60–63; stock MIUI is the fallback. |
| MIUI thermal awareness | Decodes PowerKeeper scenario IDs/offsets, shows MIUI performance mode, and yields to camera/phone/ARVR/performance while keeping custom thermal for video/game/benchmark. |
| Diagnostics | Read-only daemon/env/thermal/history/log, MIUI `sconfig`, scenario, caps, cooling states, takeover state and the newest `mi_thermald` throttle events. |
| Background | Settings shows a live battery-optimization exemption check; `doctor` adds MIUI whitelist/appops/millet checks. |

Removed on purpose: per-app DND, refresh-rate follow, charge limit,
bypass-charging policy, battery/thermal guards, quiet hours, backup/import,
boost/jank overlay, storage maintenance, and the game-mode checker. The
project is intentionally small. The scheduler/WALT track is R&D only
([docs/EVIDENCE-DEVICE.md](docs/EVIDENCE-DEVICE.md)).

## Architecture

```
Compose UI ──► HomeViewModel ─────────────────┐
   │             │ (apply/restore while off)  │ CLI: surya-prof apply ...
   │             ▼                            ▼
   │      DynamicProfileService ──► DaemonClient ──► su ──► surya-prof serve
   │        (FGS + notification)      │  JSON-lines on stdio   (root daemon)
   │                                  │                          │
   └── DynamicProfileState ◄── events ┘                    ┌─────┴─────┐
       (StateFlows for the UI)                             │ arbiter   │
                                                           │ worker    │
   config.json (app-owned, atomic) ── read ──────────────► │ watchers  │
                                                           │ bridge    │
                                                           │ engine    │
                                                           └───────────┘
```

- The Rust daemon owns every decision and every write.
- The Kotlin app is a thin client: Compose UI, FGS lifecycle, `su` plumbing,
  Android-only signals (screen/keyguard).
- `config.json` is app-owned and written atomically; the daemon reads it at
  start, on `config_changed`, and on mtime changes.
- State lives in `/data/adb/suryaprofiler/`; the binary is deployed to
  `/data/local/tmp/suryaprofiler/surya-prof`.

### Config schema 2

```json
{
  "schema": 2,
  "enabled": false,
  "dynamic": true,
  "custom_thermal": true,
  "base_profile": "balance",
  "app_map": {
    "com.android.chrome": "powersave"
  },
  "rnd_input_suspend": false,
  "thread_affinity": false
}
```

`custom_thermal` defaults to `true` (automatic per Device Profile). Schema 1
files are normalized on first app load: `app_profiles[*].profile` is folded
into `app_map`, and every removed key is dropped.

## MIUI 14 constraints that shaped the design

1. **The screen-on CPU floor is MIUI's and interaction-scoped**:
   `msm_performance.cpu_min_freq` is typically `1612800` little / `1843200`
   big during a wake/touch boost, but that boost lasts only ~1–3 s and
   adb-driven sessions can run floor-less. Writing `scaling_min_freq` below
   the live value is clamped; a cap below it is planned `Locked` and retried
   once after the boost decays. The engine always reads the live floor.
2. **Foreground events are `wm_*`**, not `am_*`:
   `wm_resume_activity`, `wm_set_resumed_activity`. A cold/cleared event
   buffer falls back to `dumpsys activity activities`. Surya Profiler is a
   normal foreground app (never offered in Apps Profile): opening it from a
   mapped app returns to the universal base. Only real overlays (SystemUI,
   IME, permission dialogs) are transient.
3. **Split screen is not in `ProcessMonitor`** on this build; a
   `dumpsys window` probe reads `visible=true mode=split-screen-*` plus the
   task affinity (`A=<uid>:<pkg>`).
4. **MIUI Battery Saver** is `Settings.Global low_power`; the bridge holds the
   user's value and restores it on release.
5. **`input_suspend` is the only charge node the app may write**, and only
   through the R&D switch. `battery_charging_enabled` stays framework-owned.
6. **Thermal takeover is opt-in by profile**: the daemon installs four
   encrypted configs under `/data/vendor/thermal/config` with private
   `sconfig` indices **60–63** and follows the active Device Profile. The map
   is generated from `/vendor/etc/thermal-map.conf`, so stock entries stay
   across MIUI updates. `mi_thermald` rejects map IDs >64, so 60–63 is the
   highest safe private range. PowerKeeper IDs 20–25 (+30/+100/+200/+500 offsets)
   are decoded and never confused with ours. MIUI camera/phone/ARVR/performance
   scenarios are yielded to; video/game/benchmark keep the custom profile.
   Service OFF removes the custom files and restores the captured `sconfig`.
   The configs keep the stock battery charging ladder; the app never touches
   charge nodes.

Architecture: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), protocol: [docs/PROTOCOL.md](docs/PROTOCOL.md), thermal: [docs/THERMAL.md](docs/THERMAL.md).

Full evidence: [docs/EVIDENCE-DEVICE.md](docs/EVIDENCE-DEVICE.md),
[docs/EVIDENCE-THERMAL.md](docs/EVIDENCE-THERMAL.md),
[docs/ROM-HARMONY.md](docs/ROM-HARMONY.md), [docs/HISTORY.md](docs/HISTORY.md).

## Build

```bash
./scripts/check.sh     # fmt, clippy, Rust tests, Kotlin tests, APK
./scripts/build.sh     # cross-build arm64 + APK
./scripts/deploy.sh    # adb install + device health summary
```

Prerequisites: Rust stable, `cargo-ndk`, Android SDK + NDK
(`30.0.16248370` by default), JDK 17, and `adb` for deploy. `syncCore` copies
`core/profiles.json` and the release `surya-prof` binary into
`app/src/main/assets/`; `check-assets.sh` fails if the bundled binary/profile
pack is stale or the versions disagree.

Architecture: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) · protocol:
[docs/PROTOCOL.md](docs/PROTOCOL.md) · thermal:
[docs/THERMAL.md](docs/THERMAL.md) · device matrix:
[docs/DEVICE-E2E.md](docs/DEVICE-E2E.md).

## CLI

| Command | Purpose |
|---|---|
| `serve` | stdio daemon (JSON-lines; app-driven lifecycle) |
| `plan <id>` | dry-run: what would be written, per-key status |
| `apply <id>` | snapshot → write → read-back verify → set active (refused while the daemon runs) |
| `verify <id>` | drift detection (read-only) |
| `restore` | release holds + write the stock snapshot back (explicit repair / Service OFF) |
| `clean` | release saver/input-suspend/thermal holds without a daemon |
| `thermal-clean` | remove an orphaned custom thermal takeover (root) |
| `status` | active profile, snapshot, catalog, device info |
| `doctor` | environment self-check (incl. BatteryService/fuel-gauge consistency) |
| `probe` | read every catalog node (JSON) |
| `profiles` / `catalog` | bundled profiles / full parameter catalog |

## Safety

- Every profile write goes through `catalog::guard_path` (catalog membership
  + forbidden-prefix check); thermal takeover has its own explicit allowlist.
  Framework paths (`msm_performance`, perf HAL, charge internals, SELinux)
  are never profile targets.
- Read-back verification is a hard gate for real drift. A stricter framework
  clamp (thermal cap / QoS floor) is recorded as framework-held, not silently
  rewritten; a lost cap/floor is a failure.
- `stock.json` is a persistent union-stock baseline; forced applies reconcile
  every union key.
- **Service OFF = true stock restore**: release holds (MIUI saver + R&D input
  suspend), remove the custom thermal map/configs, restore `sconfig`, write
  `stock.json` back, verify, then stop. A failed/unconfirmed restore keeps the
  service ON. No reboot is needed.
- The daemon also restores stock on a clean exit when a profile is still
  active (app/service killed).
- No SELinux changes, no `persist.sys.aries.*` props, no vendor dfps props.

## Attribution

Architecture and several implementation patterns are derived from
MiFineTune (reference architecture; local only, not bundled) (Apache-2.0). See `NOTICE`.
