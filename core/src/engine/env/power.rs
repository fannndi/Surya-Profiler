//! Read-only battery/charger probe for the Power usage screen.
//!
//! The daemon's fast power sampler and the 30 s env sampler share this code:
//! every field is optional, a missing node is simply absent, and nothing here
//! ever writes sysfs/proc. Sign convention is the device's, verified
//! 2026-10-09/22: negative `current_now` = charging, positive = discharging.

use super::parsers::{first_i64, first_u32, parse_temp_tenths, uah_to_mah};
use super::sysfs::read_trim;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Charger input supplies in preference order (surya / sm6150 names).
/// `battery`/`bms` are the fuel gauge, not an input rail.
const INPUT_SUPPLIES: &[&str] = &["usb", "pc_port", "main", "dc", "wireless"];

/// One charger input rail reported by the kernel (read-only).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct InputSupply {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub online: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voltage_uv: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_ua: Option<i64>,
    /// USB_CDP / USB_PD / Main / … when the rail reports it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

#[derive(Default)]
struct Uevent {
    online: Option<bool>,
    voltage_uv: Option<u32>,
    current_ua: Option<i64>,
    kind: Option<String>,
}

/// One battery/charger reading. No timestamp: the daemon stamps samples.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PowerReading {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pct: Option<u8>,
    /// `charging` | `discharging` | `full` | `not_charging` | `unknown`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Negative = charging, positive = discharging (device sign).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_ua: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voltage_uv: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temp_c: Option<f32>,
    /// Remaining charge from `charge_counter` (µAh); quantized by the FG.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charge_counter_uah: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charge_full_mah: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cycle_count: Option<u32>,
    /// MIUI R&D bypass node (read-only here; the bridge owns the write).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_suspend: Option<bool>,
    /// Framework charge guard (`battery_charging_enabled`), read-only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charge_enabled: Option<bool>,
    /// Charger input rail, when the kernel exposes one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<InputSupply>,
}

/// Reads the battery + charger nodes under a configurable sysfs root.
pub struct PowerProbe {
    root: PathBuf,
}

impl PowerProbe {
    pub fn new(root: &Path) -> Self {
        PowerProbe {
            root: root.to_path_buf(),
        }
    }

    /// One read-only sample. A missing/denied node stays `None`.
    pub fn read(&self) -> PowerReading {
        let battery = "battery";
        PowerReading {
            pct: self.read_parse(battery, "capacity", super::parsers::parse_pct),
            status: self.read_parse(battery, "status", parse_battery_status),
            current_ua: self.read_parse(battery, "current_now", first_i64),
            voltage_uv: self.read_parse(battery, "voltage_now", first_u32),
            temp_c: self.read_parse(battery, "temp", parse_temp_tenths),
            charge_counter_uah: self
                .read_parse(battery, "charge_counter", first_i64)
                .filter(|v| *v >= 0),
            charge_full_mah: self.read_parse(battery, "charge_full", uah_to_mah),
            cycle_count: self.read_parse(battery, "cycle_count", first_u32),
            input_suspend: self.read_parse(battery, "input_suspend", parse_bool01),
            charge_enabled: self.read_parse(battery, "battery_charging_enabled", parse_bool01),
            input: self.read_input(),
        }
    }

    fn read_parse<T>(&self, supply: &str, node: &str, f: fn(&str) -> Option<T>) -> Option<T> {
        let path = self
            .root
            .join("sys/class/power_supply")
            .join(supply)
            .join(node);
        read_trim(&path).and_then(|s| f(&s))
    }

    /// Best input rail. Priority: online **with** a measured/settled value,
    /// then any rail with a value (some rails have no `online` node), then an
    /// online-only rail. surya's `usb` exposes values only through `uevent`,
    /// while `main` exposes `input_*_settled` files — both are supported.
    fn read_input(&self) -> Option<InputSupply> {
        let mut best_values = None;
        let mut online_only = None;
        for name in INPUT_SUPPLIES {
            let ue = self.read_uevent(name);
            let online = self.read_parse(name, "online", parse_bool01).or(ue.online);
            let voltage_uv = self
                .read_parse(name, "voltage_now", first_u32)
                .or_else(|| self.read_parse(name, "input_voltage_settled", first_u32))
                .or(ue.voltage_uv);
            let current_ua = self
                .read_parse(name, "current_now", first_i64)
                .or_else(|| self.read_parse(name, "input_current_now", first_i64))
                .or_else(|| self.read_parse(name, "input_current_settled", first_i64))
                .or(ue.current_ua);
            let kind = self
                .read_parse(name, "real_type", parse_str)
                .or_else(|| self.read_parse(name, "type", parse_str))
                .or(ue.kind);
            if online.is_none() && voltage_uv.is_none() && current_ua.is_none() {
                continue;
            }
            let supply = InputSupply {
                name: (*name).to_string(),
                online,
                voltage_uv,
                current_ua,
                kind,
            };
            let has_value = supply.voltage_uv.is_some() || supply.current_ua.is_some();
            if online == Some(true) && has_value {
                return Some(supply);
            }
            if online == Some(true) {
                online_only.get_or_insert(supply);
            } else if has_value {
                best_values.get_or_insert(supply);
            }
        }
        best_values.or(online_only)
    }

    /// `uevent` is the only place some Qualcomm rails expose VBUS/VBUS
    /// current (surya's `usb` has no `voltage_now`/`current_now` files).
    fn read_uevent(&self, supply: &str) -> Uevent {
        let path = self
            .root
            .join("sys/class/power_supply")
            .join(supply)
            .join("uevent");
        let Some(text) = read_trim(&path) else {
            return Uevent::default();
        };
        let mut out = Uevent::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key {
                "POWER_SUPPLY_ONLINE" => out.online = parse_bool01(value),
                "POWER_SUPPLY_VOLTAGE_NOW" => out.voltage_uv = first_u32(value),
                "POWER_SUPPLY_INPUT_CURRENT_NOW" => out.current_ua = first_i64(value),
                "POWER_SUPPLY_CURRENT_NOW" if out.current_ua.is_none() => {
                    out.current_ua = first_i64(value)
                }
                "POWER_SUPPLY_REAL_TYPE" => out.kind = parse_str(value),
                "POWER_SUPPLY_TYPE" if out.kind.is_none() => out.kind = parse_str(value),
                _ => {}
            }
        }
        out
    }
}

/// Non-empty, non-placeholder text (some rails report `Unknown`/`-22`).
fn parse_str(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.is_empty() || s == "Unknown" || s == "-22" {
        None
    } else {
        Some(s.to_string())
    }
}

/// `Charging` -> `charging`, `Not charging` -> `not_charging`; unknown text
/// remains absent (the daemon never guesses a state).
pub fn parse_battery_status(raw: &str) -> Option<String> {
    let s = match raw.trim() {
        "Charging" => "charging",
        "Discharging" => "discharging",
        "Full" => "full",
        "Not charging" => "not_charging",
        "Unknown" => "unknown",
        _ => return None,
    };
    Some(s.to_string())
}

/// Kernel 0/1 flag -> bool.
pub fn parse_bool01(raw: &str) -> Option<bool> {
    match raw.trim() {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    }
}
