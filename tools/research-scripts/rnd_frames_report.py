#!/usr/bin/env python3
"""Summarize SurfaceFlinger --latency captures.

The CSV has three nanosecond columns per line (desired, present, ready) plus
optional `# layer=` comments. Duplicate frames from overlapping 1 s polls are
deduplicated by present timestamp.

Usage: rnd_frames_report.py research/raw/rnd/*.frames
"""
import glob
import sys


def percentile(xs, p):
    if not xs:
        return 0.0
    xs = sorted(xs)
    k = (len(xs) - 1) * p / 100.0
    lo = int(k)
    hi = min(lo + 1, len(xs) - 1)
    return xs[lo] + (xs[hi] - xs[lo]) * (k - lo)


def summarize(path):
    frames = {}
    layer = ""
    for line in open(path, errors="replace"):
        line = line.strip()
        if line.startswith("#"):
            layer = line
            continue
        parts = line.split()
        if len(parts) != 3:
            continue
        try:
            desired, present, ready = (int(v) for v in parts)
        except ValueError:
            continue
        if present <= 0:
            continue
        frames[present] = (desired, ready)
    ts = sorted(frames)
    if len(ts) < 5:
        return None
    intervals = []
    for a, b in zip(ts, ts[1:]):
        dt = (b - a) / 1e6
        # Ignore app pauses / poll gaps; keep real jank below 500 ms.
        if 0 < dt < 500:
            intervals.append(dt)
    if len(intervals) < 3:
        return None
    # Use only intervals that belong to the active render stream; a stale
    # buffer entry from another clock/era would otherwise poison the span.
    active_ms = sum(intervals)
    fps = len(intervals) * 1000.0 / active_ms if active_ms > 0 else 0
    jank20 = sum(1 for x in intervals if x > 20.0) / len(intervals) * 100
    jank33 = sum(1 for x in intervals if x > 33.0) / len(intervals) * 100
    p99 = percentile(intervals, 99)
    return {
        "file": path.split("/")[-1],
        "layer": layer.replace("# layer=", "")[:40],
        "frames": len(ts),
        "fps": fps,
        "p50": percentile(intervals, 50),
        "p90": percentile(intervals, 90),
        "p95": percentile(intervals, 95),
        "p99": p99,
        "max": max(intervals),
        "jank20": jank20,
        "jank33": jank33,
        "low1": 1000.0 / p99 if p99 > 0 else 0,
    }


def main():
    files = []
    for pat in sys.argv[1:]:
        files.extend(p for p in glob.glob(pat) if p.endswith(".frames"))
    print(
        f"{'file':24s} {'frames':>7s} {'fps':>6s} {'p50':>6s} {'p90':>6s} "
        f"{'p95':>6s} {'p99':>7s} {'max':>7s} {'jank20%':>8s} {'jank33%':>8s} {'1%low':>6s}"
    )
    for path in sorted(files):
        s = summarize(path)
        if not s:
            print(f"{path.split('/')[-1]:24s}  (not enough frames)")
            continue
        print(
            f"{s['file']:24s} {s['frames']:7d} {s['fps']:6.1f} {s['p50']:6.2f} "
            f"{s['p90']:6.2f} {s['p95']:6.2f} {s['p99']:7.2f} {s['max']:7.2f} "
            f"{s['jank20']:8.1f} {s['jank33']:8.1f} {s['low1']:6.1f}"
        )


if __name__ == "__main__":
    main()
