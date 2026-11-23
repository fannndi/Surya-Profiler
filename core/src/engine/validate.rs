//! Per-kind value validation against live probe options.
//!
//! Responsibility: `(resolved, None)` when acceptable (clamped to real OPPs /
//! core_ctl bounds), `(raw, Some(reason))` when locked.
//! Non-goals: plan/status (`plan.rs`), read-back compare (`readback.rs`).

use crate::engine::catalog::{self, Kind};
use crate::engine::probe::ProbeData;

/// Validate `wanted` for an entry against live options.
/// Returns `(resolved, None)` or `(raw, Some(reason))` when locked.
pub fn validate_value(
    e: &catalog::Entry,
    wanted: &str,
    probe: &ProbeData,
    planned_max_cpus: Option<i64>,
) -> (String, Option<String>) {
    let wanted = wanted.trim();
    let locked = |msg: &str| (wanted.to_string(), Some(msg.to_string()));
    match e.kind {
        Kind::Int | Kind::RepeatInt => match wanted.parse::<i64>() {
            Ok(v) if v >= 0 => {
                if let Some(scope) = e.min_cpus_of {
                    // kernel: if (val < state->num_cpus) return -EINVAL
                    let n = probe.options.cluster_cpus.get(scope).copied().unwrap_or(0);
                    if n > 0 && (v as usize) < n {
                        return locked(&format!("must be >= {n} (cluster CPU count, core_ctl.c)"));
                    }
                }
                if let Some((lo, hi)) = e.range {
                    if v < lo || v > hi {
                        return locked(&format!("out of range {lo}..={hi} (kernel bound)"));
                    }
                }
                (v.to_string(), None)
            }
            _ => locked("not a non-negative integer"),
        },
        Kind::MinCpus => match wanted.parse::<i64>() {
            Ok(v) if v >= 0 => {
                // kernel clamps silently: min(val, max_cpus) — pre-clamp so
                // read-back verification matches instead of flagging drift.
                // Prefer the max_cpus THIS plan writes: the live value may
                // still be the previous (sleep) profile's cap, which would
                // otherwise make a wake apply silently under-apply.
                let max =
                    planned_max_cpus.or_else(|| probe.options.core_ctl_max.get(e.scope).copied());
                match max {
                    Some(m) => (v.min(m).to_string(), None),
                    None => (v.to_string(), None),
                }
            }
            _ => locked("not a non-negative integer"),
        },
        Kind::Freq | Kind::FreqMin | Kind::FreqMax => {
            let n: u64 = match wanted.parse() {
                Ok(n) => n,
                Err(_) => return locked("not a frequency"),
            };
            let best = match probe.options.freqs.get(e.scope) {
                Some(list) if !list.is_empty() => {
                    // Direction matters: a cap must not round up above the
                    // requested cap; a floor must not round down below it.
                    match e.kind {
                        Kind::FreqMax => list
                            .iter()
                            .filter(|f| **f <= n)
                            .max()
                            .copied()
                            .unwrap_or_else(|| *list.iter().min().unwrap()),
                        Kind::FreqMin => list
                            .iter()
                            .filter(|f| **f >= n)
                            .min()
                            .copied()
                            .unwrap_or_else(|| *list.iter().max().unwrap()),
                        _ => *list
                            .iter()
                            .min_by_key(|f| (**f as i64 - n as i64).unsigned_abs())
                            .unwrap(),
                    }
                }
                _ => return locked("no OPP list for scope"),
            };
            // MIUI's `msm_performance.cpu_min_freq` can raise the effective
            // floor during a wake/touch boost (interaction-scoped). A cap
            // below the live floor is silently clamped, so plan it Locked and
            // let apply retry once after the boost decays; never write a
            // value that can never read back during a persistent floor.
            if e.kind == Kind::FreqMax {
                let min_key = format!("{}.scaling_min_freq", e.scope);
                let live_min = probe
                    .entries
                    .get(&min_key)
                    .and_then(|s| s.value.as_deref())
                    .and_then(|v| v.trim().parse::<u64>().ok());
                if let Some(floor) = live_min {
                    if best < floor {
                        return locked(&format!(
                            "max {best} below live floor {floor} (MIUI power policy)"
                        ));
                    }
                }
            }
            (best.to_string(), None)
        }
        Kind::PwrLevel => match wanted.parse::<usize>() {
            Ok(v) if probe.options.gpu_levels > 0 && v < probe.options.gpu_levels => {
                (v.to_string(), None)
            }
            _ => locked(&format!(
                "pwrlevel out of range 0..{}",
                probe.options.gpu_levels.saturating_sub(1)
            )),
        },
        Kind::Gov => one_of(
            wanted,
            probe.options.governors.get(e.scope).map(|v| v.as_slice()),
            "governor",
        ),
        Kind::GpuGov => one_of(wanted, Some(&probe.options.gpu_governors), "gpu governor"),
        Kind::IoSched => one_of(wanted, Some(&probe.options.io_schedulers), "io scheduler"),
        Kind::TcpCc => one_of(wanted, Some(&probe.options.tcp_cc), "tcp cc"),
        Kind::Ints => {
            let toks: Vec<&str> = wanted.split_whitespace().collect();
            if toks.is_empty()
                || !toks
                    .iter()
                    .all(|t| t.parse::<i64>().map(|v| v >= 0).unwrap_or(false))
            {
                return locked("not a list of non-negative integers");
            }
            (toks.join(" "), None)
        }
        Kind::Mask => match validate_mask(wanted, probe.options.cpu_count) {
            Ok(norm) => (norm, None),
            Err(msg) => locked(&msg),
        },
        Kind::FlagYN => match wanted.to_ascii_uppercase().as_str() {
            "Y" | "N" | "0" | "1" => (wanted.to_ascii_uppercase(), None),
            _ => locked("expected Y/N/0/1"),
        },
        Kind::Text => {
            if wanted.is_empty() {
                locked("empty value")
            } else {
                (wanted.to_string(), None)
            }
        }
    }
}

fn one_of(wanted: &str, options: Option<&[String]>, what: &str) -> (String, Option<String>) {
    match options {
        Some(opts) if opts.iter().any(|o| o == wanted) => (wanted.to_string(), None),
        Some(opts) => (
            wanted.to_string(),
            Some(format!("{what} not available (have: {})", opts.join(" "))),
        ),
        None => (
            wanted.to_string(),
            Some(format!("{what} options unreadable")),
        ),
    }
}

/// Validate a CPU mask (`0-5`, `0,2,4-7`) against the online CPU count.
pub fn validate_mask(want: &str, cpu_count: usize) -> Result<String, String> {
    let mut parts = Vec::new();
    for tok in want.split(',') {
        let tok = tok.trim();
        if tok.is_empty() {
            return Err("empty mask token".into());
        }
        let segs: Vec<&str> = tok.split('-').collect();
        let (lo, hi) = match segs.as_slice() {
            [a] => {
                let v: usize = a.parse().map_err(|_| format!("bad cpu '{a}'"))?;
                (v, v)
            }
            [a, b] => {
                let x: usize = a.parse().map_err(|_| format!("bad cpu '{a}'"))?;
                let y: usize = b.parse().map_err(|_| format!("bad cpu '{b}'"))?;
                if x > y {
                    return Err(format!("inverted range {tok}"));
                }
                (x, y)
            }
            _ => return Err(format!("bad mask token '{tok}'")),
        };
        if hi >= cpu_count {
            return Err(format!("cpu {hi} >= cpu_count {cpu_count}"));
        }
        parts.push(if lo == hi {
            lo.to_string()
        } else {
            format!("{lo}-{hi}")
        });
    }
    Ok(parts.join(","))
}

#[cfg(test)]
mod tests;
