# Thermal takeover (MIUI 14 / surya)

Four custom encrypted configs are installed under
`/data/vendor/thermal/config` and selected by private `sconfig` IDs:

| Profile | `sconfig` | Config |
|---|---:|---|
| balance | 60 | `thermal-balance.conf` |
| game | 61 | `thermal-game.conf` |
| powersave | 62 | `thermal-powersave.conf` |
| sleep | 63 | `thermal-sleep.conf` |

`mi_thermald` accepts map IDs 0–64; 60–63 is outside every MIUI base ID
(0–25) and offset (`+30/+100/+200/+500`). The custom map is generated from
`/vendor/etc/thermal-map.conf`, preserving stock entries and adding 17/18
fallbacks when absent.

## Yield policy

- **Yield:** camera `12/17/18`, phone `8`, ARVR `15`, performance mode
  (`sconfig=1` with `thermal-global-mode` or `persist.sys.thermal.config=1`).
- **Keep custom:** video `21`, game `9/13/16`, benchmark `10`, navigation
  `19`, game-normal `20`, yuanshen `23`, unknown values.
- A stale `sconfig=1` **without** the performance-mode flag is not a yield;
  the daemon reasserts the custom profile.

## Cleanup / ownership

- Marker `.suryaprofiler` owns the config dir. A marker-less map is adopted
  only when its private IDs point at our four config names.
- Service OFF removes the **map first**, then configs, then marker; a failed
  map removal keeps the marker so the next attempt can retry.
- Unknown `sconfig` makes `mi_thermald` keep its last config, so release
  forces stock normal when needed.
- `surya-prof clean` releases saver/input-suspend/thermal holds without a
  running daemon; `restore` runs it as its final step.

## Filesystem layout

- `daemon/bridge/thermal/profile.rs` — profile/ID table + embedded configs
- `paths.rs` — all thermal paths
- `io.rs` — guarded writes + read-back verification
- `perf.rs` — MIUI performance-mode detection/yield helpers
- `map.rs` — vendor map parse/build/ownership
- `install.rs` / `select.rs` / `sync.rs` / `release.rs`

The only sysfs node written is `thermal_message/sconfig`; config files are
allowlisted by name. Detailed evidence: `research/THERMAL-MIUI14.md`.

## REA-confirmed internals (2026-11-23)

Decompiled from the shipped binaries (evidence:
`research/REA-2026-11-23.md`):

- **`mi_thermald` reload triggers** — two inotify watches, mask `IN_MODIFY`:
  (1) `thermal_message/sconfig` — every write makes it re-read the node and,
  when the value *changed*, re-run `thermal_switch_config`; (2) the whole
  `/data/vendor/thermal/config/` directory — any file write there re-runs
  `thermal_switch_config` for the *current* ID. This is exactly why our sync
  (rewrite configs, then select) works, and why a second select write with an
  unchanged value is a no-op.
- **Selection order** — `parse_thermal_map` uses
  `/data/vendor/thermal/config/thermal-map.conf` whenever it exists
  (`access()==0`), else `/vendor/etc/thermal-map.conf`; region map
  (`thermal-region-map.conf` + `ro.boot.hwversion`) does not exist on surya.
  Map IDs ≥ 65 are rejected (`id < 0x41`), and a missing/invalid entry makes
  the daemon fall back to the value in `thermal-global-mode`.
- **Fallback config path** — for a selected map entry it probes
  `/data/vendor/thermal/config/<name>` first, then `/vendor/etc/<name>`, and
  chmods the /data copy to 0644 when wrong. Stock entries in our generated
  map therefore always resolve to vendor configs.
- **`thermal_cpu_set_level`** — writes `"<device> <value>"` lines into the
  per-device `cpu_limits` fd (e.g. `cpu0 1324800`) and lseeks back.
- **`boost`** — mi_thermald's boost feature needs a `boost_limit` device
  entry (`thermald-devices.conf`); without it "boost is not allowed".
- **PowerKeeper (`com.miui.powerkeeper`)** — base ID space 0–24 with offsets
  `+100` (unfolded multi-screen), `+30` (performance mode), `+200` (IEC
  all-48), then IEC/SPTM test handlers; writes the resolved ID straight to
  `thermal_message/sconfig` and skips when unchanged. It also writes
  `board_sensor_temp` readers / `cpu_nolimit_temp` and applies per-game
  *virtual-sensor compensation* (Genshin/sgame/Yuanshen list: `-2000`).
- **Performance mode chain** (source-confirmed) —
  `ThermalSwitchHandler.onPerformenceEnabled(1)` sets the default thermal ID,
  `SystemProperties.set("persist.sys.thermal.config","1")`,
  `storeThermalGlobalMode(1)` writes `/data/vendor/thermal/thermal-global-mode`,
  then `sconfig` gets "1". Our yield detection (global-mode file OR prop)
  covers the whole chain.
- **Path props** — `mi_thermald` sets `vendor.sys.thermal.data.path` to its
  hardcoded base and never reads a prop; PowerKeeper resolves
  `vendor.sys.thermal.data.path` → `sys.thermal.data.path` → hardcoded
  `/data/vendor/thermal/`. The scanner accepts either prop only when it
  points under `/data/vendor/thermal`.
