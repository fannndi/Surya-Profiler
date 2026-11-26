# R&D history — device E2E & RE sessions

Kronologi bukti E2E device dan sesi reverse-engineering (isi penuh,
struktur digabung 2026-11-24).


---

## DEVICE-E2E-2026-11-19.md

> **Historical document.** Sections before “Slim cleanup — 2026-11-20”
> describe the 0.1 feature set (DND, refresh, charge limit, boost, etc.),
> which was removed in 0.2. The slim section at the bottom is the current
> behaviour and evidence.

## Device E2E — 2026-11-19/20

Device: POCO X3 NFC, MIUI 14 V14.0.2.0.SJGIDXM, Android 12, APatch/FolkPatch.
Build: Surya Profiler `0.1.0` debug APK, Rust `surya-prof` release arm64.

### Setup

- APK installed as `com.suryaprofiler`.
- Root granted to the app via FolkPatch/APatch Superuser.
- App deployed `surya-prof` to `/data/local/tmp/suryaprofiler/`.
- State dir created at `/data/adb/suryaprofiler/` (profiles, snapshot, stock,
  state, stats).
- FGS `DynamicProfileService` started, daemon PID live, JSON-lines connected.

### Profile apply

| Action | Result |
|---|---|
| First launch (base balance) | `apply balance: done in 142ms`; state `active=balance` |
| Tap Power Save | `apply powersave: done in 102ms`; p0 governor `powersave`, p0 max 1612800, p6 max 1843200, GPU max 5, core_ctl 2/4 |
| Tap Game | `apply game: done in 91ms`; p0 max 1804800, p6 max 2304000, GPU 0/3, core_ctl 6/6, stune boost 10, deadline |
| Screen off + 10s | `apply sleep`; p0 min/max 576000/1017600, p6 652800/1324800, core_ctl 1/2, GPU max 5 |
| Wake + unlock | base profile re-applied (refresh restored 90 Hz) |

All plans on-device: `powersave ok`, `balance ok`, `game ok`, `boost ok`
(no locked keys); `sleep` plans clean when screen is off.

### Bridge / Apps Profile

- Added `com.android.chrome -> powersave` via config (R&D edit).
- Opening Chrome: `evaluate(event): -> powersave (app)`, `MIUI saver ON`.
- Leaving to launcher: `evaluate(event): -> game (base)`, saver restored.
- Per-app refresh 60 Hz: `refresh follow 60 Hz (com.android.chrome)`;
  `dumpsys display` active mode 3 (60 Hz), `user_refresh_rate=60`.
- Leaving: `refresh restored (90)`; active mode 2 (90 Hz), setting 90.

### Service OFF

- Tapping Service off (confirm) sent `restore`:
  - `bridge: MIUI saver restored`
  - `bridge: refresh restored (90)`
  - `restore: ok=true failed=0`
  - daemon `bye` and exit
- `state.json`: `{"active":null,"last_mode":"hands-off"}`
- FGS stopped; no daemon process.
- **Hands-off means no node writes.** The UI wording was corrected to say so
  (it previously claimed "written back to stock").

### Issues found during E2E

1. **Saver attribution race**: after leaving a powersave-mapped app, the daemon
   briefly (one 15s periodic tick) applied `powersave` as the base because the
   `LiveFlags` cache still had our own `low_power=1` for up to 5s. Fixed with a
   6s release-suppression window in `Bridge::user_saver`.
2. **First launch before root grant**: deploy returned `su: error=13` and the
   ViewModel stopped at the error; granting root later did not recover until a
   relaunch. `ON_RESUME` now runs the full `refresh()` (deploy + service start).
3. **UI report contradiction**: daemon-path reports have no per-op `results`,
   so the dialog printed "Everything already in sync" even when `wrote=36`.
   Fixed: that text now requires `wrote == 0 && failed == 0`.
4. **Wrong UI wording**: Service row said "Off · stock" and the confirm dialog
   claimed stock restore. Both corrected to hands-off.
5. **Stale config during manual R&D edits**: the app keeps its config in
   memory; a root-side edit is only visible to the daemon and is overwritten
   on the next app write. For normal use the app is the single writer, so this
   is expected; R&D config edits were reverted to a clean config afterwards.

### Final state after E2E

- App installed, clean config (`enabled=false`, base `balance`, no app map).
- Service off; daemon exited; `last_mode=hands-off`.
- Tuning values were left as last applied (hands-off by design).

---

## Follow-up — 2026-11-20

Second device pass after the cleanup build (147 unit tests, new daemon binary
`802448` bytes). The device stayed on the same ROM/build.

### Startup fallback (new)

- Cleared `logcat -b events`, then started the Service from the app with the
  events buffer empty.
- Daemon log: `seed: peeked=Some("com.suryaprofiler") best=None` followed by
  `evaluate(unlock): -> balance (base)` — `dumpsys activity activities` supplied
  the current package even though the `wm_*` buffer was empty, and the
  startup fallback applied the base instead of idling.
- Without this, the old daemon would have stayed `active=null` until the next
  app switch (observed once during this session).

### Per-app DND (new)

- Granted policy access with `adb shell cmd notification allow_dnd
  com.suryaprofiler`; daemon log `DND access: true`.
- Set Chrome → Do Not Disturb = Total silence via Apps Profile UI, saved.
- Opening Chrome: `bridge: DND total (com.android.chrome)`, app log
  `dnd applied: total`, `settings get global zen_mode` = 2
  (`ZEN_MODE_NO_INTERRUPTIONS`, enabler `com.suryaprofiler`).
- Leaving Chrome: `bridge: DND released`, `dnd restored: 1`, zen mode returned
  to the captured user value.
- Reset the profile afterwards; policy access revoked.

### Per-app refresh via UI (re-test)

- Set Chrome → Refresh rate = 60 Hz through the Apps Profile dialog (not a
  config edit), saved.
- Opening Chrome: `bridge: refresh follow 60 Hz`; `mActiveModeId=3`,
  `settings get system user_refresh_rate` = 60.
- Leaving Chrome: `bridge: refresh restored (90)`; `mActiveModeId=2`,
  `user_refresh_rate` = 90.

### Charge limit

- Service ON, battery 100%, limit 80%: switch ON →
  `battery_charging_enabled=0`, `holds.charge_held=true`, log
  `charge paused at 100% (limit 80)`.
- Switch OFF → `battery_charging_enabled=1`, hold cleared, log
  `charge resumed (node 1)`.
- Repeated through the automation API (`SET_CHARGE_LIMIT`) with the same
  result.

### Automation API

- `SET_PROFILE powersave` → config base `powersave`, daemon
  `set_base: powersave`, profile applied.
- `BOOST` → `apply boost` for the 5 s window, then
  `boost window over` / back to base.
- `SET_CHARGE_LIMIT --ez enabled true --ei pct 80` → pause/resume as above.
- `SET_PROFILE balance` restored the base afterwards.

### Tile / notification stop path

- With charge limit holding, started the service with
  `com.suryaprofiler.action.STOP` (root, because the service is not exported;
  the tile/notification PendingIntent has the app identity).
- Result: daemon `ipc: restore`, `bridge: charge resumed (node 1)`,
  `restore: ok=true`, `holds` all cleared, daemon exited, app
  `config.enabled=false`. Tuning values remain applied (hands-off).

### UI report wording

- Service OFF from Home now shows **“Service off complete”** with
  “Bridge holds were released. Profile tuning values were left as-is
  (hands-off)…”, not “Restore complete”.
- If the daemon release fails, the app keeps the service enabled and shows an
  error instead of stopping into a half-released state.

### Final state

- Service off (`enabled=false`), base `balance`, no app map, no per-app
  profiles, charge limit off.
- `surya-prof status`: `active=null`, `last_mode=hands-off`.
- `battery_charging_enabled=1`.
- Tuning nodes left as last applied (hands-off by design).

### Multi-window fallback (new)

- Created a real two-pane split with
  `am stack move-task 211 6 true` (Chrome, primary) and
  `am stack move-task 215 7 true` (Settings, secondary); screenshot confirmed
  Chrome on top and Settings at the bottom.
- `ProcessMonitor` emitted **no** multi-window line for this split.
- The new daemon probe (`dumpsys window`, on foreground change and every
  5 supervisor ticks = 15 s) detected both visible tasks:
  `bridge: multi-window ON (com.android.settings)` →
  `evaluate(multiwindow): -> balance (multi-window)`.
- Pressing Home collapsed the split:
  `bridge: multi-window off` → `evaluate(multiwindow): -> game (base)`.
- The synthetic move is the scriptable equivalent of the Recents split
  gesture; re-check with a manual gesture after MIUI updates.

---

## Slim cleanup — 2026-11-20

Second-generation cleanup after the product decision: only **Device
Profile** and **Apps Profile** remain; `boost` is gone, and Service OFF now
restores stock instead of leaving the last tuning values in place.

### Scope

- Kept: 3 Device Profiles + hidden Sleep, Apps Profile mapping, Dynamic
  Profile, multi-window → Balance, MIUI Battery Saver follow, automation
  `SET_PROFILE`, one R&D `input_suspend` switch, diagnostics.
- Removed: DND, refresh-rate follow + SF 1035 + dex helper, charge limit /
  charge-once, bypass policy + floor, battery/thermal guards, quiet hours,
  backup/import, jank boost, storage maintenance, MIUI performance mirror,
  game-mode checker, ultra-saver retire, app-side drift guard, biometric
  gate, usage-stats foreground fallback.
- Config schema 1 → 2. `app_profiles[*].profile` folded into `app_map`; old
  keys dropped. `app/src/main/assets/profiles.json` has 4 profiles.

### Device evidence

- Config after first launch:
  `{"schema":2,"enabled":false,"dynamic":true,"base_profile":"balance",
  "app_map":{},"rnd_input_suspend":false}` — no schema-1 keys.
- Service ON applied `balance`; tapping Game applied `game` and persisted
  `base_profile:"game"`.
- Screen off → `evaluate(sleep): -> sleep (screen off)` after the 10 s
  grace; screen on/unlock → `game (base)`.
- Apps Profile: Chrome → Power Save via the bottom sheet; opening Chrome
  logged `evaluate(event): -> powersave (app)` and
  `bridge: MIUI saver ON (com.android.chrome)` (`low_power=1`). HOME
  restored `low_power=0` and `game (base)`.
- Multi-window: Chrome + Settings split (`am stack move-task`) →
  `bridge: multi-window ON (com.android.settings)` →
  `balance (multi-window)`. With a Power Save-mapped Chrome in the split,
  `low_power` stayed `0` (the saver gate now excludes split screen).
  Collapsing the split → `multi-window off` → `game (base)`.
- Automation: `SET_PROFILE balance` → `automation: set profile balance`,
  `set_base: balance`, applied; `BOOST` is no longer registered.
- R&D input suspend: Settings switch ON → `bridge: R&D input suspend ON`,
  `holds.json` `bypass_held:true`; Service OFF → `R&D input suspend OFF
  (node 0)`, `bypass_held:false`, config `rnd_input_suspend:false`.
- Service OFF (Home and tile/notification):
  - daemon log: `ipc: restore (service off)`, `MIUI saver restored`
    (when held), `R&D input suspend OFF`, `restore: ok=true wrote=3/4
    verified=… failed=0`, then stdin close / bye.
  - `state.json`: `active:null`, `last_mode:"restore"`.
  - `holds.json`: all false/null; `low_power=0`; `battery_charging_enabled=1`;
    daemon process gone. **No reboot.**
  - A following `surya-prof restore` reports the same 3 representation-
    normalized keys (`io.scheduler`, `core_ctl.busy_up_thres`,
    `core_ctl.busy_down_thres`); they are verified and not left drifted.
- Final device state: service OFF, base `balance`, no app map, no R&D
  suspend, no daemon, `low_power=0`, `zen_mode=0`, `user_refresh_rate=90`,
  `battery_charging_enabled=1`.

### Host tests

- 114 unit tests + 8 host E2E pass; `cargo fmt --check` and
  `cargo clippy --all-targets -- -D warnings` clean.
- APK debug builds; device deploy binary is 727280 bytes.

---

## Thermal telemetry — 2026-11-20 (v0.3.0)

- Diagnostics → **Thermal (MIUI)** card verified on device:
  `normal · sconfig 0`, `temp 0 · screen 1`, quiet/NVM `44.8 / 45.9 °C`,
  CPU6/CPU7 cooling `0 / 14`, caps `L 1.71 GHz · B 1.94 GHz`.
- Home Diagnostics row shows `thermal normal` alongside battery/temp/GPU.
- `surya-prof doctor` adds:
  - `env_thermal_config` pass (`sconfig=0`, `mode=normal`, `temp_state=0`,
    `screen_state=1`)
  - `env_thermal_caps` pass (`little_mhz=1708`, `big_mhz=1939`,
    `cpu6_cooling=0`, `cpu7_cooling=0`)
- Read-only only: no `sconfig`/`temp_state`/thermal-config writes; Service
  state and tuning were untouched by the telemetry test.
- Host tests: 115 unit + 8 E2E; `cargo fmt --check` and clippy clean.
- APK v0.3.0 (versionCode 3); deployed binary 734944 bytes,
  md5 `6d28e2beb9c70dfc54a0db1ffe75701f`.

---

## Custom thermal takeover — 2026-11-20 (v0.4.0)

- Installed four encrypted configs + custom map in
  `/data/vendor/thermal/config` (marker `.suryaprofiler`); vendor files
  untouched. Private `sconfig`: balance 20, game 21, powersave 22, sleep 23.
- Profile switching verified:
  - balance → `sconfig=20`, `decrypt.txt` 1199 B (balance)
  - game → `sconfig=21`, 1199 B (game)
  - powersave → `sconfig=22`, 1189 B
  - sleep (screen off) → `sconfig=23`, 1053 B; screen on reasserted base
- Game under load (2 big-core burn):
  - at `nvm-therm` ≈ 56 °C, `policy6 max` stepped from 2 304 000 to
    2 169 600/1 939 200; `cooling_device8` state 2/3; quiet stayed ≈ 52–53 °C
  - sub-floor targets (`1.5552`/`1.2096`) were clamped by the 1.8432 GHz
    screen-on floor, so final configs use floor-aware steps and direct
    `nvm-therm-adc` (the MIUI `VIRTUAL-SENSOR` is rejected on this boot).
- Camera: MIUI wrote `sconfig=12`; daemon logged
  `thermal: yielding to MIUI scenario 12`. Back to Home → `sconfig=22`
  (active base) and decrypt returned to the custom powersave config.
- Settings switch `custom_thermal` off/on:
  - off → files removed, `sconfig=0`, decrypt 1016 B, service stayed ON
  - on → files reinstalled, profile re-selected.
- Service OFF:
  - `bridge: thermal: custom profiles released (stock map)`
  - files removed, `sconfig=0`, decrypt normal 1016 B, `holds.json` thermal
    fields null/false, daemon exited, `active:null`, `last_mode:"restore"`.
- Charging untouched: `battery_charging_enabled=1`, `input_suspend=0`,
  `low_power=0`; every custom config keeps the stock `MONITOR-BAT` ladder.
- `surya-prof thermal-clean` returns `{"ok":true}` and repairs an orphan.
- Host tests: 126 unit + 8 E2E; clippy/fmt clean.
- APK v0.4.1 (versionCode 5, md5 `47d8d9359802a5843457ce39be9e9bb8`);
  deployed binary 765416 bytes, md5
  `dd82b1ccd2ee4b38b31a0a0ca16c14af`.
- Final device state: service ON, base Game, `custom_thermal:true`,
  `sconfig=21`, custom files installed, `battery_charging_enabled=1`,
  `input_suspend=0`, `low_power=0`.

---

## BatteryService doctor check — 2026-11-20 (v0.4.1)

- Reproduced a stale status bar with `dumpsys battery set level 91`:
  `UPDATES STOPPED`, BatteryService level 91, sysfs `capacity=99`.
- `surya-prof doctor` → `battery_service`:
  - `pass:false`, `updates_stopped:true`, `service_pct:91`,
    `sysfs_capacity:99`, `match:false`
  - hint: `run: dumpsys battery reset`
- After `dumpsys battery reset`:
  - BatteryService level 99, status bar 99, sysfs 99
  - `battery_service` `pass:true`, `match:true`
- Informational only (not critical); no tuning/charging behavior changed.

---

## Universal base / Dynamic Profile fix — 2026-11-20 (v0.4.2)

Root cause: `com.suryaprofiler` was in `TRANSIENT_PACKAGES`, so returning to
the profiler kept `last_real` on YouTube and every `config_changed` from app
resume re-applied the YouTube mapping.

Fix:

- Surya Profiler is a normal foreground app (never mappable in Apps Profile).
- `decision_foreground()` prefers a real `last_fg`; only SystemUI/IME/dialog
  overlays fall back to `last_real`.
- `config_changed`, mtime reload and unlock seed use that helper.
- Foreground watcher drops logcat ring-buffer history older than its spawn.

Device evidence (base Balance):

| Transition | Active | Thermal `sconfig` |
|---|---|---:|
| YouTube | powersave | 22 |
| YouTube → Surya Profiler | balance | 20 |
| WhatsApp (unmapped) | balance | 20 |
| Home | balance | 20 |
| Epic Seven | game | 21 |
| Epic Seven → Surya Profiler | balance | 20 |
| Dynamic OFF → YouTube | balance | 20 |
| Service restart while Surya in front | balance | 20 |

- Returning from a Powersave-mapped app also restored MIUI saver
  (`low_power=0`).
- Daemon log after the fix:
  `seed: peeked=Some("com.suryaprofiler") best=Some("com.suryaprofiler")`,
  then `evaluate(...): -> balance (base)`.
- No replay of the old YouTube event on daemon restart.
- Charging untouched: `battery_charging_enabled=1`, `input_suspend=0`,
  `low_power=0`.
- Host tests: 130 unit + 9 E2E; clippy/fmt clean.
- APK v0.4.2 (versionCode 6, md5 `83cf37dfee09e8a8013d6201ae463c40`);
  deployed binary 765432 bytes, md5
  `d7c3e2eb57abb3cca9a326b1a4d5800f`.
- Final state: service ON, base Balance, active balance, `sconfig=20`,
  dynamic true, custom thermal true, `battery_charging_enabled=1`,
  `input_suspend=0`, `low_power=0`.

---

## MIUI thermal harmony — 2026-11-20 (v0.5.0, host + device)

Root cause found in the extracted ROM / PowerKeeper APK:

- `ThermalSwitchHandler` writes `sconfig` base IDs `0–25` plus offsets
  `+30/+100/+200/+500`.
- Its package map maps `com.google.android.youtube=21`,
  `com.android.camera=12`, `com.xiaomi.mihomemanager=22`, benchmarks `10`.
- The v0.4.x custom indices `20–23` overlapped MIUI game-normal/video/demo/
  yuanshen. YouTube could therefore be mistaken for the custom game profile
  (or mask a MIUI yield).
- `mi_thermald` keeps the last config for an unknown `sconfig`
  (`use config file which last set`), so Service OFF must force stock normal
  before removing the map.
- **`mi_thermald` rejects map IDs above 64** (disassembly + device log:
  `item: 60/61/62/63` parsed, `70–73` skipped). The first v0.5.0 preview used
  70–73 and loaded stock normal instead of custom; the final range is 60–63
  (outside MIUI's 0–25 and +30 offsets, below the parser limit).

Changes verified on device:

- Private indices **60–63**; legacy `20–23` migrated on the first sync.
  The device was upgraded from a 70–73 preview map: the daemon logged
  `keeping custom over MIUI unknown(70)` and rewrote `sconfig 60`.
- Map generated from `/vendor/etc/thermal-map.conf` (decrypt + preserve stock
  entries + 4K/camera fallbacks 17/18 + custom entries). The device map has
  18 entries and `decrypt.txt` matches the active custom config.
- Yield policy: camera 12/17/18, phone 8, ARVR 15, performance mode 1 /
  `thermal-global-mode` / `persist.sys.thermal.config`. Video 21, game 9/13/16,
  benchmark 10 and unknown values keep the custom profile.

Device evidence (base Balance, service ON):

| Transition | Active | `sconfig` | `decrypt.txt` | Log |
|---|---|---:|---|---|
| Surya / launcher | balance | 60 | `[BAL-SS-CPU6]` | `balance active (sconfig 60)` |
| YouTube | powersave | 62 | `[PS-SS-CPU6]` | `keeping custom over MIUI video (sconfig 21)` |
| YouTube → Surya | balance | 60 | `[BAL-SS-CPU6]` | MIUI saver restored |
| Camera | balance (yield) | 12 | stock camera | `yielding to MIUI camera` |
| Camera → Surya | balance | 60 | `[BAL-SS-CPU6]` | `balance active (sconfig 60)` |
| Epic Seven | game | 61 | `[GAME-SS-CPU6]` | `game active (sconfig 61)` |
| Epic Seven → Surya | balance | 60 | `[BAL-SS-CPU6]` | `balance active (sconfig 60)` |
| MIUI perf mode `persist.sys.thermal.config=1` | balance (yield) | 1 | stock normal | `yielding to MIUI performance mode` |
| perf off + `sconfig=0` | balance | 60 | `[BAL-SS-CPU6]` | `balance active (sconfig 60)` |
| daemon exit / service stopped | stock | 0 | stock normal | thermal released, restore 17/17 |

- `doctor` on device: `ok:true`; `thermal_takeover` pass (`decrypt_profile:
  balance`, `has_custom_indices:true`, `has_legacy_indices:false`,
  `map_source:data`, 18 entries); `miui_thermal_mode` reports
  `scenario:custom-balance`; `thermal_log` parses the MIUI dump.
- `mi_thermald` log confirms `item: 60/61/62/63 thermal config` and
  `change to config file /data/vendor/thermal/config/thermal-balance.conf`.
- Charging untouched: `battery_charging_enabled=1`, `low_power=0`; the R&D
  input-suspend hold was off (`holds.json bypass_held=false`).
- Host tests: **153 unit + 10 E2E**; clippy/fmt clean.
- APK v0.5.0 (versionCode 7, md5 `79070979da4ae9eb11432d3b4453ddf3`);
  binary 795664 bytes, md5 `efd42c39e4d91c371ed81051694f9514`.
- Final device state: service ON, base Balance, active `balance`, custom
  thermal `balance` (`sconfig 60`), `persist.sys.thermal.config=0`,
  `thermal-global-mode=0`, `low_power=0`, charging enabled.

Notes:

- The full Service OFF UI path (confirmation dialog → `restore` → stop) was
  validated in v0.4.2. v0.5.0 device testing used the daemon exit path
  (`am stopservice` as root), which runs the same `release_all` + restore
  sequence; it removed the marker/map and restored `sconfig=0`.
- A reboot test was not repeated in this pass; boot recovery was validated in
  v0.4.2. The current service is ON and will reinstall the map on the next
  start (`thermal_refresh_pending`).


### Follow-up note: video scenario testing

YouTube can enter picture-in-picture, so it is not a reliable "video app in
foreground" stimulus. The thermal policy itself is app-agnostic (`sconfig 21`
keeps the custom profile); device re-checks should use Mi Video/VLC or another
non-PIP player. YouTube remains a valid PowerKeeper mapping example in the
ROM evidence.

---

## Modularization + P0 hardening — 2026-11-20 (v0.5.0 tree, build after refactor)

Host: **156 unit + 10 daemon E2E + 1 watcher**, clippy/fmt clean,
`scripts/check.sh` green. Kotlin: 15 JVM unit tests (ProfileId/DaemonEvent/
ConfigCodec). APK md5 `fd2accbc97bd293fa9d959be2fbfac0e`; binary 815256 bytes,
md5 `db8222474f8ae02c54bc04732cbe73db`.

Device checks on the refactored build:

- Service OFF via the **UI switch + confirmation dialog** completed:
  `restore: ok=true wrote=17 verified=17 failed=0 holds=ReleaseReport {
  saver_ok: true, input_suspend_ok: true, thermal_ok: true }`.
  `config.json` ended `enabled:false, restore_pending:false`; `sconfig=0`;
  `/data/vendor/thermal/config/` empty; `holds.json` all false;
  `battery_charging_enabled=1`, `low_power=0`; daemon exited cleanly.
- Service ON again: daemon running, base Balance, `sconfig=60`,
  `decrypt.txt=[BAL-SS-CPU6]`, `doctor ok:true`.
- Sleep: screen off → `63` (`[SLP-SS-CPU6]`) after the 10 s grace; wake → `60`.
- Stale performance mode: set `persist.sys.thermal.config=1` → yields to
  `sconfig=1`; set it back to `0` **without touching sconfig** → daemon logs
  `keeping custom over MIUI performance (sconfig 1)` and reasserts `60`.
- YouTube (full-screen launch) → Power Save `62` (`[PS-SS-CPU6]`, MIUI video
  21 kept custom); return → `60`. Camera → yields to `12`; return → `60`.
- PIP caveat: YouTube can enter picture-in-picture; the checklist now prefers
  a non-PIP player (Mi Video/VLC/MX Player). MX Player's welcome screen is not
  mapped to MIUI video on this build (stayed Balance), so it is not a substitute
  stimulus by itself.

P0 behavior validated on host/device: per-surface hold gating, immediate
restore gate, `set_base` rejection while restoring/disabled, map-first thermal
cleanup, marker-less map adoption, `surya-prof clean` without a daemon, and
`applied` false when every write is rejected.


### Reboot + final protocol v3 (2026-11-21)

- Reboot with Service ON: boot receiver started the FGS and daemon, PID file
  present, `doctor ok:true`, sleep thermal `63` while locked, then base Game
  `61` (`[GAME-SS-CPU6]`) after wake. `battery_charging_enabled=1`,
  `low_power=0`.
- Protocol v3 verified in logcat (`daemon: hello v3`); `holds.json` now writes
  `input_suspend_held`/`input_suspend_saved` (old `bypass_*` files are read via
  serde aliases).
- Final build: APK md5 `fd2accbc97bd293fa9d959be2fbfac0e`; binary 815256
  bytes, md5 `db8222474f8ae02c54bc04732cbe73db`.


### Self-vibration diagnosis (2026-11-21)

Symptom: periodic vibration even with MIUI touch vibration off.

Evidence (`adb shell dumpsys vibrator_manager`):

```text
Previous vibrations for usage TOUCH:
  ... effect=Prebaked{effect=CLICK ...}, uid: 10161, opPkg: com.suryaprofiler
```

One entry appeared every ~15 s, exactly after `evaluate(periodic)` /
`apply game: in place`. Cause: `DaemonEventRouter` called `vibrateClick()` on
every `applied` event (including in-place periodic confirms); the newly added
`VIBRATE` permission made that legacy haptic active. `EFFECT_CLICK` with
`Usage=TOUCH` is not suppressed by MIUI's touch-vibration setting.

Fix: removed the haptic and the `VIBRATE` permission. After redeploy, the
vibrator history stayed at 13 entries for ~1m45s while the 15 s applies kept
running; no new `opPkg: com.suryaprofiler` entries appeared. Final APK
`fd2accbc97bd293fa9d959be2fbfac0e`; binary `db8222474f8ae02c54bc04732cbe73db`.


---

## RND-BATTERY-PERF-2026-11-21.md

## RND — Battery efficiency & performance tuning (2026-11-21)

Device: POCO X3 NFC (`surya`), MIUI 14 `V14.0.2.0.SJGIDXM`, kernel
`4.14.190-perf`, root APatch. Base commit: v0.5.0 tree.

Scope agreed with user:

- Scenarios: idle screen-on · Chrome scroll · 1080p video · Epic Seven ·
  screen-off sleep. 10 min per run.
- New catalog nodes: **SoC only** (CPU/GPU/RAM). Storage (f2fs) and net
  (`tcp_notsent_lowat`) skipped by decision.
- Final device state after all experiments: **Service OFF + full stock**.

### Method

- Harness: `tools/research-scripts/rnd_sampler.sh` (1 Hz read-only sampler; STEP=5
  for screen-off), `rnd_stim.sh` (scriptable stimulus: unlock/idle/scroll/
  video/game/off), `rnd_ab.sh` (one run: apply → stimulus → bypass charge →
  sample → gfxinfo → restore), `rnd_variants.py` (variant profile generator),
  `rnd_report.py` (metrics).
- **Bypass charging** (`battery/input_suspend=1`, the only allowed charge
  node) keeps adb on USB while the device discharges — honest drain without
  unplugging. Restored to `0` after every run (verified).
- Screen-off runs sample every 5 s to reduce wake pokes; drain is read from
  `charge_counter` (µAh monotonic) as the primary metric.
- Every tuning run uses the `surya-prof` CLI with `--profiles variant.json`
  (single writer; daemon off; no watchdog interference). Restore after every
  run; screen state verified per scenario.
- Cooling gate: quiet-therm < 35.0 °C before screen-on runs (auto in
  `rnd_ab.sh`).

### Phase 1 — new-node audit (read-only probe + write/readback/restore)

Ownership sources checked: `init.qcom.post_boot.sh` (all arms),
`perf/commonresourceconfigs.xml`, `targetresourceconfigs.xml`, strings of
`vendor.qti.hardware.perf@2.2-service`, `libqti-perfd.so`,
`libqti-perfd-client.so`.

| Node | Stock | Written by | Write test | Verdict |
|---|---|---|---|---|
| `kgsl-3d0/ifpc` | 1 | none found | ok, restored | Free; stock=1 is the efficiency setting (inter-frame power collapse). Only A/B value: 0 for max perf. |
| `kgsl-3d0/bus_split` | 1 | none found | ok, restored | Free; stock=1 enables split BW voting. No better direction for battery. |
| `kgsl-3d0/gpu_llc_slice_enable` | 1 | none found | ok, restored | Free; stock=1 = GPU uses system cache (perf+efficiency win). Do not disable. |
| `kgsl-3d0/gpuhtw_llc_slice_enable` | 1 | none found | ok, restored | Free; same as above (HTW path). |
| `kgsl-3d0/preempt_level` | 0x1 | none found | ok, restored | Free (0x0 tested). Perf-oriented; no battery win expected. |
| `/dev/stune/background/schedtune.boost` | 0 | none found | ok, restored | Free candidate for background placement. |
| `/dev/stune/background/schedtune.prefer_idle` | 0 | none found | ok, restored | Free candidate (idle-CPU bias for background). |
| `/dev/stune/system-background/schedtune.*` | — | — | absent | no-go (no schedtune group files) |
| `/dev/stune/schedtune.colocate` | 0 | — | **read-only (0444)** | no-go |
| `/dev/stune/schedtune.sched_boost_no_override` | 0 | none found | ok, restored | Free; tied to forbidden `sched_boost` semantics — keep untouched unless evidence. |
| `cpu0/core_ctl/enable` | 1 | post_boot `disable_core_ctl()` (other arms; moorea arm leaves it 1) | ok, restored | Baseline-family; lever: 0 = park nothing (all little cores online, no core_ctl latency). |
| `devfreq/soc:qcom,cpu{0,6}-cpu-l3-lat/max_freq` | 1459200000 | post_boot writes min+max for sibling SoCs (lito/arm…), not the moorea arm | ok, restored | New catalog candidate (Baseline family), bounds = `available_frequencies` list (300M..1459.2M). |
| WALT nodes (`sched_window_stats_policy`, `sched_ravg_hist_size`, `sched_restrict_tasks_spread`, `sched_small_task`, per-CPU `sched_mostly_idle_*`/`sched_prefer_idle`) | — | — | **absent on this kernel** | track closed (only `sched_load_boost` exists — perf-HAL/ROM owned, stays forbidden) |
| cpuidle | `current_governor_ro` only | — | read-only | no-go (already guarded) |
| f2fs / `tcp_notsent_lowat` | — | — | — | skipped (user decision: non-SoC) |

GPU A/B note: all five kgsl knobs are stock-optimal for efficiency; the only
plausible perf lever is `ifpc=0` (and it trades power). Confirm in the game
scenario; otherwise document as stock-optimal (no profile writes).

### Phase 2 — A/B runs

Metric columns: `mW` = mean electrical power; `mAh` = integrated current;
`ccmAh/h` = charge_counter drain rate; `jank%` / `p95` = `dumpsys gfxinfo`
(browser-UI process only — Chrome renders page scroll in its renderer
process, so gfxinfo jank is NOT usable for scroll smoothness on this build;
power/freq/gpu-busy are the scroll metrics).

#### IMPORTANT FINDING — the "screen-on floor" is not a screen state

Re-verified on 2026-11-21 with a clean reboot (stock, service off, holds
empty):

- After wake + swipe, `msm_performance.cpu_min_freq` shows a **~3 s wake
  boost** (`0:1804800 6:2304000` — cpu_boost powerkey-style values), then
  returns to `0` for the rest of screen-on.
- A real (injected) swipe produces a **~1 s touch blip**
  (`0:1248000 6:1094400`), never the full `1612800/1843200` floor.
- `kernel.sched_boost` stays `0` during screen-on today (yesterday's audit
  recorded `1` during screen-on).
- PowerKeeper dump during screen-on/unlocked: `mIsBacklightOn = false`,
  `mIsTpOn = false` — its own screen model mismatched the panel (the
  backlight node read 140/4095 ≈ 3 % during the first attempts: MIUI maps
  `screen_brightness` straight onto the 0..4095 backlight scale, not 0..255).
- 3 minutes of normal use + fresh reboot still did not reproduce the
  sustained floor via adb-driven interaction.

Consequences:

1. The RUNTIME-BEHAVIOR / DEVICE-FACTS claim "screen-on floor is constant
   while screen is on" must be downgraded to "floor engages during specific
   MIUI-detected active-use states; adb-driven sessions run **floor-less**".
2. Every run must record its regime: the sampler now stores
   `msm_performance.cpu_min_freq` and `sched_boost` per sample
   (`floor` = 1612800/1843200 present, `boost` = blips, `none`).
3. The floor-less regime is itself a **real usage regime** (reading, video,
   any no-touch screen-on period) — A/B results in this regime are valid for
   those scenarios.
4. Profile design implication: caps/hispeed values must be sane in BOTH
   regimes, because MIUI decides the regime, not the app.
5. The automated screen-on A/Bs below were run floor-less (`regime=none`),
   brightness locked at 400/4095, refresh 60 Hz.

#### Sleep (screen off, 10 min each, STEP=5)

Primary metric: integrated current (mAh) → mean power. `charge_counter` is
quantized (~51 mAh steps at high SoC) — dropped as a metric.

| Run | Config | mW | mAh | p0 mean/max MHz | p6 mean/max | p0 active |
|---|---|---|---|---|---|---|
| SL0-stock | true stock | 574.0 | 21.7 | 1482/1805 | 848/2304 | 6 |
| SL1-ship | shipped sleep | 531.2 | 20.0 | 592/1018 | 766/1325 | 2 |
| SL2-capL | caps 768/1094 | 502.1 | 18.9 | 657/768 | 654/1094 | 2 |
| SL3-park | core_ctl 2/4 | 529.6 | 20.1 | 799/1018 | 690/1325 | 4 |
| SL4-vm | stock writeback | 454.5 | 17.1 | 788/1018 | 716/1325 | 2 |
| SL5-stock | bookend | 455.2 | 17.2 | 1447/1805 | 908/2304 | 6 |
| SL6-stock2 | stock repeat | 523.5 | 19.7 | 1479/1805 | 914/2304 | 6 |
| SL7..SL9 | capL ×2 / stock ×1 repeat | TBD | TBD | | | |

Incident log (2026-11-21 ~01:55 → 10:49): the experiment server restarted
mid-run SL7 with `input_suspend=1` still set; the device discharged on
battery ~9 h (97 % → 58 %) and the Surya Profiler app auto-restarted the
service (base balance + thermal takeover). Recovery: udev rule for the USB
node (Xiaomi vendor `2717`) installed via `pkexec`, then `input_suspend=0`,
`am stopservice` (holds cleared), `am force-stop` app, `sconfig=0` verified,
device back to stock. Restarts must always run `rnd_ab.sh` with the restore
step, and a watchdog for `input_suspend` would have caught this earlier.

First-pass reading:

- Stock-vs-stock run variance (574 vs 455 mW, ±11%) is larger than any
  profile-vs-stock delta — background job bursts dominate the window.
- Caps change the frequency distribution massively (p0 mean 1482 → 592)
  while power moves only slightly — screen-off energy is wake-burst shaped,
  and bursts at capped freqs are roughly as efficient per unit work.
- SL4-vm (stock writeback) ≈ SL5-stock exactly → VM writeback lever shows
  no signal in sleep.
- Decision pending the SL6–SL9 alternating repeats.

#### Balance — screen-on scroll (Chrome, floor-less regime)

| Run | Config | mW | mAh | p0 mean/max | p6 mean/max | p0 active | regime |
|---|---|---|---|---|---|---|---|
| BL0-stock (10 m) | stock | 1080.1 | 43.1 | 1602/1805 | 978/2304 | 6 | none |
| BL1-ship (10 m) | balance | 1007.6 | 39.8 | 1499/1709 | 745/1939 | 6 | none |
| BL2-stock (6 m) | stock | 1085.2 | 25.2 | 1601/1805 | 754/2304 | 4 | none |
| BL3-ship (6 m) | balance | 1153.8 | 26.8 | 1519/1709 | 746/1939 | 4.1 | none |
| BI1-stock-idle (6 m) | stock, home | 1016.8 | 23.5 | 1614/1805 | 796/2304 | 4.2 | none |

Reading:

- Stock runs: 1080 / 1085 mW (scroll), 1016.8 (idle). Balance runs: 1007.6 /
  1153.8 mW (scroll). The pair order flips the sign — **between-run
  workload noise dominates**; stock mean ≈ balance mean ≈ 1082 mW.
- CPU sits near the MIUI floor (p0 ≈ 1.5–1.6 GHz) in every screen-on run;
  the profile caps (little 1.7088/1.8048 GHz, big 1.9392 GHz) rarely bind at
  scroll-level workloads.
- core_ctl active_cpus floats 4↔6 with load even with the profile parked
  min=4 — parking depth is load-dependent, not a constant power lever.
- Conclusion so far: in the no-touch screen-on regime the CPU profile is
  power-neutral; the remaining real levers are the heavy-load regimes
  (video decode, game) and MIUI's own interaction policy.

#### Video — 1080p30 local playback via MX Player (HW decode, 6 min)

| Run | Config | mW | mAh | p0 mean | p6 mean | regime |
|---|---|---|---|---|---|---|
| BV0-stock | stock | 1863.2 | 43.7 | 1631 | 733 | none |
| BV1-ship | balance | 1831.2 | 42.9 | 1558 | 736 | none |
| BV2-ps | powersave | **1718.5** | 40.2 | **576 (pinned)** | 653 | none |

Reading:

- The video mapping to **Power Save is a real win: −7.7 % vs stock** in the
  no-touch regime. The `powersave` governor pins little at 576 MHz; hardware
  decode (VENUS) carries the playback, so the pinned CPU does not starve it
  (no software-decode signature in the CPU trace). This supports the
  YouTube→Power Save Apps-Profile mapping.
- Balance ≈ stock for video: the caps (1.7088/1.9392 GHz) do not bind at
  decode-level load.
- Caveat: with the pinned 576 MHz, touching UI controls inside the player
  (seek etc.) will feel slower until MIUI's interaction floor re-engages.

#### Game — Epic Seven lobby (GPU-bound, automated, 6 min)

| Run | Config | mW | mAh | p0 mean | p6 mean | p0 act | gpu busy | regime |
|---|---|---|---|---|---|---|---|---|
| GM0-stock | stock | 2245.0 | 52.4 | 1717 | 843 | 4.1 | 76.1 % | none |
| GM1-ship | game | 2429.1 | 57.2 | 1695 | 1120 | 6.0 | 76.3 % | none |
| GM4-gpumin4 | game, GPU floor 355 | 2523.3 | 59.2 | 1678 | 1119 | 6.0 | 76.6 % | none |
| GM5-mig-stock | game, stock migration | 2442.4 | 57.6 | 1686 | 1121 | 6.0 | 76.5 % | none |

Reading:

- The shipped Game profile costs **≈ +8 % power in the lobby** with
  **identical GPU utilization** — the cost is CPU-side: big-cluster mean
  1120 MHz vs 843 (hispeed 2304@70 + `pl=1`), all little cores online
  (`min_cpus=6`), L3 floor 940.8 MHz. That is the responsiveness insurance
  doing its job at a GPU-bound load.
- The GPU-floor and migration levers produce no measurable change
  (`gpu busy` flat at ~76 %; deltas inside the ±100 mW run noise — the
  lobby's own animation state varies between runs).
- Gameplay FPS cannot be measured honestly by automation (gfxinfo does not
  cover GL renderers), so the Game profile keeps its shipped values: they
  are the deliberate perf-first choice for the interaction/game regime,
  and MIUI's thermal takeover already bounds the thermal side
  (custom steps 2169600/1939200/1843200).
- User decision 2026-11-21: skip the manual gameplay pair, codify with
  shipped Game values unchanged.

### Phase 2 conclusions (per profile)

| Profile | Measured verdict | Action |
|---|---|---|
| sleep | ≈ stock within ±11 % noise on 10-min windows (7 runs); deeper caps, parking depth and writeback levers show no signal | keep shipped values |
| balance | power-neutral in scroll (both 6- and 10-min pairs straddle stock mean); caps do not bind at scroll load | keep shipped values |
| powersave | **−7.7 % in the video scenario** (pinned 576 MHz, HW decode unaffected) — validates the video→Power Save mapping direction | keep shipped values |
| game | +8 % in GPU-bound lobby at equal GPU utilization; that margin is the intended responsiveness headroom; no lever shows a free win | keep shipped values |

The measurable battery/perf wins of the product therefore live in
**scenario mapping** (video apps → Power Save via Apps Profile) and in the
**thermal takeover**, not in CPU value changes for the no-touch regimes.
MIUI's interaction policy owns the touch-regime floor.

### Phase 3 — codify

Decisions (all evidence-backed, device-verified writes/reads where relevant):

1. **`core/profiles.json`: unchanged.** Every measured regime either matches
   stock within noise (sleep, balance-scroll) or confirms the shipped
   intent (video→Power Save −7.7 %; game keeps its perf headroom).
2. **Catalog: no additions.** All Phase-1 audited nodes are stock-optimal
   or unused by any winning profile, so per the slim-build rule
   (no entries without a profile value) they stay documented here instead:
   - `devfreq/soc:qcom,cpu{0,6}-cpu-l3-lat/max_freq` — writable, Baseline
     family (post_boot writes min+max for sibling SoCs), bounds =
     `available_frequencies` (300M..1459.2M). Live stock 1459.2M.
   - `kgsl-3d0/{ifpc,bus_split,gpu_llc_slice_enable,gpuhtw_llc_slice_enable,preempt_level}`
     — writable, no ROM/perf-HAL writer found; stock values (1/1/1/1/0x1)
     are already the efficiency-optimal settings. Not cataloged.
   - `/dev/stune/background/{boost,prefer_idle}` — writable, Free; no A/B
     win measured (background placement stays MIUI's business). Not cataloged.
   - `/dev/stune/schedtune.sched_boost_no_override` — writable but tied to
     the forbidden `sched_boost` semantics; leave untouched.
   - `cpu0/core_ctl/enable` — writable, Baseline family; parking-depth A/B
     showed no signal, so the shipped min/max_cpus values stay the surface.
   - WALT nodes (`sched_window_stats_policy`, `sched_ravg_hist_size`,
     `sched_restrict_tasks_spread`, `sched_small_task`, per-CPU
     `sched_mostly_idle_*`/`sched_prefer_idle`) — **absent on this kernel**;
     the v0.5.x scheduler R&D track is closed (only `sched_load_boost`
     exists and stays perf-HAL/ROM-owned = forbidden).
   - cpuidle: read-only only (`current_governor_ro`); the existing guard is
     correct.
   - f2fs + `tcp_notsent_lowat`: skipped per user decision (non-SoC).
3. **Harness**: `tools/research-scripts/rnd_{sampler,stim,ab,variants,report,gfxinfo}.sh|.py`
   kept for future passes. Known-good patterns: bypass-charging
   measurement (`input_suspend` capture/restore), regime labeling
   (`msm_min`+`sched_boost` columns), Awake-guarded screen toggles,
   unlock-after-cooldown ordering.
4. **Final device state**: Service OFF + full stock (user decision).
   Brightness/refresh settings restored to their user values.

Measurement caveats for future passes (documented in the findings above):

- `charge_counter` quantizes (~51 mAh) at high SoC — use current
  integration as the primary metric.
- gfxinfo cannot see Chrome's renderer or game GL surfaces — no jank
  metric for scroll/game without a vendor frame-capture path.
- Between-run workload noise on this device is ±5–11 %; paired
  alternation is mandatory, and 10-min windows are the practical minimum
  for sleep, 6 min for heavy-load screen-on runs.
- The MIUI screen-on floor is interaction-scoped (see the finding above) —
  any future screen-on A/B must log the regime per run.

### Final device state (2026-11-21, end of R&D)

- Service OFF, daemon gone, `state.json` `active=null / last_mode=restore`.
- `holds.json` all clear; `input_suspend=0`, `battery_charging_enabled=1`,
  `low_power=0`; `sconfig=0`, `temp_state=0`, `screen_state` follows panel;
  `/data/vendor/thermal/config/` empty; `decrypt.txt` = stock normal.
- Tuning nodes read back at stock: CPU (schedutil, 576000–1804800 /
  652800–2304000 screen-off), core_ctl 1/4/6 enable=1, GPU 0/6 + ifpc=1 +
  bus_split=1, stune background 0/0, L3 300M–1459.2M, io cfq, vm
  20/5/200/500 stat 1.
- User settings restored: `screen_brightness_mode=1`, brightness 110,
  `user_refresh_rate=60` (was already the user's value).
- `surya-prof status`: `active=null`, catalog 92/92, framework evidence
  stock (`sched_boost=0`, `msm_perf_locks` unlocked, `input_boost` stock).
- R&D raw artifacts archived locally, outside the repo (CSVs, gfxinfo captures,
  variant JSONs), device `/sdcard/surya_rnd/` (scroll.html, vtest.mp4) and
  `/data/local/tmp/surya_rnd/` (harness scripts).


---

## RND-SOC-2026-11-22.md

## RND — SoC parameter discovery & value optimization (2026-11-22)

Status: **in progress**. Device: POCO X3 NFC (`surya`), MIUI 14
`V14.0.2.0.SJGIDXM`, kernel 4.14.190, APatch root. Service OFF at start.

Scope per user decision: **SoC only** (CPU/GPU/RAM/bus DCVS/thermal). Storage
(f2fs/block) and network are out of scope for this pass. Benchmark windows are
**3–5 minutes** per run (user decision), using paired A/B/B/A to cancel linear
thermal drift.

### Method

- `tools/research-scripts/rnd_run.sh` — one run: prep 60 Hz panel + brightness 400,
  apply profile (CLI, daemon off), optional bus override, launch scenario,
  bypass charging (`input_suspend=1`), sample power at 1 Hz + SurfaceFlinger
  frame timestamps at 1 Hz, restore.
- `tools/research-scripts/rnd_pair.sh` — A/B/B/A sequence; `rnd_compare.py` aggregates.
- Frame metrics come from `dumpsys SurfaceFlinger --latency` on the real render
  layer (`ChromeChildSurface` for Chrome, `SurfaceView[...](BLAST)` for the
  game). This fixes the previous `gfxinfo` blind spot for GL/Chrome renderers.
- Power is trapezoidal integration of `current_now * voltage_now` (mW/mAh);
  `charge_counter` is ignored (quantized).
- 3-minute screening runs; 5-minute runs only for final decisions.

### F0 — write/readback/restore audit (2026-11-22)

All nodes below were written, read back, held for **62 s** under Chrome scroll,
and restored. They are writable from the APatch root context and not immediately
overwritten by the perf HAL:

| Node | Stock | Probe value | Held 62 s |
|---|---:|---:|---|
| `soc:qcom,cpu-cpu-llcc-bw/min_freq` | 4577 | 7110 | yes |
| `soc:qcom,cpu-llcc-ddr-bw/min_freq` | 762 | 1720 | yes |
| `soc:qcom,cpu0-cpu-ddr-latfloor/min_freq` | 762 | 1144 | yes |
| `soc:qcom,cpu0-cpu-llcc-lat/min_freq` | 4577 | 7110 | yes |
| `soc:qcom,gpubw/min_freq` | 0 | 1720 | yes |
| `soc:qcom,cpu0-cpu-l3-lat/max_freq` | 1459200000 | 1017600000 | yes |
| `/dev/cpuctl/top-app/cpu.shares` | 1024 | 2048 | yes |
| `/dev/cpuctl/background/cpu.shares` | 1024 | 256 | yes |
| `/proc/sys/kernel/sched_cstate_aware` | 1 | 0 | yes |
| `kgsl-3d0/popp` | 0 | 1 | yes |

The same probe found the following **not writable / not present**:
`kgsl-3d0/devfreq/*` (SELinux), `sched_util_clamp_*`/uclamp (absent),
WALT nodes (absent), BBR (absent), `max_sectors_kb` (= `max_hw_sectors_kb` 512),
UFS `clkscale_enable` (SELinux).

### Completed A/B results

All runs: Chrome scroll (`http://127.0.0.1:8000/scroll.html`, local server,
60 Hz, brightness 400, airplane not used) or Epic Seven **title screen**
(`Tap to Start`, GPU-bound, 76 % busy), 180 s per run, A/B/B/A.

| Pair | A | B | mW A → B | p95 A → B | fps | Verdict |
|---|---|---|---:|---:|---:|---|
| P1-BAL | stock | balance | 1519.7 → 1453.1 (**−4.4 %**) | 17.00 → 16.99 ms | 60/60 | **balance beats stock** |
| P2-GAME | stock | game | 2410.6 → 2473.2 (+2.6 %) | 16.72 → 16.70 ms | 60/60 | game costs power, no fps gain on title |
| P3-GMIN4 | game | game, `core_ctl.min_cpus=4` | 2553.8 → 2484.2 (**−2.7 %**) | 16.70 → 16.70 ms | 60/60 | small win |
| P4-GPL0 | game | game, `pl=0`, big hispeed 1.9392@85 | 2749.8 → 2587.0 (−5.9 %) | 16.71 → 16.70 ms | 60/60 | **invalid-ish**: A1 had a transient boost (2905 mW); A4 vs B is ~neutral |
| P5-GECO | game | game, min4 + pl0 + stock-ish sched | 2580.4 → 2521.9 (−2.3 %) | 16.70 → 16.70 ms | 60/60 | small/noisy win |
| P6-GPL0M4 | game | game, min4 + pl0 | 2568.7 → 2652.8 (+3.3 %) | 16.70 → 16.70 ms | 60/60 | B2 outlier (2797 mW, big 1554 MHz); B3 ≈ A |
| P7-GPL0M4 (300 s) | game | game, min4 + pl0 | 2494.9 → 2478.9 (−0.6 %) | 16.70 → 16.70 ms | 60/60 | neutral; CPU extras not the bottleneck |
| P8-GSCPU | game | game, stock CPU/sched/VM/cpuset, GPU full | 2469.7 → 2504.7 (+1.4 %, noise) | 16.70 → 16.72 ms | 60/60 | big cluster −26.7 %; stress-page p95 33→17 ms |
| P9-GSCPU | stock | game, stock CPU/sched/VM/cpuset | 2473.2 → 2427.9 (**−1.8 %**) | 16.72 → 16.72 ms | 60/60 | **Game now beats stock** |
| P10-BUS | balance | balance + bus medium | 1440.6 → 1437.2 (−0.2 %) | 17.00 → 16.98 ms | 60/60 | no measurable win |

Observation: the game title screen is GPU-bound and capped at 60 fps. CPU-side
profile changes cannot raise fps there; they only add or remove power. The old
Game profile spent +2.6 % power with no fps gain (big cluster 1124 vs 835 MHz)
and made a CPU/GPU canvas stress page worse (p95 33 ms vs 17 ms on stock;
jank 5.2 % vs 3.3 %). The stock-CPU Game keeps full GPU, game touch boost,
`top-app.prefer_idle` and the deadline I/O values; it is **−1.8 % power vs
stock** on the title screen with identical 60 fps, and it removes the stress-
page regression. Bus-DCVS floors were writable and held for 62 s, but produced
no measurable scroll win, so they are documented and **not cataloged**.

### Decisions

1. **Balance:** keep shipped values. Verified **−4.4 % power** vs stock in
   Chrome scroll at identical 60 fps / p95.
2. **Game:** replace the aggressive CPU profile with the measured stock-CPU
   variant (`game` in `core/profiles.json`): full CPU/GPU caps, stock
   sched/VM/cpuset, `cpu_boost.input_boost_freq=0:1804800`,
   `stune.top-app.prefer_idle=1`, deadline I/O. Measured −1.8 % vs stock.
3. **Thermal game:** raise SS-CPU6 56/60 → 58/62 °C, SS-CPU0 58 → 60 °C,
   GPU 58/62 → 60/64 °C, CCC 61 → 63 °C. `MONITOR-BAT`, `MONITOR-BCL`,
   `MONITOR-TEMP_STATE` and LCD stay; kernel LMH/BCL remains the hard limiter.
4. **Catalog:** no new nodes. Bus-DCVS/cgroup/sched/popp probes are documented
   above; only nodes with a measured profile win are added (slim-build rule).
5. **Power Save / Sleep:** unchanged; the previous 2026-11-21 pass already
   validated Power Save on video (−7.7 %) and found no sleep signal above the
   noise floor. A touch-boost experiment is a future option, not shipped.

### Next

1. Build/deploy 0.7.0 and run the device E2E: Service ON → game mapping →
   `sconfig=61` → Service OFF stock restore, with the new thermal trips.
2. Validate the raised thermal trips in a longer sustained session when
   available; 5-minute runs do not reach 58 °C on this device.
3. Keep the 3–5 minute A/B protocol for future sweeps; 5 minutes only when a
   3-minute result is inside noise.


---

## REFERENCES-2026-11-23.md

## Audit repo referensi & adaptasi — 2026-11-23

Empat repo referensi di `/home/<user>/Surya-Profiler/Reference/` diaudit
(subagent, source-level) dan diverifikasi live di device surya (MIUI
14 V14.0.2.0.SJGIDXM, kernel 4.14.190, kondisi stock: service OFF,
`/data/vendor/thermal/config` kosong, `sconfig=0`). Surya-Profiler lisensinya
Apache-2.0; perhatian lisensi per repo di bawah.

### 1. Referensi yang diaudit

| Repo | Lisensi | Verdict |
|---|---|---|
| `vtools_en` (Scene 5.3) | **GPLv3** | Sumber ide terbanyak — dianulir hanya di level spesifikasi/angka, **tidak menyalin kode** (ide/fakta teknis tidak berhak cipta; kode Kotlin/shell-nya GPL) |
| `packages_apps_XiaomiParts` (AOSPA fork) | Apache-2.0 | Bukan sumber node performa (display/audio/haptic); 1 temuan telemetri |
| `mi-thermal-crypt` | no explicit license file | Sudah tercakup — `core/src/engine/crypt.rs` ditulis independen + differential vectors (bukan salinan kode) |
| `android_packages_apps_GameSpace` | Apache-2.0 | Tidak layak untuk MIUI stock (butuh patch framework + privapp) |

### 2. Fakta live device (2026-11-23)

| Cek | Hasil |
|---|---|
| `/proc/sys/kernel/sched_boost_top_app` | **ABSENT** (hanya `sched_boost`, nilai 0, perf-HAL-owned) |
| `/proc/sys/kernel/sched_prefer_sync_wakee_to_waker` | **ABSENT** (node 5.x) |
| `/sys/devices/platform/soc/1d84000.ufshc/{clkscale,clkgate,hibern8}_enable` | Ada, root:root 0644, nilai 1/1/1, mtime segar → **writer aktif = vendor init** |
| Owner ufshc | `/vendor/etc/init/hw/init.qcom.rc` menulis `clkscale_enable` 0/1; perf HAL `vendor.qti.hardware.perf@2.2-service` berjalan |
| `thermal-engine` binary | **Tidak ada** di MIUI 14 — daemon-nya `mi_thermald` (running, root). Rencana "thermal-engine -o doctor dump" dianulir: `thermal.dump`/`last_thermal.dump` + `decrypt.txt` sudah mencakup |
| `/sys/class/drm/sde-crtc-0/measured_fps` | Ada dan terbaca via su (`fps: 0.0 duration:1000000 frame_count:0` saat screen off) — sampler kita **sudah memakai path ini** (`screen_fps`); perlu verifikasi nilai live (60/90) saat E2E |
| prop `sys.thermal.data.path` | `/data/vendor/thermal/` (dasar scanner file asing) |
| `/data/adb/modules` | Tidak ada (APatch tanpa module) — scan tetap defensif |
| Su context | `u:r:magisk:s0` (APatch/FolkPatch) |

### 3. Yang diadaptasi (implementasi 0.7.x)

1. **`forbidden.rs`**: + `sched_boost_top_app`, `sched_prefer_sync_wakee_to_waker`
   (absent-on-kernel, pola sched_coloc_*), + ufshc `clkscale/clkgate/hibern8`
   (vendor init-owned, bukti di atas).
2. **`crypt.rs`**: komentar test dikoreksi ke CLI yang valid (`-i`/`-o` wajib);
   differential vectors dari binary referensi (n=0/1/15/16/17 full-hex,
   4096/4097/8192 sha256, pola non-ASCII 65 byte) — semua terverifikasi
   round-trip; helper `is_encrypted` + `decrypt_or_plain`.
   **Catatan oracle**: binary `mi-thermal-crypt` benar PKCS7 standar (fixture
   vendor 291→304 byte round-trip identik); kebingungan awal audit disebabkan
   lupa flag `-e` (default = decrypt). `decrypt()` kita melakukan CBC penuh
   dulu, jadi kekhawatiran "trailing \n\n lolos unpad" tidak berlaku.
3. **`decrypt_or_plain` dipasang di `vendor_map_entries_at`** — build MIUI lain
   bisa memuat map plaintext (temuan Scene); tanpa ini map akan diam-diam
   fallback ke tabel hardcoded.
4. **Doctor: scanner file thermal asing** (`bridge/thermal/files.rs` +
   `diagnose/thermal_files.rs` + `doctor --fix-thermal`): klasifikasi
   owned/encrypted-map/encrypted-conf/plaintext-map/plaintext-conf/unknown,
   scan module (`/data/adb/modules/*/system*`), prop path, repair
   `chattr -R -i` → hapus asing → `restorecon -DFR` (hanya via CLI saat
   daemon berhenti). Motivasi: file asing mem-bayangi config vendor saat
   `sconfig=0` sehingga "Service OFF = stock" tidak lagi benar.
5. **`measured_fps`**: sudah tercakup sampler (`screen_fps`) — tidak ada
   perubahan kode; verifikasi nilai live di E2E.

### 3b. E2E device scanner (2026-11-23, surya)

| Step | Hasil |
|---|---|
| `doctor` scan, kondisi takeover aktif (owned 6 file) | `thermal_foreign_files pass=true, clean=true`, base ter-resolve dari prop |
| Tanam 3 file asing (`thermal-foreign.conf` blob, `thermal-map.conf.alt` plaintext map, `leftover.bin`) | scan: `pass=false`; klasifikasi tepat — `encrypted-conf`, `plaintext-map` (entries `[0, 9]`), `unknown-binary`; owned tidak terflag |
| Force-stop app (path stdin-EOF 0.6.0) | exit restore daemon: `sconfig=0`, 6 file owned terhapus, **3 file asing bertahan** (bukti masalah nyata) |
| `doctor --fix-thermal` (daemon off) | `removed: [thermal-foreign.conf, thermal-map.conf.alt]`, `failed: []`, `chattr_ok=true`, `restorecon_ok=true`, re-scan `clean=true`, overall `ok=true` |
| Round-trip takeover | deploy→restart daemon: `sconfig=60` + decrypt.txt match; force-stop: `sconfig=0` + config dir bersih (2× konsisten) |
| `screen_fps` live (screen on, swipe) | `fps: 14.9` → `21.9` → `34.9` — telemetri nyata terbaca via `measured_fps` |
| Guard test baru | `guard_path` menolak 5 node baru (sched_boost_top_app, prefer_sync_wakee, ufshc ×3) — unit test `forbidden_paths_rejected` |

### 4. Yang TIDAK diadaptasi (melanggar hard rule / out of scope)

- `msm_performance` unlock 2.5/2.75 GHz (Scene powercfg) — no-op clamp kernel,
  hard rule #1.
- `cpu_boost` powerkey/sched_boost_on_input, `sched_boost(_top_app)` write,
  `sched_load_boost` — forbidden.
- bus-DCVS `llcc-bw`/`llcc-ddr-bw`, devfreq `1d84000.ufshc`, lpm/lmk/vm-swap —
  forbidden / tanpa win terukur (RND-SOC-2026-11-22).
- Remount `/vendor`, replace perfd/perfboostsconfig, uninstall
  `com.miui.daemon`/joyose, stop mi_thermald — bertabrakan coexist/harmony.
- Scene: accessibility foreground (kita: logcat watcher lebih baik), biner
  `scene-scheduler` (tertutup, tidak bisa diaudit, melanggar single-writer),
  swap-controller, xposed addin, freeze/suspend app.
- GameSpace: GameMode API (kosong di MIUI stock, tanpa listener publik),
  `sys.perf_profile` (tidak ada konsumen), FPS via `registerTaskFpsCallback`
  (hidden/privileged — jalur benar: `measured_fps`/`dumpsys gfxinfo`).
- XiaomiParts: KCAL/HBM/CABC/Dirac/haptic/refresh-rate `Settings.System`
  (framework-owned; padanan `persist.sys.aries.*` kita).

### 5. Referensi tuning per-app sm6150 (angka murni, dari Scene
   `powercfg/sm6150/powercfg-utils.sh` `adjustment_by_top_app` + tabel 730g)

Nilai stock-max kita: little 1804800, big 2208000 (setelah unlock constraint
kita: **tanpa msm_performance**, sesuai 0.7.0). Angka Scene yang melebihi itu
di-clamp; hanya dipakai sebagai kandidat untuk iterasi profil per-app
berikutnya, bukan katalog:

| App/kelas | Pengaturan Scene | Adaptasi clamp ke stock |
|---|---|---|
| Genshin (balance) | `sched_config` top-app 50/68 (bg 67/80), `stune.top-app.boost=20`, `gpu_pl_up=2`, freq 1804800/2500000/1939200/2750000 | sched_config & stune dalam range kita; freq big → 2208000 |
| douyin/bilibili (streaming) | `core_ctl` cpu0/cpu7 on, cpuset.foreground 0-3 | kandidat profil "video": core_ctl little on + cpuset |
| game umum (fast) | little max 1804800, big 2208000, schedutil hispeed 1804800/2304000, gpu min_pwrlevel naik 1 step | identik dengan nilai kita saat ini |

Frame-time harness kita (RND-SOC-2026-11-22) adalah alat validasi; angka di
atas hanya memasuki backlog eksperimen, tidak mengubah katalog tanpa A/B.

### 6. Desain tertunda: thread-affinity per-app (Fase 6)

Desain lengkap terkunci 2026-11-23 (diskusi): modul `bridge/affinity/`,
config per-app `affinity: [{comm, mask}]` di `config.json` (schema-2),
eksekusi `taskset -pc <tid>` idempoten (baca `Cpus_allowed_list` dulu),
capture/restore per-tid ala holds.json, guard uid target + mask ⊆ cpuset
profil aktif, opt-in default off, polling 1 s saat game foreground, host
test via fake `/proc` (env var baru `SURYAPROFILER_PROC_ROOT`). Belum
diimplementasi — menunggu doctor/guard stabil.

### 7. Lisensi & atribusi

- Scene/vtools: GPLv3 — semua peminjaman di atas adalah fakta teknis
  (path node, angka, format config, perilaku daemon) atau penulisan-ulang
  dari spesifikasi; tidak ada kode Scene disalin. Biner `scene-scheduler`
  tidak didistribusikan.
- XiaomiParts/GameSpace: Apache-2.0 — atribusi pola tetap
  dicatat di sini sesuai etika. `mi-thermal-crypt` tidak menyertakan file
  lisensi; implementasi kita independen, diverifikasi differential vectors.


---

## REA-2026-11-23.md

## REA reverse-engineering session — 2026-11-23

Target: komponen MIUI/QTI yang **memiliki** parameter yang tidak kita tulis
via profil — supaya peta ownership tidak lagi berbasis observasi runtime
saja. Semua biner ditarik dari device surya (MIUI 14 V14.0.2.0.SJGIDXM,
kernel 4.14.190) via adb+su, dianalisis offline dengan REA (Ghidra 12.2 untuk
ELF aarch64, JADX headless untuk APK). Tidak ada target yang dieksekusi.

### 1. Target & identitas (SHA-256)

| Komponen | Path di device | SHA-256 |
|---|---|---|
| `mi_thermald` | `/vendor/bin/mi_thermald` | `8e205836920d7df442f505d0dfa77e25afb5bf0f6f95d8c7353a948f71f2ca04` |
| QTI perf HAL | `/vendor/bin/hw/vendor.qti.hardware.perf@2.2-service` | `eca08da65f90fc5eed25bad35eeda1ef` |
| power HAL (AIDL) | `/vendor/bin/hw/android.hardware.power-service` | `d41eeb61439e6d7b02e1cc6204c360a9` |
| MIUI `perfservice` | `/system_ext/bin/perfservice` | `a04cac9d9db3328aaa740fcf297a9c2f` |
| `miuibooster` | `/system/xbin/miuibooster` | (tidak didecompile penuh) |
| PowerKeeper | `/system/app/PowerKeeper/PowerKeeper.apk` | `c3c9b99be797af683f372924191fb6a478b013f4455c26cd1fd5c510a22ff5ab` |
| MiuiDaemon | `/system/app/MiuiDaemon/MiuiDaemon.apk` | (tidak didecompile penuh) |

Komponen berjalan saat audit: `mi_thermald` (u:r:mi_thermald:s0),
`vendor.qti.hardware.perf@2.2-service`, `android.hardware.power-service`,
`perfservice`, `miuibooster`, `com.miui.powerkeeper`.

### 2. mi_thermald — model internal terdecompile

Dari `main()` (0x104500) dan fungsi terkait (Ghidra pseudocode, evidence
dalam sesi REA; alamat = offset PIE):

1. **Base dir hardcoded.** `main()` memanggil
   `property_set("vendor.sys.thermal.data.path", "/data/vendor/thermal/")`
   dan tidak pernah membaca prop untuk base. `sys.thermal.data.path` yang
   terisi di device diset oleh komponen lain (PowerKeeper membacanya).
2. **Region map.** Bila `/vendor/etc/thermal-decrypt` ada → mode
   "not use decrypt"; bila tidak, `thermal-region-map.conf` didekrip ke
   `decrypt.txt` dan dicocokkan dengan `ro.boot.hwversion` (surya: 4.90.0).
   **Kedua file tidak ada di surya** → pasangan default terpakai:
   vendor `/vendor/etc/thermal-map.conf` + data
   `/data/vendor/thermal/config/thermal-map.conf`.
3. **Data menang.** `parse_thermal_map` (0x104c88):
   `if (access(DAT_0010e270,0)==0 && param_1==0)` → pakai path DATA, else
   vendor. Ini konfirmasi kode untuk shadowing yang kita tangani scanner.
   Map ID dibatasi `< 0x41` (65) — entri lain ditandai `-1` dan dilewati.
4. **Dua inotify** (mask 2 = IN_MODIFY):
   - `switch_config_thread` (0x105068) memantau
     `thermal_message/sconfig`: setiap tulis → re-read → jika nilai
     **berubah** (`DAT_0010e37c`) → `thermal_switch_config()`.
   - `thermal_config_update` (0x1051a0) memantau direktori
     `/data/vendor/thermal/config/`: setiap tulis file →
     `thermal_switch_config()` dengan ID **saat ini**.
   Implikasi praktis: (a) tulis sconfig dua kali nilai sama = no-op;
   (b) rewrite map/config oleh daemon kita langsung memicu reload tanpa
   harus menulis sconfig lagi; (c) tulisan MIUI yield ke id lain memicu
   konfigurasi yield — persis model kita.
5. **`thermal_switch_config` (0x105280).** Cari entri map dengan
   `id == nilai-sconfig-aktif`; probe `/data/vendor/thermal/config/<name>`
   (chmod 0644 bila salah), fallback `/vendor/etc/<name>`. Bila file ada:
   dekrip → `decrypt.txt` → `device_instance_remove_all_list`,
   `free_ss_algo_info`, `free_monitor_algo_info`, `free_all_tci_info`,
   `load_config`, `ss_algo_init`, `monitor_algo_init`. Bila id **tidak ada**
   di map: re-read `thermal-global-mode` → jadi ID aktif (diulang maks 3×).
   Mutex 0x10e380 menserialisasi.
6. **`thermal_cpu_set_level` (0x109868)** menulis `"<device> <value>"`
   (mis. `cpu0 1324800`) ke fd `cpu_limits` per device + `lseek(0)`.
7. **boost** — `boost_init` (0x10a9f0) butuh device `boost_limit` dari
   `thermald-devices.conf`; tanpa itu "do not allow boost", dengan timer
   restore terdaftar. Node `thermal_message/boost` bukan milik kita.

Node yang ditulis mi_thermald (string + decompile): `sconfig`,
`thermal_message/{cpu_limits,boost,screen_state,board_sensor_temp}`, cooling
`cur_state`, `cpu%d/cpufreq/scaling_max_freq`, `cpu%d/online`,
`/sys/class/power_supply/battery/capacity`. Semua sudah dalam forbidden
kita (`/sys/class/thermal` prefix).

### 3. PowerKeeper (com.miui.powerkeeper) — model sumber-terkonfirmasi

Dari `ThermalSwitchHandler` + `ThermalStoreUtils` (JADX smali):

- **Ruang ID** `writeThermalId(int)`: menolak `id < 0 || id >= 25`
  (base 0–24). Offset sesuai decoder `thermal_scenario.rs` kita:
  `+100` (bila `!mIsDisplayFolded`), `+30` (performance mode aktif),
  `+200` (IEC all-48), lalu IEC-test/SPTM handler. Id 9/13/16/20/23 =
  game → `gameSendIntent`.
- **Idempoten**: bila `mCurrentThermalId == i2` → skip. Konsisten dengan
  change-only mi_thermald.
- **Performance mode chain** `comunicateWithNative(mode, true)`:
  (1) `ThermalSwitchHandler.onPerformenceEnabled(mode)` →
  `THERMAL_ID_DEFAULT = mode`; (2) `SystemProperties.set(
  "persist.sys.thermal.config", mode)`; (3) `storeThermalGlobalMode(mode)`
  menulis file `/data/vendor/thermal/thermal-global-mode`; (4) menulis mode
  langsung ke `THERMAL_COMMINICATE_CONFIG =
  /sys/class/thermal/thermal_message/sconfig`.
  Deteksi yield kita (file global-mode ATAU prop) mencakup semua jalur ini.
- **Path prop**: `PROP_THERMAL_FILE_PATH_NEW = "vendor.sys.thermal.data.path"`,
  `PROP_THERMAL_FILE_PATH = "sys.thermal.data.path"`, default
  `/data/vendor/thermal/`; `getThermalConfigFilePath()` menambah `config/`.
- **Konstanta thermal**: `BOARD_SENSOR_TEMP_FILE =
  /sys/class/thermal/thermal_message/board_sensor_temp`,
  `CPU_NOLIMIT_TEMP_FILE = .../cpu_nolimit_temp`, default
  `CPU_NOLIMIT_TEMP = 25000`, batas kompensasi virtual-sensor 48000, dan
  **kompensasi virtual-sensor per-game** `DEFAULT_SP_ON_SPECIAL_GAME_
  VIRTUAL_SENSOR_COMPENSATION = -2000` untuk
  `com.miHoYo.ys.mi, com.tencent.tmgp.sgame, com.miHoYo.ys.bilibili,
  com.miHoYo.Yuanshen, com.miHoYo.GenshinImpact`.
  (Inilah sebabnya sensor virtual MIUI terasa "lebih dingin" saat game
  tertentu — decoder/dokumen thermal kita bisa memakai ini untuk
  interpretasi `board_sensor_temp` vs `nvm-therm-adc`.)
- Zip whitelist konfigurasi cloud (`confirmPath`): normal, camera, class0,
  4k, 8k, tgame, mgame, phone, nolimits, video, only, map, high, sgame,
  pubgmhb, youtube, arvr, extreme, navigation, demo, unfold, devices, boost,
  india, engine, huanji, videochat, camera2, abnormal, yuanshen,
  dolbyvision, sptm — nama `thermal-<token>.conf`. Scanner file asing kita
  tetap lebih ketat (hanya marker-owned).

### 4. QTI perf HAL — peta node deklaratif (cross-check katalog)

`vendor.qti.hardware.perf@2.2-service` adalah shell tipis (Perf.cpp,
`PerfConfigDataStore::GetProperty`); logika di
`libqti-perfd.so`/`libperfconfig.so`/`libperfgluelayer.so` dan konfigurasi
deklaratif `/vendor/etc/perf/*.xml` (ditarik semua).

- `targetconfig.xml`: target **sdmmagpie** (SocIds 365,366), 2 klaster
  (little 6 + big 2), `CpufreqGov="1"` (**schedutil**), `CoreCtlCpu="0"`
  (core_ctl pada klaster little) — mengonfirmasi invariant kernel kita.
- `commonsysnodesconfigs.xml`: 9 node `msm_performance` (semua di prefix
  forbidden kita).
- `commonresourceconfigs.xml` + `targetresourceconfigs.xml`: **186 node
  unik** yang dikelola/ditulis perf HAL.

#### 4a. SHARED (kita & perf HAL sama-sama menulis) — 26 node

Katalog kita menulis node yang juga dideklarasikan perf HAL. Ini permukaan
"shared": tulisan kita bisa ditimpa sementara oleh hint perf HAL, dan
reconcile supervisor kita yang menyelamatkan. Tidak ada yang perlu
diforbidden — semuanya Baseline dengan drift yang diharapkan:

```
/dev/cpuset/{background,foreground,system-background,top-app}/cpus
/dev/stune/foreground/schedtune.prefer_idle
/dev/stune/top-app/schedtune.{boost,prefer_idle}
/proc/sys/kernel/sched_{downmigrate,upmigrate,group_downmigrate,
  group_upmigrate,little_cluster_coloc_fmin_khz,many_wakeup_threshold,
  migration_cost_ns,min_granularity_ns,min_task_util_for_boost,
  min_task_util_for_colocation,sync_hint_enable}
/sys/class/kgsl/kgsl-3d0/{default,max,min}_pwrlevel
/sys/devices/system/cpu/cpufreq/policy{0,6}/schedutil/{hispeed_freq,
  hispeed_load,pl,down_rate_limit_us}  (XML mengeja cpu0; policy6 = klaster
  big pada surya)
/sys/module/cpu_boost/parameters/input_boost_freq  (sudah terdokumentasi
  sebagai Baseline; post_boot seed + perf HAL save)
```

#### 4b. Perf-HAL-owned yang TIDAK kita sentuh (di luar forbidden)

cpuctl uclamp (`top-app/foreground cpu.uclamp.latency_sensitive`,
`top-app cpu.uclamp.min`, `cpu.notify_on_migrate`), per-task
`/proc/<pid>/sched_boost|sched_low_latency`, `dev/stune/*/schedtune.colocate`,
kgsl `devfreq/{mod_percent,min_freq,max_freq}` + `default_pwrlevel`,
bus_dcvs/llcc/cpubw/memlat family, `interactive`/`ondemand` governor params
(tidak relevan di schedutil), `/dev/cpu_dma_latency`, lpm_levels,
`process_reclaim`, `vm/swap_ratio`, SPECIAL_NODE internal (lock_max_cores,
display_off, irq_balancer, dsb).

**Perubahan guard**: `/dev/cpuctl` ditambah ke `FORBIDDEN_PREFIXES`
(dengan bukti XML; tidak ada entri katalog kita di sana) + 2 assertion
tes. Sisanya sudah tercakup prefix existing atau tidak bisa ditulis dari
katalog (membership gate).

#### 4c. miuibooster

`/system/xbin/miuibooster` (server binder `MiuiBoosterService`): request
`CpuHighFreq / DdrHighFreq / GpuHighFreq / IOHighFreq / ThreadPriority`,
auth via `persist.sys.mibridge_auth_uids` / `mispeed_auth_uids`, membaca
`cpuinfo_max_freq` cpu4/6/7 + devfreq `qcom,cpu0-llcc-ddr-lat`. Ini jalur
"booster MIUI" — bukan permukaan tulis kita; dicatat agar doctor/env tidak
mengartikan lonjakan freq-nya sebagai drift profil kita.

### 5. Perubahan kode yang dihasilkan sesi ini

1. `bridge/thermal/files.rs`: `thermal_base` kini menerima **kedua** prop
   (`vendor.sys.thermal.data.path` → `sys.thermal.data.path`) dan tetap
   menuntut nilai di bawah `/data/vendor/thermal`; scan melaporkan kedua
   prop (`data_path_prop: {new, old}`); komentar selaras dengan temuan REA.
2. `catalog/forbidden.rs`: prefix `/dev/cpuctl` + komentar bukti
   (`commonresourceconfigs.xml`, uclamp hint surface).
3. `catalog/mod.rs`: 2 assertion guard baru (`cpu.uclamp.min`,
   `cpu.notify_on_migrate`).
4. `docs/THERMAL.md`: bagian "REA-confirmed internals".

### 6. Yang TIDAK diubah (dengan alasan)

- **Node shared 4a** tidak diforbidden dan tidak dihapus dari katalog —
  penulisan Baseline + reconcile tetap desain yang benar untuk permukaan
  bersama; menghapusnya justru menyerahkan nilai dasar ke hint perf HAL.
- `schedtune.colocate`, uclamp, per-task `/proc/<pid>/sched_boost` tidak
  ditambahkan ke katalog (permukaan hint perf HAL; Scene juga menulis
  beberapa dari sana — kita sengaja tidak meniru).
- `libthermalfeature.so` (5.9 MB) dan `libthermalclient.so` ditarik tapi
  tidak didecompile penuh (waktu); client terbukti IPC thermal-engine
  standar QTI (`/dev/socket/thermal-send-*`). Tidak ada indikasi node
  katalog baru di sana.

### 7. Batas analisis

- Ghidra pseudocode ≠ sumber asli; zero-init noise membuat beberapa fungsi
  terpotong — bagian yang dikutip sudah dibaca penuh (read-back via view).
- `MiuiDaemon.apk` hanya di-triage strings (tidak decompiled). Keduanya
  di-triage cukup untuk menyimpulkan bukan penulis node katalog baru,
  tapi belum peta fungsi.
- Kesimpulan "shared" berbasis konfigurasi deklaratif vendor; tulisan
  runtime aktual tergantung hint yang benar-benar terjadi di device.

### 8. Kelanjutan sesi (2026-11-24) — chain ownership lengkap

- **`perfservice`** (`/system_ext/bin/perfservice`, sha256
  `7b2e294f…`): proxy binder murni — service `vendor.perfservice` dengan
  interface `com.qualcomm.qti.IPerfManager`, dlopen
  `libqti-perfd-client_system.so` (+ `libqti-iopd-client_system.so`),
  forward `perf_hint`, `perf_lock_acq(_rel)`, `perf_lock_rel`,
  `perf_sync_request`, `perf_ux_engine_events`, `get_perf_hal_ver`.
  **Tidak menulis node sysfs.**
- **`power-hal`** (`android.hardware.power-service`, sha256 `7e5a9aa3…`):
  QTI AIDL power HAL (`vendor/qcom/opensource/power/Power.cpp`) — hint
  `set_interactive`, `setBoost`/`setMode`, dlopen `libqti-perfd-client.so`,
  forward `perf_hint`/`perf_lock_acq`/`perf_lock_rel`; membaca
  `/vendor/etc/powerhint.xml` via **libxml2** (file ada, 13 797 byte).
- Jadi **satu-satunya penulis node di rantai QTI adalah perf HAL**
  (libqti-perfd + XML). Chain: PowerKeeper → `vendor.perfservice` →
  perf HAL → XML → node.
- **`perfboostsconfig.xml`** (sdmmagpie): 18 config, 11 hint ID unik.
  Decode nilai (opcode dipanggulkan komentar XML sendiri): launch 0x1081
  t1 2 s (`sched_boost=1`), resume 0x1082 400 ms, launch t3/t4 **15 s**
  (`sched_boost=3` + sched migrate 95/100), t5/t6 2 s (cpuset top-app=cpu4,
  uclamp per-cpu 60/33, stune 40), input boost 0x1087 persistent
  (min-freq big 1324 / little 1248), touch 0x1080 80 ms (`sched_boost=2`,
  min-freq 1094/1248), restore 0x1083 400 ms. Tabel lengkap:
  `docs/ROM-HARMONY.md`. Nilai `0xFFF` = unset/restore.
- **`powerhint.xml`** ditarik (13 797 byte) untuk audit hint power HAL —
  inventaris hint QTI yang sama channel-nya (perf_hint).
- **`thermal_scenario.rs`**: rentang base dipersempit 0–24 (REA:
  `writeThermalId` menolak `id >= 25`) + helper SP-game
  (`is_miui_sp_game`, list 5 paket) + tes.
- **Diag baru**: `miui_sp_game: Option<bool>` di `DiagInfo` (daemon
  menghitung dari foreground), UI Thermal menampilkan catatan kompensasi
  −2 °C saat aktif.

### 9. E2E game asli: Seven Knights (2026-11-24, surya)

`com.netmarble.skiagb` (Seven Knights Idle, Netmarble) — **bukan** anggota
list SP-game MIUI (kompensasi −2 °C hanya Genshin/sgame/Yuanshen; untuk
Seven Knights `miui_sp_game=false` memang benar, bukan gap). Dipetakan
ke profil Game lewat `config.json` (`app_map`), diuji dengan daemon asli:

| Step | Hasil |
|---|---|
| App start saat layar mati | daemon spawn, `sconfig=63` (sleep profile) |
| Wake + profiler foreground | `sconfig=60` (universal base) |
| Seven Knights cold launch | `sconfig=61` — mapping game lewat daemon asli |
| Gameplay (tap simulasi) | `measured_fps` 42.8 → stabil 29.8 (game idle-lock 30 fps); `sconfig=61` stabil |
| `thermal.dump` selama sesi | MIUI tetap `MONITOR-BAT`/`MONITOR-BCL` — **tanpa override sconfig** untuk Seven Knights |
| HOME (keluar game) | `sconfig=60` — universal base re-assert dari game asli (0.4.2) |
| force-stop (Service OFF) | `sconfig=0`, dir config kosong, `persist.sys.thermal.config=0` — stock restore penuh |

Sesi ini juga menegaskan aturan 0.4.2 pada game nyata: keluar dari game
termapan kembali ke base, dan profiler/launcher sendiri bukan transient.

## 2026-11-24 — 0.8.0 freq floor audit + bukti perilaku floor MIUI

### Konteks: laporan "CPU freq macet tinggi saat idle"

Semua profil menulis `scaling_min_freq` rendah (576/652 kHz kHz-scale:
576000/652800 kHz), jadi freq idle yang tinggi berarti *pihak lain* yang
menahan floor. Sebelum 0.8.0 daemon buta terhadap `scaling_min_freq`
(sampler tidak membacanya) dan hanya watchdog 30 menit yang menulis ulang.

### Probe terkontrol (1 Hz, `scaling_min_freq` policy0/policy6 + sconfig)

Wake tanpa unlock (`KEYCODE_WAKEUP`, tanpa sentuhan lain):

```
02:46:57 lmin=1248000 bmin=979200 sconfig=63   <- wake: MIUI pin overlay
02:47:07 lmin=576000 bmin=652800 sconfig=63   <- +10 s: dilepas, kembali ke floor sleep kita
02:47:08..02:48:25 stabil 576000/652800 (80 s screen-on-idle, TIDAK ada pin)
```

Sesi daemon (log 0.8.0 pertama): app launch (`am start`) menaikkan floor
1248000/979200 selama **~44 s** (launch-boost MIUI, bukan interaksi),
lepas sendiri; audit re-assert pertama (grace 30 s, trigger
`freq_audit`) healed 3 key; sleep apply menulis floor 576/652 dan tetap
setelah screen-off. Kesimpulan:

- **Floor screen-on MIUI adalah overlay berbatas** (wake ~10 s, launch
  ~40+ s) dan saat release MIUI mengembalikan nilai sebelumnya (floor
  profil kita) — sistem harmonis tanpa perang tulis.
- **Screen-off tidak ada pin** yang sah; floor tinggi saat sleep =
  stuck hint / tuner asing — di sinilah sentinel bertindak.
- Tindakan audit dibatasi screen-off; drift screen-on hanya dicatat
  sekali per episode (`MIUI holds the screen-on CPU floor ... release
  expected; not fighting it`), mematuhi hard rule 1.

### E2E device 0.8.0 (pertama, log daemon nyata)

| Step | Hasil |
|---|---|
| Deploy + service start | log file `/data/adb/suryaprofiler/logs/daemon.log` aktif (ring + file + stderr) |
| Launch boost | audit: "MIUI holds the screen-on CPU floor 1248000/979200 kHz ... not fighting it" |
| Grace 30 s (2 sampel @15 s) → re-assert | `evaluate(freq_audit)` → `auto-revive: healed 3 keys for balance` |
| Screen off → sleep | sleep apply; floor 576/652; sconfig 63 |
| force-stop | exit restore ok=true wrote=17 verified=17 failed=0, holds released |

Catatan: audit versi pertama (sebelum revisi ini) sempat re-assert saat
screen-on — eksperimen di atas yang mengoreksi desainnya.

---

## REFERENCES-AND-PLACEMENT-2026-11-24.md

## Audit referensi lanjutan + thread placement 0.9.0 — 2026-11-24

Scope: audit source-level keempat repo referensi (bukan hanya angka), cek
device read-only, lalu implementasi Fase 6 (thread placement) + hardening
thermal/diagnostik.

### 1. Temuan audit yang belum tercatat

| Sumber | Temuan | Tindakan |
|---|---|---|
| Scene `powercfg/sm6150/profile.json` | Mask affinity per-app: Genshin `UnityMain→C0`, `UnityGfx→40`, `UnityMultiRende→20`, `Worker Thread→1C`, other `3F`; sgame `UnityMain/UnityGfx/UnityPreload→C0`; PUBG `RenderThread→80`, `Thread-/TaskGraphNP→70` | Dasar rules `bridge/affinity` (hanya main/render yang diambil; worker/audio tidak) |
| Scene `powercfg/sm6150/active.sh` | `set_cpu_freq` menulis `msm_performance`; powerkey boost; `sched_boost_on_input`; big core_ctl on; UFS clkscale off; angka 2.5/2.75 GHz | Semua ditolak (hard rule #1/#5/#10, OPP big surya max 2304000) |
| Scene `thermal_conf3/config.sh` | `chattr -R -i` → hapus → `restorecon`; rebuild bila `config` jadi file; restart `mi_thermald` | `doctor --fix-thermal` sudah setara; tambah rebuild config-path (0.9.0) |
| Scene `mi-thermal-config/miui-thermal.go` | AES-128-CBC, key=IV `thermalopenssl.h`, PKCS5/7; file plaintext dilewati | Konfirmasi `crypt.rs`; `decrypt_or_plain` sudah menangani map plaintext |
| `mi-thermal-crypt` | **Tidak ada file LICENSE** (bukan Apache-2.0) | Koreksi tabel lisensi di atas |
| XiaomiParts `FPSInfoService` | `measured_fps` = `/sys/class/drm/card0/sde-crtc-0/measured_fps` | Sampler kita sudah memakai path yang sama |
| XiaomiParts refresh/KCAL/HBM/CABC | `Settings.System`/display props, butuh sharedUserId system | Out of scope (framework-owned) |
| GameSpace | `GameManager.setGameMode`, game bar overlay, privapp/system UID | Tidak ada node/trik yang bisa dipakai di MIUI stock |
| Vendor thermal (decrypt) | `thermal-chg-only.conf`/`thermal-engine.conf` **plaintext**; `thermal-normal.conf` memakai `[VIRTUAL-SENSOR]` weight 1 = `nvm-therm-adc` | Koreksi README thermal (bukan “VIRTUAL-SENSOR ditolak”) |

### 2. Cek live device (read-only, sebelum implementasi)

| Cek | Hasil |
|---|---|
| Root / taskset | `u:r:magisk:s0`; `/system/bin/taskset` (toybox) |
| `/proc/<pid>/task/<tid>/{comm,status}` | terbaca; `Cpus_allowed_list` tersedia |
| cpuset stock | bg `0-2`, sysbg `0-3`, fg `0-2,4-7`, fg/boost `4-7`, top-app `0-7` |
| UFS health | `life_time_estimation_a=0x06` (50–60%), `eol_info=0x01` (normal) |
| Thermal context | owned files `thermal_data_file`, 0644; `restorecon` tidak perlu |
| `measured_fps` | terbaca sebagai root |

### 3. Implementasi 0.9.0

- **Config:** `thread_affinity` (schema 2 aditif, default false); Kotlin
  `AppConfig`/`ConfigCodec`/`DynamicProfileConfig` + Settings switch.
- **Daemon:** `bridge/affinity/` (`rules.rs`, `proc.rs`, `mod.rs`), state
  `affinity.json`, `SyncCtx.thread_affinity`, release report
  `affinity_ok`, startup orphan reconcile.
- **Rules:** `UnityMain`/`GameThread` → `c0` (6-7), `UnityGfx*`/
  `RenderThread`/`RHIThread` → `40` (6); other threads untouched.
- **Safety:** exact package + uid ≥ 10000; mask subset dari
  `Cpus_allowed_list`; tidak pernah melebarkan mask; bila app mengubah
  affinity thread yang kita pegang, record dilepas dan thread di-yield
  untuk sesi app itu (tidak dipin ulang); restore hanya bila mask masih
  sama dengan yang kita tulis.
- **Hardening thermal:** `SelectOutcome::Failed` → `install_all()` sekali
  (marker masih ada), lalu retry normal setelah backoff.
- **Doctor/diag:** config-path bukan direktori dilaporkan + `--fix-thermal`
  rebuild; UFS health `storage_life/storage_eol` + kartu Diagnostics.
- **Docs:** README/ARCHITECTURE/PROTOCOL/DEVICE-E2E/AGENTS diperbarui;
  klaim VIRTUAL-SENSOR dan lisensi mi-thermal-crypt dikoreksi.

### 4. E2E device (surya, 2026-11-24)

| Step | Hasil |
|---|---|
| Build/deploy 0.9.0 | APK versionCode 11 / versionName 0.9.0; binary md5 cocok |
| Config `thread_affinity:true`, Seven Knights (`com.netmarble.skiagb`) foreground | `sconfig=61` (game), daemon log `pinned UnityGfxDeviceW->6, UnityGfxDeviceW->6, RenderThread->6, RenderThread->6` |
| Thread masks | `RenderThread` 6, `UnityGfxDeviceW` 6; `UnityMain` sudah 6-7 tanpa tulisan kita (tidak disentuh) |
| Leave ke launcher | `sconfig=60`; `affinity.json` kosong; log `released 4 thread(s)`; mask kembali ke 6-7 (captured old mask) |
| Force-stop app (stdin EOF) | exit cleanup `ok=true restored=true wrote=13 verified=13 failed=0 affinity_ok=true`; 4/4 restore; `affinity.json` kosong |
| Switch OFF | tidak ada `taskset`/entry (host E2E `placement_stays_off...`) |
| Corrupt `thermal-game.conf` (7 byte) lalu masuk game | log `select failed ... configs reinstalled (vendor)`; file kembali 1200 byte; `decrypt.txt=[GAME-SS-CPU6]`; retry → `game active (sconfig 61)` |
| Leave game | `sconfig=60`, `decrypt.txt=[BAL-SS-CPU6]` |
| Doctor | overall `ok=true`; `storage_health` `0x06`/`0x01`; `thermal_takeover` pass |
| Final state | service ON, base balance, `sconfig=60`, `thread_affinity=false`, `affinity.json` kosong, stayon false |

### 5. Keputusan

1. **Default OFF** dan hanya untuk app yang di-map ke Game. `UnityMain`
   yang sudah 6-7 tanpa tulisan kita dibiarkan; pin kita fokus ke render
   thread.
2. **Tidak melawan app:** jika app menulis ulang affinity thread yang
   kita pegang, record dilepas. Ini menghindari perang `taskset` tiap 3 s.
3. **Tidak ada node katalog baru.** Thread placement adalah operasi proses
   (`taskset`), bukan sysfs; guard-nya package/uid/mask.
4. **Thermal repair** aman: hanya menulis ulang file milik marker; tidak
   menyentuh config vendor/asing.
5. **UFS health read-only** masuk doctor/diag; tidak ada write.

---

## RENDER-SCALE-2026-11-24.md

## Render scale + display scale R&D dan 0.10.0 — 2026-11-24

### 1. Mekanisme

- **Per-app:** Android 12 GameManagerService shell
  `cmd game downscale <ratio> <pkg>` → PlatformCompat
  `DOWNSCALED` + `DOWNSCALE_xx` → `CompatModePackages`. Uniform buffer
  downscale; layout/dp/aspect tetap, hanya DPR yang turun.
- **Global:** `wm size <WxH>` + `wm density <D>`; exact 20:9 presets, density
  mengatur layout UI (bukan performa).
- Root `cmd` berfungsi (`u:r:magisk:s0`); `cmd game help` keluar 255 di AOSP
  walau sukses, jadi feature probe harus membaca teksnya, bukan exit code.
- Capture prior per-app dari `dumpsys platform_compat`; capture
  size/density dari `wm size`/`wm density` (override atau physical).

### 2. Uji global 720×1600 (surya)

| Density | dp width | Catatan |
|---:|---:|---|
| 240 | 480dp | kecil |
| **267** | **431dp** | sama dengan layout native 432dp |
| 293 | 393dp | ~10% lebih besar |
| **320** | **360dp** | spec 720p vendor (`set_density_by_fb`), UI lebih besar/terbaca |
| 400 | 288dp | terlalu besar |

ROM: `system/system/build.prop` `ro.sf.lcd_density=440`;
`vendor/bin/init.qcom.early_boot.sh` → lebar ≥720 = 320, ≥1080 = 480;
device memakai override `display_density_forced=400`.
Aspect 720×1600 = 20:9 exact; screenshot 720×1600; touch benar.

### 3. Per-app: layout/DPI otomatis

Chrome JS metrics:
- native: `inner 432×815`, DPR 2.5.
- native + 0.5: `inner 432×815`, **DPR 1.25**, `mCompatFrame=540×1200`,
  `mGlobalScale=2.0`.
- native + 0.65: `mCompatFrame=702×1560`, `mGlobalScale=1.538`.
- 720p/320 + 0.8: `mCompatFrame=576×1280`, `mGlobalScale=1.25`.
Jadi aspect/layout/dp tidak berubah; hanya buffer/DPR. Perubahan live dapat
merestart app (Chrome), jadi apply persistent sebelum launch.

### 4. Daya (60 s, brightness 400, 60Hz, `input_suspend=1`)

Chrome WebGL stress (GPU-bound):

| Kondisi | mW | GPU% | Delta |
|---|---:|---:|---:|
| Native 1080p | 2209.6 | 85.1 | — |
| 720p @320 | 1636.7 | 65.1 | −26% |
| 720p @267 | 1571.7 | 66.3 | −29% |
| Native + 0.65 | 1564.0 | 66.5 | −29% |
| 720p @320 + 0.8 | 1309.8 | 49.0 | −41% |

Seven Knights:

| Kondisi | mW | GPU% | Delta |
|---|---:|---:|---:|
| Native | 2781.1 | 79.1 | — |
| 720p @320 | 1789.3 | 80.2 | −36% |
| Native + 0.8 | 2039.3 | 75.4 | −27% |

TikTok (powersave): native 1911.8 mW (GPU 8.0%) vs 720p 2011.7 (4.5%) vs
native+0.65 1912.8 (4.4%) vs 720p+0.8 1970.0 (3.4%) — **tidak ada win**;
decode/network/panel dominan. Density tidak mengubah buffer app (tetap
720×1600), jadi bukan knob performa.

### 5. E2E device 0.10.0

| Step | Hasil |
|---|---|
| Deploy | APK 0.10.0 / versionCode 12; binary md5 cocok |
| Config `app_downscale` game 0.8 | log `downscale 80% (prior native)`; `downscale.json` record |
| Seven Knights | `mCompatFrame=864×1920`, `mGlobalScale=1.25`, `mFrame=1080×2400`, `sconfig=61` |
| Display `hd` | `wm size 720×1600`, density 320; `display.json` original `{size:null,density:400}` |
| Combo hd + game 0.8 | `mFrame=720×1600`, `mCompatFrame=576×1280`, `mGlobalScale=1.25` |
| Force-stop (stdin EOF) | `display: restored size physical, density 400`; `downscale restored to native`; state JSON kosong; `downscale_ok/display_ok=true`; `wm` 1080×2400 @400 |
| Final state | service ON, base balance, `sconfig=60`, config `display_scale=off`, `app_downscale={}` |

UI screenshots: Home pada 720p/320 tetap rapi; Settings native/720p/267/320
dan app row badge diverifikasi.

---

## POWER-USAGE-2026-11-26.md

## Power usage (0.11.0) — implementation, host + device evidence — 2026-11-26

Scope decided with the user after auditing `Reference/vtools_en`:
`ActivityPowerUtilization` (peak record, battery % curve, per-app average)
and `ActivityCharge` (charge speed/capacity/time/temp). Our version adds
battery-side watts, charger input when the kernel exposes it,
`input_suspend`/charge-guard warnings, Device Profile attribution and
screen-off coverage, while staying **read-only**: no charge limit/control.

### Design

- `engine/env/power.rs`: `PowerProbe` reads `battery` (capacity/status/
  current/voltage/temp/charge_counter/charge_full/cycle_count/
  input_suspend/battery_charging_enabled) and the first online input rail
  (`usb`/`pc_port`/`main`/`dc`/`wireless`). Sign is the verified device
  convention: negative = charging, positive = discharging.
- `daemon/power/`: sampler 3 s screen-on / 30 s screen-off; one-minute
  buckets; 7-day `power-history.jsonl` append-only; all-time
  `power-peaks.json`; clean-exit flush; `power_clear`.
- Attribution: active Device Profile (including Sleep) every sample;
  foreground package only while screen-on **and unlocked**.
- Protocol v3 additive: `power {watch, window_s?}` / `power_clear` ->
  `power` snapshot + `power_live` (sample + in-progress bucket).
- UI: Home -> Power usage; live card, charge session, signed chart
  (1h/6h/24h + screen-off strip), peak tiles, by-profile, by-app, clear.
  Sessions/per-app/profile math is client-side in pure Kotlin.

### Host verification (2026-11-26)

- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`: clean.
- Rust: **222 lib tests** (+11 power/bucket/store/probe) and **20 daemon
  E2E** (+`daemon_power.rs`: fake tree, live stream, one-minute-equivalent
  buckets, charge/discharge peaks, screen-off context, clear).
- Kotlin: **40 JVM tests** (+5 parser, +11 model) and `assembleDebug`.
- Asset lockstep: cross-built arm64 daemon is 0.11.0 and byte-identical to
  `app/src/main/assets/surya-prof`; `check.sh` passes.

### Device evidence (surya, 2026-11-26)

Build: APK 0.11.0 / versionCode 13 (final APK md5
`037505293a58bc06b0f504b5f443a899`); daemon binary md5
`b4f08d85021c0ab947f237d4478461ac`. Android 12 / MIUI 14, plugged USB.

| Step | Result |
|---|---|
| Read-only nodes | `battery/current_now=-798645 µA`, `voltage_now=4403272 µV`, `status=Charging`, `capacity=85`, `temp=344`, `charge_counter=3731030 µAh`, `charge_full=5111000`, `cycle_count=41`, `input_suspend=0`, `battery_charging_enabled=1` |
| Input rail | `usb` has **no** `voltage_now`/`current_now` files; its `uevent` reports `VOLTAGE_NOW=4748096`, `INPUT_CURRENT_NOW=1317690`, `REAL_TYPE=USB_CDP`. `main` has `input_voltage_settled=5000000` / `input_current_settled=1500000` but no `online`. The probe's uevent/settled fallbacks selected `usb (USB_CDP)`. |
| Live UI | `Charging · 85%`, 797 mA, 3.51 W, 4.40 V, 34.6 °C, health 5111 mAh / 99% of 5160 / 41 cycles, input `usb (USB_CDP) · 4.78 V · 1.18 A · 5.64 W`, est. full 1h39m — matches sysfs at the same time |
| Bypass (Settings switch) | `input_suspend=1` → status Discharging, `+464 mA`, 1.94 W usage, warning “Charging bypass active (R&D input_suspend=1)”, est. left 8h08m; input rail still reported 5.18 V / 671 mA. Switch off → `input_suspend=0`, Charging, warning gone |
| Session card | Transient bypass correctly changed “Charging session · now” → “Last charge session” with end time (live-direction override; the 1-min bucket average was still charge) |
| Screen-off | A 60 s bucket with `screen_on:false`, `profile:"sleep"`, `samples:2` (deep-sleep 30 s cadence) was recorded and appeared as a Sleep profile row |
| Stats | `By profile`: Sleep 1.41 W / 337 mA / 1m, Balance 1.03 W / 246 mA / 2m. `By app`: Surya Profiler ≈1.03 W / 246 mA / 34.9 °C avg / 35.2 °C max / 2m |
| Peak record | `charge -802002 µA @ 4.40 V`, `discharge +545959 µA @ 4.33 V`, `temp 35.1 °C`, all timestamped; `power-peaks.json` schema 1 |
| History | `power-history.jsonl` one line/minute (mixed bypass bucket example: avg `-365234`, min `-800171`, max `+545959`, 60 s, 20 samples); file survived Service OFF/ON and reloaded |
| Chart | 1h/6h/24h selector works; 1h rendered signed usage (purple) / charge (orange) and the screen-off strip; 1-minute-average caveat documented |
| Clear | Confirm dialog → both `power-history.jsonl` and `power-peaks.json` deleted; live sampling continued (new session 1 mAh / 6 s), then history re-accumulated |
| Service OFF | Full UI path: 19/19 written, 19 verified, 0 failed; `state.active=null / last_mode=restore`, `sconfig=0`, `holds` all false; `power-history.jsonl` persisted. Power screen showed the service-off CTA. |
| Service ON | Daemon restarted, `active=balance`, `sconfig=60`, history reloaded and a new minute appended |

Final state: service ON (base balance), `sconfig=60`, `input_suspend=0`,
`battery_charging_enabled=1`, `holds.json` healthy, app on the Power screen.
No power node was written by the new feature; the only charge write was the
existing R&D switch, restored to 0.

Known caveat: short charge/usage switches inside one minute are averaged in
the chart (the live card and peak record still show the instantaneous
extremes). This is documented in the chart caption.

---

## PHASE-0-2026-11-26.md

## Phase 0 safety + telemetry integrity — 2026-11-26

### Scope

Stabilize the daemon/service and Power telemetry before scenario-aware
optimization. No refresh/DFPS, MIUI power-mode, charge-control or new sysfs
surface was added.

- Startup foreground peek + stale-result sequencing; periodic `dumpsys`
  fallback; lock screen returns to the universal base.
- Two-phase bridge sync updates saver attribution before slow thermal work.
- Thermal release preserves decoded MIUI offset scenarios (`112`, `108`,
  `117`, `118`, …); forced reconcile never raises a stricter external cap.
- Unowned `input_suspend=1` recovery; atomic/fsynced write-ahead hold storage;
  fail-closed recovery when a live thermal takeover has no trustworthy holds.
- OS-backed exclusive `flock` (`daemon.lock`) replaces the PID create/unlink
  race; `daemon.pid` remains diagnostic only.
- Power uses actual elapsed time, wakes on screen changes, joins before final
  flush, merges same-minute segments, dedupes durable replays, reconciles
  peaks, heals legacy duplicate-minute files on load, and makes Clear
  fail-closed. A zero-duration first reading contributes extrema/peaks without
  creating a represented-time row.
- Protocol v3 adds segment bounds/energy/ID; the app invalidates stale live
  data on disconnect and resubscribes after the hello handshake.

### Host verification

- `./scripts/check.sh`: **264 Rust unit tests**, **21 daemon E2E**, **54 Kotlin
  JVM tests**, fmt/clippy/architecture/asset lockstep and APK build all pass.
- The Power E2E exposed and verified the zero-duration-first-reading fix:
  charge/discharge extrema survive a direction change before the first
  interval, while suspend gaps still create no bucket.

### Device evidence (surya, final 0.11.0)

Final daemon md5 `b14e4fe1b55b704730b1c1024372f9c3`; APK md5
`6db01e7720f11fedbb8d1914f072ad7b`; deployed binary md5 matched.

| Step | Result |
|---|---|
| Startup | `seed: peeked=Some("com.suryaprofiler")` → `balance (base)`; `sconfig=60` |
| Legacy history heal | Startup log `history: merged 70 rows into 67 unique minutes`; the JSONL file was atomically rewritten (new size/mtime) |
| Power reconnect | Power screen showed live 677 mA / 2.88 W; SIGKILL daemon PID 3778 → supervisor restarted PID 4986, recovered holds, reapplied Balance and live sampling resumed |
| Crash recovery | Restart log `bridge: recovered holds from previous run`; custom thermal map refreshed and `sconfig=60` restored without a second writer |
| Camera ownership | Camera foreground → `bridge: thermal: yielding to MIUI camera`, `sconfig=12` |
| Service OFF | `restore: ok=true wrote=19 verified=19 failed=0`; every release report true; `active=null / last_mode=restore`; daemon exited |
| MIUI scenario preserved | After Service OFF, `sconfig=12` (camera) remained; `doctor` reported `scenario:"camera"`, `marker:false`, no custom/legacy indices, `thermal_takeover.pass:true` |
| Stock safety state | `input_suspend=0`, `battery_charging_enabled=1`, `low_power=0`, `wm size=1080x2400`, override density 400; `holds.json` all false/empty |
| Doctor | Overall `ok:true`; battery 70%, 35.3 °C, charging |

Earlier in the same Phase 0 campaign, a pre-final candidate kept YouTube on
Power Save after killing its daemon, recovered the mapped profile through the
startup peek, rejected an older out-of-order peek, and rejected a second
daemon through the OS lock. The final build retained those paths and repeated
the crash/restart recovery above.

Not exercised live: injecting an orphan `input_suspend=1` was denied by the
device's SELinux policy; backups were restored and the node remained `0`.
The write-ahead/orphan recovery path is covered by host unit tests and must be
re-attempted on a device/policy that permits the injection.

Final state: Service OFF, stock restore verified, MIUI camera thermal scenario
preserved, Power history/peaks retained for the next run.
