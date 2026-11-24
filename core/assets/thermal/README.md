# Custom thermal configs

Plaintext sources for the four configs Surya Profiler installs under
`/data/vendor/thermal/config` (encrypted at runtime by
`core/src/engine/crypt.rs`). They are embedded in the daemon via
`include_str!` and selected by private `sconfig` indices:

| File | `sconfig` | Used by |
|---|---:|---|
| `thermal-balance.txt` | 60 | Balance |
| `thermal-game.txt` | 61 | Game |
| `thermal-powersave.txt` | 62 | Power Save |
| `thermal-sleep.txt` | 63 | Sleep |

Why 60–63: PowerKeeper's `ThermalSwitchHandler` writes base IDs 0–25 plus
offsets `+30` (performance), `+100` (foreground), `+200`/`+500` (IEC). Its
package map maps YouTube to 21, for example. 60–63 is outside every MIUI
value and below `mi_thermald`'s map-ID limit (IDs > 64 are rejected on this
build).

The map is generated at install time from `/vendor/etc/thermal-map.conf` plus
the four private entries. Stock entries (including 4K/8K fallbacks 17/18)
are preserved across MIUI updates; the hardcoded map is only a host/fallback.

Rules:

- `SS`/GPU steps use `nvm-therm-adc` **directly**. The vendor config wraps
  the same sensor in a weight-1 `[VIRTUAL-SENSOR]`; a config that references
  `VIRTUAL-SENSOR` must define it (vendor does). Pointing at `nvm-therm-adc`
  directly is equivalent on surya and avoids a redundant alias.
- Keep big-cluster targets at/above the 1 843 200 kHz **live boost** floor and
  little-cluster targets at/above 1 612 800 kHz; lower values are clamped when
  the boost is active. The floor is interaction-scoped (wake/touch), so these
  are floor-aware upper steps, not a hardcoded screen-on constant. Sleep
  (screen off) may use lower targets.
- `MONITOR-BAT` is copied **exactly** from vendor `thermal-normal.conf`.
  It is MIUI's stock charging-current ladder, not a new charge control.
- Keep `MONITOR-TEMP_STATE`, `MONITOR-BCL` and a `CCC` section. Do not ship
  a `nolimits`-style config without the CCC/backlight safety net.
- Update `core/src/daemon/bridge/thermal/tests.rs` when changing a frequency/step; the daemon
  verifies the decrypted `decrypt.txt` against these files before selecting a
  profile.
- Game trips were raised on 2026-11-22 (SS-CPU6 58/62 °C, SS-CPU0 60 °C,
  GPU 60/64 °C, CCC 63 °C). Kernel LMH/BCL and `MONITOR-BAT`/`TEMP_STATE`
  remain the hard limit; a sustained >10 min game validation is still the
  right check after any further trip change.

Stock MIUI remains the fallback: the custom map keeps `0` → vendor
`thermal-normal.conf`. MIUI camera (12/17/18), phone (8), ARVR (15) and
performance mode are yielded to; video/game/benchmark keep the active custom
profile.
