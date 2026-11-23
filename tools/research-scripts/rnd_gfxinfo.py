#!/usr/bin/env python3
"""Parse `dumpsys gfxinfo <pkg>` captures produced by rnd_ab.sh.

Extracts: total frames, janky frames count+percent, and 50/90/95/99th frame
time percentiles. Usage: rnd_gfxinfo.py research/raw/rnd/*.gfxinfo
"""
import re
import sys

PAT_TOTAL = re.compile(r"Total frames rendered: (\d+)")
PAT_JANKY = re.compile(r"Janky frames: (\d+) \(([\d.]+)%\)")
PAT_PCT = re.compile(r"(\d+)th percentile: (\d+)ms")


def parse(path):
    total = janky = jank_pct = None
    pcts = {}
    for line in open(path, errors="replace"):
        m = PAT_TOTAL.search(line)
        if m:
            total = int(m.group(1))
        m = PAT_JANKY.search(line)
        if m:
            janky, jank_pct = int(m.group(1)), float(m.group(2))
        m = PAT_PCT.search(line)
        if m:
            pcts[int(m.group(1))] = int(m.group(2))
    return total, janky, jank_pct, pcts


def main() -> None:
    print(f"{'file':32s} {'frames':>7s} {'janky':>7s} {'jank%':>6s} {'p50':>5s} {'p90':>5s} {'p95':>5s} {'p99':>5s}")
    for path in sys.argv[1:]:
        total, janky, jank_pct, pcts = parse(path)
        g = lambda p: pcts.get(p, "-")
        print(f"{path.split('/')[-1]:32s} {total or '-':>7} {janky or '-':>7} {jank_pct or '-':>6} "
              f"{g(50):>5} {g(90):>5} {g(95):>5} {g(99):>5}")


if __name__ == "__main__":
    main()
