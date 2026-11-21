#!/usr/bin/env python3
"""Summarize RND A/B sampler CSVs (research/raw/rnd/*.csv).

Metrics per file:
  win_s, mean_mw (current*voltage), integ_mah (trapezoid of current),
  cc_mah (charge_counter drop), cc_rate_mah_h, batt/quiet/nvm temp mean-max,
  p0/p6 mean cur/max freq, mean active_cpus, gpu busy mean.

Usage: rnd_report.py research/raw/rnd/SL*.csv
"""
import csv
import glob
import sys


def fnum(x, default=0.0):
    try:
        return float(str(x).replace(" %", "").strip())
    except (TypeError, ValueError):
        return default


def load(path):
    rows = []
    with open(path, newline="") as fh:
        r = csv.DictReader(fh)
        for row in r:
            if not row or "t" not in row:
                continue
            rows.append(row)
    return rows


def summarize(path):
    rows = load(path)
    if len(rows) < 3:
        return None
    t = [fnum(r["t"]) for r in rows]
    dt_total = t[-1] - t[0]
    if dt_total <= 0:
        return None
    power_mw = [fnum(r["batt_uA"]) * fnum(r["batt_uV"]) / 1e9 for r in rows]
    # trapezoid integration of current (uA * s -> uAh -> mAh)
    integ_uas = 0.0
    for i in range(1, len(rows)):
        integ_uas += (fnum(rows[i]["batt_uA"]) + fnum(rows[i - 1]["batt_uA"])) / 2 * (t[i] - t[i - 1])
    integ_mah = integ_uas / 3600.0 / 1000.0
    cc = [fnum(r["charge_counter_uAh"]) for r in rows]
    cc_drop_uah = cc[0] - min(cc)
    cc_rate_mah_h = cc_drop_uah / 1000.0 / (dt_total / 3600.0)
    temps = {
        "batt": [fnum(r["batt_temp_dC"]) / 10 for r in rows],
        "quiet": [fnum(r["quiet_mC"]) / 1000 for r in rows],
        "nvm": [fnum(r["nvm_mC"]) / 1000 for r in rows],
    }
    p0_cur = [fnum(r["p0_cur"]) / 1000 for r in rows]
    p0_max = [fnum(r["p0_max"]) / 1000 for r in rows]
    p6_cur = [fnum(r["p6_cur"]) / 1000 for r in rows]
    p6_max = [fnum(r["p6_max"]) / 1000 for r in rows]
    p0_act = [fnum(r["p0_active"]) for r in rows]
    gpu_busy = [fnum(r["gpu_busy"]) for r in rows]
    msm = [str(r.get("msm_min", "0")) for r in rows]
    boost = [fnum(r.get("sched_boost", "0")) for r in rows]
    floor_frac = sum(1 for m in msm if "1612800" in m or "1843200" in m) / len(msm)
    boost_frac = sum(1 for b in boost if b >= 1) / len(boost)
    return {
        "file": path.split("/")[-1],
        "win_s": round(dt_total),
        "mean_mw": round(sum(power_mw) / len(power_mw), 1),
        "integ_mah": round(integ_mah, 1),
        "cc_mah": round(cc_drop_uah / 1000, 1),
        "cc_rate": round(cc_rate_mah_h, 1),
        "batt_t": round(max(temps["batt"]), 1),
        "quiet_t": round(max(temps["quiet"]), 1),
        "nvm_t": round(max(temps["nvm"]), 1),
        "p0_cur": round(sum(p0_cur) / len(p0_cur)),
        "p0_max": round(sum(p0_max) / len(p0_max)),
        "p6_cur": round(sum(p6_cur) / len(p6_cur)),
        "p6_max": round(sum(p6_max) / len(p6_max)),
        "p0_act": round(sum(p0_act) / len(p0_act), 1),
        "gpu_busy": round(sum(gpu_busy) / len(gpu_busy), 1),
        "floor_frac": floor_frac,
        "boost_frac": boost_frac,
    }


def main() -> None:
    files = []
    for pat in sys.argv[1:]:
        files.extend(p for p in glob.glob(pat) if p.endswith(".csv"))
    print(f"{'file':28s} {'s':>5s} {'mW':>7s} {'mAh':>6s} {'ccmAh':>6s} {'ccmAh/h':>7s} "
          f"{'bT':>4s} {'qT':>4s} {'nT':>4s} {'p0cur':>6s} {'p0max':>6s} {'p6cur':>6s} {'p6max':>6s} {'act0':>4s} {'gpu%':>5s} {'regime':>8s}")
    for path in sorted(files):
        s = summarize(path)
        if not s:
            print(f"{path}: not enough samples")
            continue
        regime = "floor" if s.get("floor_frac", 0) > 0.5 else ("boost" if s.get("boost_frac", 0) > 0.2 else "none")
        print(f"{s['file']:28s} {s['win_s']:5d} {s['mean_mw']:7.1f} {s['integ_mah']:6.1f} "
              f"{s['cc_mah']:6.1f} {s['cc_rate']:7.1f} {s['batt_t']:4.1f} {s['quiet_t']:4.1f} {s['nvm_t']:4.1f} "
              f"{s['p0_cur']:6d} {s['p0_max']:6d} {s['p6_cur']:6d} {s['p6_max']:6d} {s['p0_act']:4.1f} {s['gpu_busy']:5.1f} {regime:>8s}")


if __name__ == "__main__":
    main()
