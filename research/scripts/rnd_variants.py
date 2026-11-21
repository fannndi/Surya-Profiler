#!/usr/bin/env python3
"""Generate A/B variant profiles for RND experiments.

Usage:
  rnd_variants.py --id SL2-capL --base sleep \
      --set policy0.scaling_max_freq=768000 --set policy6.scaling_max_freq=1094400 \
      --out research/raw/rnd/variants/SL2-capL.json

Replaces the params of `--base` profile inside core/profiles.json with its
own params overridden by --set key=value pairs. Output keeps the same profile
id so `surya-prof apply <id> --profiles <out>` picks it up.
"""
import argparse
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[2]


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--id", required=True)
    ap.add_argument("--base", required=True)
    ap.add_argument("--set", dest="overrides", action="append", default=[])
    ap.add_argument("--drop", dest="drops", action="append", default=[],
                    help="remove a key from the base params")
    ap.add_argument("--out", required=True)
    a = ap.parse_args()

    src = json.loads((ROOT / "core/profiles.json").read_text())
    prof = next(p for p in src["profiles"] if p["id"] == a.base)
    params = dict(prof["params"])
    for d in a.drops:
        params.pop(d, None)
    for o in a.overrides:
        k, v = o.split("=", 1)
        if v == "":
            params.pop(k, None)
            continue
        params[k] = v
    out = dict(src)
    out["comment"] = f"RND variant of {a.base}: {a.id}"
    out["profiles"] = [{**prof, "params": params}]
    path = pathlib.Path(a.out)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(out, indent=2) + "\n")
    print(f"wrote {path} ({len(params)} params)")


if __name__ == "__main__":
    main()
