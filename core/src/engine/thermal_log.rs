//! Read-only parser for MIUI's `mi_thermald` event dumps.
//!
//! `mi_thermald` writes two rolling files in `/data/vendor/thermal/`:
//!
//! ```text
//! 11-19 20:22:45[MONITOR-BAT][quiet-therm-adc 39545][battery 602]
//! 11-19 20:23:14[BAL-SS-CPU6][nvm-therm-adc 51200][cpu6 2169600]
//! ```
//!
//! The timestamps have no year (MIUI's own choice); the UI shows them as-is.
//! Parsing is intentionally tolerant: a malformed line is skipped, never a
//! fatal diagnostic.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// One throttling/telemetry event from `thermal.dump` or `last_thermal.dump`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThermalEvent {
    /// `MM-DD HH:MM:SS` as printed by mi_thermald.
    pub time: String,
    /// Algorithm block name, e.g. `MONITOR-BAT`, `BAL-SS-CPU6`.
    pub algo: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sensor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sensor_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_value: Option<String>,
    /// Original line, for the share sheet / advanced debugging.
    pub raw: String,
}

/// Parses one `[ALGO][sensor value][device value]` line.
pub fn parse_line(line: &str) -> Option<ThermalEvent> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let open = line.find('[')?;
    let time = line[..open].trim();
    if time.is_empty() {
        return None;
    }
    let body = line[open..].trim().strip_prefix('[')?;
    let mut parts = body.split("][");
    let algo = parts.next()?.trim().to_string();
    if algo.is_empty() {
        return None;
    }
    let sensor = parts.next().and_then(split_kv);
    let device = parts.next().and_then(split_kv);
    Some(ThermalEvent {
        time: time.to_string(),
        algo,
        sensor: sensor.as_ref().map(|(n, _)| n.clone()),
        sensor_value: sensor.map(|(_, v)| v),
        device: device.as_ref().map(|(n, _)| n.clone()),
        device_value: device.map(|(_, v)| v),
        raw: line.to_string(),
    })
}

fn split_kv(part: &str) -> Option<(String, String)> {
    let part = part.trim().trim_end_matches(']').trim();
    if part.is_empty() {
        return None;
    }
    match part.split_once(' ') {
        Some((name, value)) => {
            let name = name.trim();
            let value = value.trim();
            if name.is_empty() {
                None
            } else {
                Some((name.to_string(), value.to_string()))
            }
        }
        None => Some((part.to_string(), String::new())),
    }
}

fn dump_paths(root: &Path) -> [PathBuf; 2] {
    [
        root.join("data/vendor/thermal/last_thermal.dump"),
        root.join("data/vendor/thermal/thermal.dump"),
    ]
}

/// Reads the two rolling dumps (older first) and returns the newest `limit`
/// parsed events. Missing files yield an empty list.
pub fn load_recent(root: &Path, limit: usize) -> Vec<ThermalEvent> {
    if limit == 0 {
        return Vec::new();
    }
    let mut events = Vec::new();
    for path in dump_paths(root) {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for line in text.lines() {
            if let Some(ev) = parse_line(line) {
                events.push(ev);
            }
        }
    }
    if events.len() > limit {
        events.split_off(events.len() - limit)
    } else {
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "surya-thermal-log-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn parses_battery_event() {
        let ev =
            parse_line("11-19 20:22:45[MONITOR-BAT][quiet-therm-adc 39545][battery 602]").unwrap();
        assert_eq!(ev.time, "11-19 20:22:45");
        assert_eq!(ev.algo, "MONITOR-BAT");
        assert_eq!(ev.sensor.as_deref(), Some("quiet-therm-adc"));
        assert_eq!(ev.sensor_value.as_deref(), Some("39545"));
        assert_eq!(ev.device.as_deref(), Some("battery"));
        assert_eq!(ev.device_value.as_deref(), Some("602"));
    }

    #[test]
    fn parses_custom_ss_and_plus_targets() {
        let ev =
            parse_line("11-19 20:23:14[BAL-SS-CPU6][nvm-therm-adc 51200][cpu6 2169600]").unwrap();
        assert_eq!(ev.algo, "BAL-SS-CPU6");
        assert_eq!(ev.device_value.as_deref(), Some("2169600"));

        let ev =
            parse_line("11-19 20:22:45[MONITOR-BCL][bat_soc 56][hotplug_cpu0+hotplug_cpu1 1+1]")
                .unwrap();
        assert_eq!(ev.device.as_deref(), Some("hotplug_cpu0+hotplug_cpu1"));
        assert_eq!(ev.device_value.as_deref(), Some("1+1"));
    }

    #[test]
    fn skips_malformed_lines() {
        assert!(parse_line("").is_none());
        assert!(parse_line("garbage").is_none());
        assert!(parse_line("[MONITOR-BAT][battery 1]").is_none());
    }

    #[test]
    fn load_recent_merges_old_then_new_and_caps() {
        let root = tmp("merge");
        fs::create_dir_all(root.join("data/vendor/thermal")).unwrap();
        fs::write(
            root.join("data/vendor/thermal/last_thermal.dump"),
            "11-19 20:00:00[MONITOR-BAT][quiet-therm-adc 38000][battery 201]\n",
        )
        .unwrap();
        fs::write(
            root.join("data/vendor/thermal/thermal.dump"),
            "11-19 20:01:00[MONITOR-BAT][quiet-therm-adc 39000][battery 602]\n\
             11-19 20:02:00[MONITOR-BAT][quiet-therm-adc 40000][battery 907]\n",
        )
        .unwrap();
        let all = load_recent(&root, 10);
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].time, "11-19 20:00:00");
        assert_eq!(all[2].time, "11-19 20:02:00");
        let last = load_recent(&root, 2);
        assert_eq!(last.len(), 2);
        assert_eq!(last[0].time, "11-19 20:01:00");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_files_are_empty() {
        let root = tmp("missing");
        assert!(load_recent(&root, 5).is_empty());
        let _ = fs::remove_dir_all(&root);
    }
}
