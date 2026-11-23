# Architecture

Two runtimes, one rule: **the Rust engine is the only writer; the Kotlin app
is a thin client.**

```
Compose UI ──► ViewModel ──► DynamicProfileService / DaemonLink ──┐
                                                       ▼
                                   DynamicProfileService (FGS)
                                                       │ JSON-lines over stdio
                                                       ▼
                              surya-prof serve (root, Rust daemon)
                                                       │
                        arbiter → worker → engine (catalog/plan/apply)
                                                       │
                                   bridge (saver/input-suspend/thermal)
```

## Dependency rules

1. `platform` (paths/fs/shell) knows nothing.
2. `engine` may use `platform`; it must **not** import `daemon`.
3. `daemon` may use `engine`; `cli`/`diagnose` may use both.
4. Kotlin `protocol` and `config` are pure (no `android.*`).
5. `ui` may use `protocol` and the `dynamic` service facade; `dynamic`/`daemon` must not import `ui`.
6. One file = one concept, target 100–250 production lines. Data tables and
   tests are exempt.
7. Every profile write passes `catalog::guard_path`; the thermal takeover uses
   its own explicit allowlist (`daemon/bridge/thermal/io.rs`).

`scripts/arch-check.sh` enforces rules 2, 4 (Kotlin purity), 5 and the
300-line budget.

## Rust map

| Path | Responsibility |
|---|---|
| `core/src/engine/catalog/` | writable registry, tiers/kinds, forbidden guard |
| `core/src/engine/catalog/entries/` | entries split by cpu/gpu-io/kernel/misc |
| `core/src/engine/probe.rs` | read-only capture + options |
| `core/src/engine/plan.rs` + `plan/` | validated plan, write rank, invariants |
| `core/src/engine/apply/` | snapshot → guarded writes → verify → restore |
| `core/src/engine/env/` | telemetry (model/parsers/sampler/scenario/sysfs) |
| `core/src/engine/thermal_*.rs` | MIUI `sconfig` decoder + `mi_thermald` dump parser |
| `core/src/diagnose/` | `doctor` (battery/miui/env/thermal checks) |
| `core/src/daemon/arbiter/` | pure decision table |
| `core/src/daemon/bridge/` | saver/input-suspend/thermal holds + lifecycle |
| `core/src/daemon/bridge/thermal/` | profile/paths/io/perf/map/install/select/sync/release |
| `core/src/main.rs` | binary entry |
| `core/src/cli.rs` + `cli/commands.rs` | CLI dispatch and one-shot commands |
| `core/src/platform.rs` | process runner with hard timeout (output drained) |
| `core/src/daemon/run.rs` | daemon PID claim + thread wiring + main loop + worker-owned exit cleanup |

## Kotlin map

| Path | Responsibility |
|---|---|
| `protocol/` | `ProfileId`, `DaemonCommands`, `DaemonEvent`, diagnostics parsers |
| `config/` | `AppConfig` + pure `ConfigCodec` (schema/migration) |
| `core/` | `RootBridge` (su/deploy), `FtClient`, engine facade, labels |
| `dynamic/` | FGS, daemon process, state, stock restore, receivers/tile |
| `ui/` | Home, Apps, Settings, Diagnostics, reusable cards/dialogs |

## Recipes

- **Add a parameter:** add an `Entry` in `engine/catalog/entries/<domain>.rs`;
  run `cargo test` (duplicate/forbidden checks); add it to `profiles.json` if
  a profile needs it.
- **Add a profile:** add to `profiles.json` + `ProfileId`; update
  `daemon::config::*_PROFILES` and the thermal table only if it takes over
  thermal.
- **Add a daemon command/event:** add to `daemon/proto.rs`, handle it in
  `daemon/commands.rs`, mirror it in `protocol/DaemonCommands.kt` or
  `protocol/DaemonEvent.kt`, and add a host/unit test.
- **Add a thermal scenario:** update `engine/thermal_scenario.rs` (PowerKeeper
  IDs), then the yield policy test in the same file.
- **Change Service OFF:** the only path is `Bridge::release_all` +
  `StockRestore`; every surface must report success before the service stops.

## Verification

```bash
./scripts/check.sh          # fmt, clippy, Rust tests, Kotlin tests, APK
./scripts/build.sh          # cross-build + APK
./scripts/deploy.sh         # adb install + health summary
```

Device evidence lives in `docs/HISTORY.md` and `docs/EVIDENCE-THERMAL.md`.
