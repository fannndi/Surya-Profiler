# tools

R&D helper scripts used on-device during the evidence sessions
(`adb` + `su`). They are **one-off probes**, not part of the product;
read each script before running it on a daily-use device.

- `probe_catalog.sh` — catalog node existence/permission/value probe
- `rnd_*.sh` / `rnd_*.py` — A/B run harness, frame-time sampler, write probes
- `audit_*.sh` — system/MIUI/stack audits (read-only)
- `game-stockcpu.variant.json` — variant used by the 0.7.0 Game-efficiency run
- `perf-hal-runtime-writers.txt` — perf-HAL runtime writer evidence
  (referenced by `core/src/engine/catalog/forbidden.rs`)
