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
