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

`set_base` is rejected while a restore is in progress or the service is
disabled. `restore` closes the bridge gate immediately.

## Events

| `event` | Payload | Meaning |
|---|---|---|
| `hello` | `version`, `pid` | first event after startup |
| `state` | `state` object (`screen_on`, `locked`, `multi_window`, `second_window`, `foreground`, `active`, `reason`, `src_pkg`) | visible runtime state changed |
| `applied` | `profile`, `reason`, `ok`, `applied`, counters, `ms`, `settle_ms` | apply finished |
| `restored` | `ok`, counters, `saver_ok`, `input_suspend_ok`, `thermal_ok` | Service OFF restore finished |
| `env` | `env` | 30 s telemetry sample (on change) |
| `diag` | `diag` | reply to `diag` |
| `stats` | `entries`, `heals_*` | reply to `stats` |
| `decision` | `trigger`, `action`, `profile`, `reason` | arbiter trace (tests/diagnostics) |
| `error` | `msg` | non-fatal error |
| `pong` | — | reply to `ping` |
| `bye` | — | graceful exit |

`restored.ok` is true only when the engine restore **and all holds** succeeded.
`applied.ok` is false when a real device had zero written/unchanged keys and
every key was locked; `applied` distinguishes a real apply from a fully locked
profile. Host simulations (`SURYAPROFILER_DEBUG=1`, no device nodes) keep the
all-missing case successful.

Unknown commands are logged and ignored. Unknown events are non-fatal and
parsed as `DaemonEvent.Unknown`.
