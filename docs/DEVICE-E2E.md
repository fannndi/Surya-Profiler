# Device E2E checklist

Run after any engine/daemon/service change. Device: POCO X3 NFC (`surya`),
MIUI 14, root APatch.

## Build + deploy

```bash
./scripts/build.sh
./scripts/deploy.sh
```

## Core matrix

- [ ] `doctor` → `ok:true`; `thermal_takeover` pass; map has 18 entries.
- [ ] Base Balance: `sconfig=60`, `decrypt.txt` starts `[BAL-SS-CPU6]`.
- [ ] Video app (Mi Video/VLC, **not YouTube** — it may enter PIP) → Power Save `62`;
      log `keeping custom over MIUI video (21)`.
- [ ] Camera → yields to `12`; return → `60`.
- [ ] Epic Seven → Game `61`; return → `60`.
- [ ] Performance mode on → yields to `sconfig=1`; off **without touching
      sconfig** → reasserts `60`.
- [ ] Sleep (screen off) → `63`; wake → base.
- [ ] Split screen → Balance.
- [ ] MIUI saver follows a Power Save-mapped app, then restores the user value.
- [ ] R&D input-suspend ON → node `1`; Service OFF → `0`.
- [ ] Service OFF (UI + notification action): `sconfig=0`, map/marker gone,
      `holds.json` all false, engine restore `18/18` (count depends on the
      installed pack), charging untouched.
- [ ] Daemon crash/stop path: `surya-prof clean` restores saver/input-suspend/
      thermal without the app.
- [ ] `battery_charging_enabled=1`, `low_power=0`.
- [ ] Sleep→wake with Game as base: `core_ctl` little cluster 6/6, not 2/6.
- [ ] `surya-prof apply` while the daemon is alive is rejected (single writer).
- [ ] Force-stop the app: `surya-prof status` shows `active=null`, `last_mode=restore`.
- [ ] `doctor --config /data/data/com.suryaprofiler/files/config.json`:
      deployed profiles + restore baseline pass.

## Reboot (optional)

- [ ] Service ON survives reboot; boot receiver starts the daemon.
- [ ] No `holds.json` leftovers after a clean stop.

Evidence goes into `research/DEVICE-E2E-2026-11-19.md`.
