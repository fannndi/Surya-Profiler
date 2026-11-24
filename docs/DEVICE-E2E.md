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
- [ ] Thread placement ON (Settings → Advanced) + a Game-mapped app in front:
      `UnityMain`/`UnityGfx*` show the pinned `Cpus_allowed_list`
      (`6-7` / `6`), other threads untouched; leave the app → all restored;
      Service OFF → restored again and `restored.affinity_ok=true`.
- [ ] Thread placement OFF → no `taskset` calls (log/strace) and no
      `affinity.json` entries.
- [ ] Per-app render scale: map a Game app to 80%; check `mCompatFrame`
      (e.g. 864×1920 at native, `mGlobalScale=1.25`) while `mFrame` stays
      native; aspect ratio/layout unchanged; Service OFF → `disable` and
      `downscale_ok=true`.
- [ ] Render scale prior capture: set 0.8 on a package via `cmd game
      downscale 0.8 <pkg>`, then configure 0.7; Service OFF must restore 0.8.
- [ ] Global display scale: preset `hd` (720×1600 @320); `wm size`/density
      read back; SystemUI/apps relayout; exact restore 1080×2400 @400 on Off.
      Check `hd-compact` (267) and 80% (864×1920 @320) too.
- [ ] Combination: `hd` + game 80% → `mCompatFrame=576×1280`, aspect 20:9;
      Service OFF restores both (`display_ok`/`downscale_ok`).
- [ ] Reboot while render/display holds are active; daemon reconciles, Service
      OFF restores; `downscale.json`/`display.json` empty afterwards.
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

Evidence goes into `docs/HISTORY.md` (dated section).
