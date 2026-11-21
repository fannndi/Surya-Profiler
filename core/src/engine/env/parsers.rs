//! Pure telemetry parsers.

pub fn parse_pct(raw: &str) -> Option<u8> {
    raw.trim()
        .parse::<i64>()
        .ok()
        .map(|v| v.clamp(0, 100) as u8)
}

/// power_supply battery temp: tenths of °C ("320" -> 32.0). <= 0 -> None.
pub fn parse_temp_tenths(raw: &str) -> Option<f32> {
    let v = raw.trim().parse::<i64>().ok()?;
    if v <= 0 {
        return None;
    }
    Some(v as f32 / 10.0)
}

/// thermal_zone temp: millidegrees ("38800" -> 38.8). <= 0 -> None
/// (level-indicator zones use negative numbers; we never sample those).
pub fn parse_temp_milli(raw: &str) -> Option<f32> {
    let v = raw.trim().parse::<i64>().ok()?;
    if v <= 0 {
        return None;
    }
    Some(v as f32 / 1000.0)
}

/// power_supply status -> charging? "Charging"/"Full" true, the rest false;
/// unknown strings -> None (absent is honest).
pub fn parse_charging(raw: &str) -> Option<bool> {
    match raw.trim() {
        "Charging" | "Full" => Some(true),
        "Discharging" | "Not charging" => Some(false),
        _ => None,
    }
}

/// "3 %" or "3" -> Some(3), clamped to 0..=100.
pub fn parse_gpu_busy(raw: &str) -> Option<u8> {
    let first = raw.split_whitespace().next()?;
    first.parse::<i64>().ok().map(|v| v.clamp(0, 100) as u8)
}

/// First whitespace token as signed integer (thermal sconfig/temp_state).
pub fn first_i32(raw: &str) -> Option<i32> {
    raw.split_whitespace().next()?.parse::<i32>().ok()
}

/// First whitespace token as signed integer (battery current in µA).
pub fn first_i64(raw: &str) -> Option<i64> {
    raw.split_whitespace().next()?.parse::<i64>().ok()
}

/// First whitespace token as u32.
pub fn first_u32(raw: &str) -> Option<u32> {
    raw.split_whitespace().next()?.parse::<u32>().ok()
}

/// First whitespace token as u64.
pub fn first_u64(raw: &str) -> Option<u64> {
    raw.split_whitespace().next()?.parse::<u64>().ok()
}

/// `charge_full` reports µAh; diagnostics speak mAh ("5008000" -> 5008).
/// Non-positive values (broken fuel gauge) stay absent.
pub fn uah_to_mah(raw: &str) -> Option<u32> {
    let v = first_i64(raw)?;
    if v <= 0 {
        None
    } else {
        Some((v / 1000) as u32)
    }
}

/// `cpuinfo_cur_freq` reports kHz; diagnostics speak MHz ("1804800" -> 1804).
pub fn khz_to_mhz(raw: &str) -> Option<u32> {
    first_i64(raw).map(|v| (v / 1000) as u32)
}

/// kgsl `gpuclk` reports Hz; diagnostics speak MHz ("430000000" -> 430).
pub fn hz_to_mhz(raw: &str) -> Option<u32> {
    first_i64(raw).map(|v| (v / 1_000_000) as u32)
}

/// DRM `measured_fps` -> frames per second:
/// "fps: 75.1 duration:1000000 frame_count:97" -> Some(75.1).
/// Screen off reports fps: 0.0 -> Some(0.0) (honest zero, not absent).
pub fn parse_measured_fps(raw: &str) -> Option<f32> {
    let rest = raw.trim().strip_prefix("fps:")?;
    rest.split_whitespace().next()?.parse::<f32>().ok()
}
