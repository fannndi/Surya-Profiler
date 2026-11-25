# Daemon protocol (JSON-lines over stdio)

- **stdin:** commands (app → daemon)
- **stdout:** events (daemon → app), one JSON object per line
- **stderr:** human logs (relayed to logcat under `SuryaProfiler`)
- Closing stdin is the shutdown signal. The daemon releases all holds and
  writes the stock snapshot back before exiting.

Protocol version: **3** (`hello.version`); `bypass_*` was renamed to
`input_suspend_*` (old `holds.json` files are read through aliases). Kotlin constants:
`DaemonCommands.PROTOCOL_VERSION`.

## Commands

| `cmd` | Payload | Meaning |
|---|---|---|
| `hello` | — | handshake, daemon replies with `state` |
| `ping` | — | liveness → `pong` |
| `shutdown` | — | graceful exit (closes the restore gate first) |
| `config_changed` | — | reload `config.json` + re-evaluate now |
| `screen` | `on: bool`, `locked: bool` | screen/keyguard state |
| `user_present` | — | keyguard dismissed |
| `fg` | `pkg: string` | foreground event (tests / compatibility) |
| `set_base` | `profile: "powersave"\|"balance"\|"game"` | apply + become universal base |
| `restore` | — | Service OFF: drop applies, release holds, restore stock |
| `diag` | — | diagnostics snapshot |
| `stats` | — | transition history + watchdog totals |
| `power` | `watch: bool`, `window_s?: u64` | power snapshot; `watch=true` also starts live `power_live` samples (default window 24 h, clamped 10 min–7 d) |
| `power_clear` | — | clear power history + all-time peaks, then reply with an empty snapshot |

`set_base` is rejected while a restore is in progress or the service is
disabled. `restore` closes the bridge gate immediately.

## Events

| `event` | Payload | Meaning |
|---|---|---|
| `hello` | `version`, `pid` | first event after startup |
| `state` | `state` object (`screen_on`, `locked`, `multi_window`, `second_window`, `foreground`, `active`, `reason`, `src_pkg`) | visible runtime state changed |
| `applied` | `profile`, `reason`, `ok`, `applied`, counters, `ms`, `settle_ms` | apply finished |
| `restored` | `ok`, counters, `saver_ok`, `input_suspend_ok`, `thermal_ok`, `affinity_ok`, `downscale_ok`, `display_ok` | Service OFF restore finished |
| `env` | `env` | 30 s telemetry sample (on change) |
| `power` | `snapshot` | reply to `power`/`power_clear` (`window_s`, `live`, `current_bucket`, `history`, `peaks`) |
| `power_live` | `sample`, `bucket?` | 3 s screen-on / 30 s screen-off sample while watching; `bucket` is the in-progress minute |
| `diag` | `diag` | reply to `diag` |
| `stats` | `entries`, `heals_*` | reply to `stats` |
| `decision` | `trigger`, `action`, `profile`, `reason` | arbiter trace (tests/diagnostics) |
| `error` | `msg` | non-fatal error |
| `pong` | — | reply to `ping` |
| `bye` | — | graceful exit |

`restored.ok` is true only when the engine restore **and all holds** succeeded
(saver, input-suspend, thermal, thread placement, render scale, display scale). `applied.ok` is false when
a real device had zero written/unchanged keys and every key was locked;
`applied` distinguishes a real apply from a fully locked profile. Host
simulations (`SURYAPROFILER_DEBUG=1`, no device nodes) keep the all-missing
case successful.

Unknown commands are logged and ignored. Unknown events are non-fatal and
parsed as `DaemonEvent.Unknown`.

## Power payload

All power values are read-only telemetry: the daemon never writes
`power_supply` nodes for this feature. The app sends `power` when the Power
screen opens/changes window, and `power {watch:false}` when it leaves.

Units and sign:

- `current_ua`: µA, device sign — **negative = charging, positive =
  discharging** (verified 2026-10-09/22 with `input_suspend`).
- `voltage_uv`: µV. Battery-side power = `abs(current_ua) × voltage_uv /
  1e9` mW. While charging this is net into the battery, not wall power.
- `input` is a charger rail (`usb`/`main`/…) when the kernel exposes one;
  values are reported/negotiated, never measured wall power. On surya the
  `usb` rail has no `voltage_now`/`current_now` files: the probe parses its
  `uevent` (`VOLTAGE_NOW`, `INPUT_CURRENT_NOW`, `REAL_TYPE`) and falls back
  to `main`'s `input_voltage_settled`/`input_current_settled`; `kind` carries
  `USB_CDP`/`USB_PD`/`Main` when reported.
- `input_suspend` is the R&D bypass node (read-only here; the bridge owns
  the write). `charge_enabled` is the framework charge guard
  (`battery_charging_enabled`), also read-only.

Sampler: **3 s screen-on / 30 s screen-off**. History is one-minute buckets,
append-only in `<state_dir>/power-history.jsonl`, seven-day retention,
all-time peaks in `power-peaks.json`. A clean daemon exit closes the
in-progress bucket; a SIGKILL loses at most that minute. `power_clear` drops
both files and resets the in-memory ring.

```json
{"event":"power","snapshot":{"window_s":86400,
  "live":{"t_ms":1700000000000,"pct":78,"status":"charging",
          "current_ua":-1240000,"voltage_uv":4120000,"temp_c":32.4,
          "charge_counter_uah":3980000,"charge_full_mah":5008,
          "cycle_count":34,"input_suspend":false,"charge_enabled":true,
          "input":{"name":"main","online":true,
                   "voltage_uv":9000000,"current_ua":1500000},
          "screen_on":true,"pkg":"com.foo","profile":"game"},
  "current_bucket":{"t_ms":1700000000000,"pct":78,"avg_ua":-1240000,
    "min_ua":-1600000,"max_ua":-900000,"avg_uv":4120000,
    "avg_temp_c":32.4,"max_temp_c":33.1,"screen_on":true,
    "dur_ms":60000,"samples":12,"pkg":"com.foo","profile":"game"},
  "history":[],
  "peaks":{"charge":{"ua":-2000000,"uv":4100000,"t_ms":1,"pct":60}}}}
```

`power_live` carries the same `sample` plus the current (partial) `bucket`;
the app merges buckets by `t_ms` so the chart moves between snapshots.
`window_s` is clamped to 600–604800. `history` is filtered to the requested
window; `peaks` is all-time until Clear.

Aggregation rules: package attribution uses the daemon's **last real
foreground** package and only when the screen is on and unlocked; profile
attribution uses the active profile (including `sleep`). A bucket is
`screen_on` when most of its samples were; charge sessions, per-app and
per-profile averages are derived client-side from the bucket list (time
weighted). Peaks pair a bucket's extreme current with that bucket's average
voltage; this is documented so a peak watt is not mistaken for a
sample-exact value.

## Diagnostics payload

`diag.diag` carries the config, env snapshot, watchers, holds, the newest
`thermal_events`, `miui_sp_game`, storage health (`storage_life`,
`storage_eol`, raw UFS hex), `downscale_held/apps`, `display_held/preset` and
`recent_logs` — the last ~120 daemon
log lines (oldest first, `<utc> <tag> [component] message`; `tag`
E/W/I/D). The `holds` object also exposes `affinity_held`, `affinity_app`
and `affinity_threads` for the opt-in thread placement.

## Config keys (schema 2)

| key | default | Meaning |
|---|---|---|
| `enabled`, `dynamic`, `custom_thermal` | true | service / app mapping / thermal takeover |
| `base_profile` | `"balance"` | universal base (`powersave`/`balance`/`game`) |
| `app_map` | `{}` | package → profile (user profiles only) |
| `rnd_input_suspend` | false | R&D: suspend charger input while the service runs |
| `freq_floor_audit` | true | re-assert the profile CPU floor when an external writer pins it high (yields to MIUI performance mode; backs off 30 s→300 s when a re-assert does not stick) |
| `thread_affinity` | false | opt-in Game thread placement: pin Unity/Unreal main+render threads while a Game-mapped app is foreground; restore on leave/screen-off/lock/Service OFF |
| `app_downscale` | `{}` | per-app render scale (`cmd game downscale`), canonical `0.3`–`0.9`; prior captured/restored |
| `display_scale` | `"off"` | optional system-wide 20:9 preset: `80`, `75`, `70`, `hd`, `hd-compact`, `60`, `50`; exact `wm size`/density restored on Service OFF |
| `log_level` | (env) | `error`/`warn`/`info`/`debug`; the env var `SURYAPROFILER_LOG` seeds the default, a config `log_level` (when present) overrides it — applied live on every config reload |
