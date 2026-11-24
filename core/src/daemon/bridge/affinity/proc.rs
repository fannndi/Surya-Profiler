//! `/proc` reads + the `taskset` call used by thread placement.
//!
//! Everything is rooted at [`Host::proc_root`] and [`Host::taskset`] so host
//! tests can use a fake tree/script. On a real device the defaults are
//! `/proc` and `/system/bin/taskset`; when the test-only
//! `SURYAPROFILER_SYSFS_ROOT` is set without an explicit
//! `SURYAPROFILER_PROC_ROOT`, placement is disabled instead of touching the
//! host's real processes.

use crate::platform;
use std::path::PathBuf;
use std::time::Duration;

pub(crate) struct Host {
    pub(crate) proc_root: PathBuf,
    pub(crate) taskset: PathBuf,
}

impl Host {
    /// Resolves the live host, or `None` in a host simulation without an
    /// explicit fake `/proc`.
    pub(crate) fn detect() -> Option<Host> {
        let proc_root = match std::env::var_os("SURYAPROFILER_PROC_ROOT") {
            Some(p) => PathBuf::from(p),
            None if std::env::var_os("SURYAPROFILER_SYSFS_ROOT").is_some() => return None,
            None => PathBuf::from("/proc"),
        };
        let taskset = std::env::var_os("SURYAPROFILER_TASKSET_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/system/bin/taskset"));
        Some(Host { proc_root, taskset })
    }

    fn task_dir(&self, pid: u32) -> PathBuf {
        self.proc_root.join(pid.to_string()).join("task")
    }
}

/// Finds the main process of `pkg` (uid >= 10000) and its `starttime`.
pub(crate) fn resolve_pid(host: &Host, pkg: &str) -> Option<(u32, u64)> {
    let rd = std::fs::read_dir(&host.proc_root).ok()?;
    let mut found: Option<(u32, u64)> = None;
    for entry in rd.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|s| s.parse::<u32>().ok())
        else {
            continue;
        };
        if cmdline_arg0(host, pid).as_deref() != Some(pkg) {
            continue;
        }
        if read_uid(host, pid).is_none_or(|uid| uid < 10_000) {
            continue;
        }
        let start = read_starttime(host, pid).unwrap_or(0);
        if found.is_none_or(|(old, _)| pid < old) {
            found = Some((pid, start));
        }
    }
    found
}

fn cmdline_arg0(host: &Host, pid: u32) -> Option<String> {
    let raw = std::fs::read(host.proc_root.join(pid.to_string()).join("cmdline")).ok()?;
    let first = raw.split(|b| *b == 0).next()?;
    let s = String::from_utf8_lossy(first).trim().to_string();
    (!s.is_empty()).then_some(s)
}

/// Parses the first Uid from `/proc/<pid>/status`.
pub(crate) fn read_uid(host: &Host, pid: u32) -> Option<u32> {
    let text = std::fs::read_to_string(host.proc_root.join(pid.to_string()).join("status")).ok()?;
    text.lines()
        .find(|l| l.starts_with("Uid:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse().ok())
}

/// `starttime` (field 22) from `/proc/<pid>/stat`; comm may contain spaces.
pub(crate) fn read_starttime(host: &Host, pid: u32) -> Option<u64> {
    let text = std::fs::read_to_string(host.proc_root.join(pid.to_string()).join("stat")).ok()?;
    let rest = &text[text.rfind(')')? + 1..];
    rest.split_whitespace().nth(19)?.parse().ok()
}

/// Live `(tid, comm)` pairs of a process.
pub(crate) fn threads(host: &Host, pid: u32) -> Vec<(u32, String)> {
    let Ok(rd) = std::fs::read_dir(host.task_dir(pid)) else {
        return Vec::new();
    };
    rd.flatten()
        .filter_map(|e| {
            let tid = e.file_name().to_str()?.parse::<u32>().ok()?;
            let comm =
                std::fs::read_to_string(host.task_dir(pid).join(tid.to_string()).join("comm"))
                    .ok()?
                    .trim()
                    .to_string();
            (!comm.is_empty()).then_some((tid, comm))
        })
        .collect()
}

/// Current `Cpus_allowed_list` of one thread.
pub(crate) fn read_mask(host: &Host, pid: u32, tid: u32) -> Option<u64> {
    let text =
        std::fs::read_to_string(host.task_dir(pid).join(tid.to_string()).join("status")).ok()?;
    let list = text
        .lines()
        .find(|l| l.starts_with("Cpus_allowed_list:"))?
        .split_whitespace()
        .nth(1)?;
    parse_list(list)
}

/// `taskset -p <hex> <tid>`, followed by a read-back check.
pub(crate) fn set_mask(host: &Host, pid: u32, tid: u32, mask: u64) -> Result<(), String> {
    let bin = host
        .taskset
        .to_str()
        .ok_or_else(|| format!("invalid taskset path {:?}", host.taskset))?;
    let hex = format!("{mask:x}");
    let tid_s = tid.to_string();
    let out = platform::run(bin, &["-p", &hex, &tid_s], Duration::from_secs(1))
        .ok_or_else(|| format!("taskset spawn failed for tid {tid}"))?;
    if !out.success() {
        return Err(format!(
            "taskset {hex} {tid} failed: {}",
            out.stderr_lossy().trim()
        ));
    }
    match read_mask(host, pid, tid) {
        Some(live) if live == mask => Ok(()),
        Some(live) => Err(format!(
            "taskset read-back for tid {tid} is {live:#x}, wanted {mask:#x}"
        )),
        None => Err(format!("no Cpus_allowed_list for tid {tid}")),
    }
}

/// Parses `0-7`, `0-2,4-7`, `6`, ... into a bit mask.
pub(crate) fn parse_list(s: &str) -> Option<u64> {
    let mut mask = 0u64;
    for part in s.trim().split(',') {
        let mut range = part.split('-');
        let a = range.next()?.trim().parse::<u32>().ok()?;
        let b = match range.next() {
            Some(v) => v.trim().parse::<u32>().ok()?,
            None => a,
        };
        if a > 63 || b > 63 || a > b {
            return None;
        }
        for cpu in a..=b {
            mask |= 1u64 << cpu;
        }
    }
    Some(mask)
}

/// Inverse of [`parse_list`] for logs (e.g. `6-7`).
pub(crate) fn format_list(mask: u64) -> String {
    let mut out = Vec::new();
    let mut start: Option<u32> = None;
    let mut last = 0u32;
    for cpu in 0..64u32 {
        if mask & (1u64 << cpu) == 0 {
            continue;
        }
        match start {
            None => {
                start = Some(cpu);
                last = cpu;
            }
            Some(_) if cpu == last + 1 => last = cpu,
            Some(s) => {
                out.push(if s == last {
                    format!("{s}")
                } else {
                    format!("{s}-{last}")
                });
                start = Some(cpu);
                last = cpu;
            }
        }
    }
    if let Some(s) = start {
        out.push(if s == last {
            format!("{s}")
        } else {
            format!("{s}-{last}")
        });
    }
    out.join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cpu_lists() {
        assert_eq!(parse_list("0-7"), Some(0xFF));
        assert_eq!(parse_list("0-2,4-7"), Some(0xF7));
        assert_eq!(parse_list("6"), Some(0x40));
        assert_eq!(parse_list(" 6-7 "), Some(0xC0));
        assert_eq!(parse_list("8"), Some(1 << 8));
        assert_eq!(parse_list("7-6"), None);
        assert_eq!(parse_list(""), None);
    }

    #[test]
    fn formats_cpu_lists() {
        assert_eq!(format_list(0xFF), "0-7");
        assert_eq!(format_list(0xF7), "0-2,4-7");
        assert_eq!(format_list(0xC0), "6-7");
        assert_eq!(format_list(0x40), "6");
        assert_eq!(format_list(0), "");
    }

    #[test]
    fn starttime_is_field_22_after_comm() {
        let dir = crate::daemon::test_util::tmpdir("affinity-stat");
        let host = Host {
            proc_root: dir.clone(),
            taskset: dir.join("taskset"),
        };
        let pid_dir = dir.join("42");
        std::fs::create_dir_all(&pid_dir).unwrap();
        let mut stat = String::from("42 (a (weird) comm) S");
        for i in 0..25 {
            stat.push_str(&format!(" {i}"));
        }
        std::fs::write(pid_dir.join("stat"), stat).unwrap();
        // Token 19 after ')' is the 18th number in this synthetic line.
        assert_eq!(read_starttime(&host, 42), Some(18));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
