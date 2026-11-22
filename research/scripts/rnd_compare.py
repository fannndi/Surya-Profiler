#!/usr/bin/env python3
"""Compare A/B/B/A short-run power + frame captures.

Usage: rnd_compare.py PREFIX
Reads research/raw/rnd/PREFIX-*.csv and .frames (tags like PREFIX-A1).
"""
import glob
import sys

from rnd_frames_report import summarize as frames_summary
from rnd_report import summarize as power_summary


def main():
    prefix = sys.argv[1]
    rows = []
    for csv in sorted(glob.glob(f"research/raw/rnd/{prefix}-*.csv")):
        tag = csv.split("/")[-1].replace(".csv", "")
        p = power_summary(csv)
        if not p:
            continue
        f = None
        fp = f"research/raw/rnd/{tag}.frames"
        try:
            f = frames_summary(fp)
        except FileNotFoundError:
            pass
        rows.append((tag, p, f))
    if not rows:
        print("no captures")
        return
    print(
        f"{'tag':18s} {'mW':>7s} {'mAh':>6s} {'p0':>5s} {'p6':>5s} "
        f"{'gpu%':>5s} {'qT':>4s} {'p95':>6s} {'p99':>6s} {'jank%':>6s} {'fps':>5s}"
    )
    by = {"A": [], "B": []}
    for tag, p, f in rows:
        letter = "A" if "-A" in tag else "B" if "-B" in tag else "?"
        print(
            f"{tag:18s} {p['mean_mw']:7.1f} {p['integ_mah']:6.1f} "
            f"{p['p0_cur']:5d} {p['p6_cur']:5d} {p['gpu_busy']:5.1f} "
            f"{p['quiet_t']:4.1f} "
            f"{(f['p95'] if f else 0):6.2f} {(f['p99'] if f else 0):6.2f} "
            f"{(f['jank33'] if f else 0):6.1f} {(f['fps'] if f else 0):5.1f}"
        )
        if letter in by:
            by[letter].append((p, f))

    def avg(xs):
        return sum(xs) / len(xs) if xs else 0.0

    if by["A"] and by["B"]:
        print()
        for key, label in (("mean_mw", "mW"), ("integ_mah", "mAh"),
                           ("p0_cur", "p0 MHz"), ("p6_cur", "p6 MHz"),
                           ("gpu_busy", "GPU %"), ("quiet_t", "quiet C")):
            a = avg([p[key] for p, _ in by["A"]])
            b = avg([p[key] for p, _ in by["B"]])
            d = b - a
            pct = (d / a * 100.0) if a else 0.0
            print(f"{label:9s} A={a:8.2f} B={b:8.2f} delta={d:+7.2f} ({pct:+5.1f}%)")
        for key, label in (("p95", "p95 ms"), ("p99", "p99 ms"),
                           ("jank33", "jank33%"), ("fps", "fps")):
            fa = [f[key] for _, f in by["A"] if f]
            fb = [f[key] for _, f in by["B"] if f]
            if fa and fb:
                a, b = avg(fa), avg(fb)
                d = b - a
                pct = (d / a * 100.0) if a else 0.0
                print(f"{label:9s} A={a:8.2f} B={b:8.2f} delta={d:+7.2f} ({pct:+5.1f}%)")


if __name__ == "__main__":
    main()
