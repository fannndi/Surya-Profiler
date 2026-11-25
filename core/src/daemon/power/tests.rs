//! Power history unit tests (bucket math, store, retention, peaks).

use super::*;
use crate::engine::env::PowerReading;

fn tmp(tag: &str) -> std::path::PathBuf {
    crate::daemon::test_util::tmpdir(tag)
}

fn reading(ua: i64, uv: u32, temp: f32, pct: u8) -> PowerReading {
    PowerReading {
        pct: Some(pct),
        status: Some(if ua < 0 { "charging" } else { "discharging" }.into()),
        current_ua: Some(ua),
        voltage_uv: Some(uv),
        temp_c: Some(temp),
        ..Default::default()
    }
}

fn sample(t_ms: u64, r: PowerReading, screen_on: bool, pkg: &str, profile: &str) -> PowerSample {
    PowerSample::from_reading(
        t_ms,
        r,
        screen_on,
        Some(pkg.to_string()),
        Some(profile.to_string()),
    )
}

fn aligned(step: u64) -> u64 {
    let now = now_ms();
    (now - now % step).saturating_sub(20 * step)
}

#[test]
fn aggregates_minute_bucket_and_persists() {
    let dir = tmp("power-agg");
    let mut h = PowerHistory::load_with(&dir, 1_000, 3_600_000);
    let t0 = aligned(1_000);
    let a = sample(
        t0,
        reading(-1_000_000, 4_000_000, 30.0, 80),
        true,
        "app.a",
        "game",
    );
    let b = sample(
        t0 + 500,
        reading(-2_000_000, 4_200_000, 32.0, 79),
        true,
        "app.a",
        "game",
    );
    let c = sample(
        t0 + 1_000,
        reading(500_000, 4_100_000, 31.0, 79),
        true,
        "app.b",
        "balance",
    );
    assert!(h.record(&a, 500).is_some());
    assert!(h.record(&b, 500).is_some());
    // Crossing the bucket boundary closes the first bucket.
    assert!(h.record(&c, 500).is_some());
    assert_eq!(h.bucket_count(), 1);

    let snap = h.snapshot(600);
    assert_eq!(snap.window_s, 600);
    assert_eq!(snap.history.len(), 1);
    let b0 = &snap.history[0];
    assert_eq!(b0.t_ms, t0);
    assert_eq!(b0.avg_ua, -1_500_000);
    assert_eq!(b0.min_ua, -2_000_000);
    assert_eq!(b0.max_ua, -1_000_000);
    assert_eq!(b0.avg_uv, 4_100_000);
    assert_eq!(b0.avg_temp_c, Some(31.0));
    assert_eq!(b0.max_temp_c, Some(32.0));
    assert_eq!(b0.samples, 2);
    assert_eq!(b0.dur_ms, 1_000);
    assert!(b0.screen_on);
    assert_eq!(b0.pkg.as_deref(), Some("app.a"));
    assert_eq!(b0.profile.as_deref(), Some("game"));
    assert_eq!(h.peaks().charge.as_ref().map(|p| p.ua), Some(-2_000_000));
    assert!(h.peaks().discharge.is_none());
    assert!(dir.join(super::model::HISTORY_FILE).exists());
    assert!(dir.join(super::model::PEAKS_FILE).exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_power_updates_live_but_not_history() {
    let dir = tmp("power-missing");
    let mut h = PowerHistory::load_with(&dir, 1_000, 3_600_000);
    let r = PowerReading {
        pct: Some(42),
        status: Some("discharging".into()),
        ..Default::default()
    };
    let s = PowerSample::from_reading(
        aligned(1_000),
        r,
        true,
        Some("app".into()),
        Some("balance".into()),
    );
    assert!(h.record(&s, 1_000).is_none());
    assert_eq!(h.bucket_count(), 0);
    let snap = h.snapshot(600);
    assert_eq!(snap.live.as_ref().and_then(|l| l.reading.pct), Some(42));
    assert!(snap.current_bucket.is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn screen_off_majority_drops_package() {
    let dir = tmp("power-screen");
    let mut h = PowerHistory::load_with(&dir, 1_000, 3_600_000);
    let t0 = aligned(1_000);
    let r = reading(300_000, 4_000_000, 30.0, 70);
    h.record(&sample(t0, r.clone(), true, "app.a", "game"), 1_000);
    h.record(&sample(t0 + 100, r.clone(), false, "app.a", "sleep"), 1_000);
    h.record(&sample(t0 + 200, r, false, "app.a", "sleep"), 1_000);
    // Next bucket closes the first.
    h.record(
        &sample(
            t0 + 1_000,
            reading(300_000, 4_000_000, 30.0, 70),
            false,
            "app.b",
            "sleep",
        ),
        1_000,
    );
    let b = &h.snapshot(600).history[0];
    assert!(!b.screen_on);
    assert_eq!(b.pkg, None, "screen-off buckets never attribute an app");
    assert_eq!(b.profile.as_deref(), Some("sleep"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn history_survives_reload_and_flush() {
    let dir = tmp("power-reload");
    let t0 = aligned(1_000);
    {
        let mut h = PowerHistory::load_with(&dir, 1_000, 3_600_000);
        h.record(
            &sample(
                t0,
                reading(-800_000, 4_000_000, 30.0, 80),
                true,
                "app",
                "game",
            ),
            500,
        );
        h.record(
            &sample(
                t0 + 1_000,
                reading(-700_000, 4_100_000, 31.0, 81),
                true,
                "app",
                "game",
            ),
            500,
        );
        h.flush();
        assert_eq!(h.bucket_count(), 2);
    }
    let h = PowerHistory::load_with(&dir, 1_000, 3_600_000);
    assert_eq!(h.bucket_count(), 2);
    assert_eq!(h.peaks().charge.as_ref().map(|p| p.ua), Some(-800_000));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn corrupt_lines_are_skipped() {
    let dir = tmp("power-corrupt");
    let path = dir.join(super::model::HISTORY_FILE);
    let good = PowerBucket {
        t_ms: now_ms().saturating_sub(1_000),
        pct: Some(70),
        avg_ua: -500_000,
        min_ua: -500_000,
        max_ua: -500_000,
        avg_uv: 4_000_000,
        avg_temp_c: Some(31.0),
        max_temp_c: Some(31.0),
        screen_on: true,
        dur_ms: 1_000,
        samples: 1,
        pkg: Some("app".into()),
        profile: Some("game".into()),
    };
    let mut body = serde_json::to_string(&good).unwrap();
    body.push('\n');
    body.push_str("{broken\n");
    std::fs::write(&path, body).unwrap();
    let h = PowerHistory::load_with(&dir, 1_000, 3_600_000);
    assert_eq!(h.bucket_count(), 1, "valid line survives a corrupt sibling");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn old_rows_are_trimmed_on_load() {
    let dir = tmp("power-trim");
    let now = now_ms();
    let old = PowerBucket {
        t_ms: now.saturating_sub(10_000_000),
        pct: Some(50),
        avg_ua: 100,
        min_ua: 100,
        max_ua: 100,
        avg_uv: 4_000_000,
        avg_temp_c: None,
        max_temp_c: None,
        screen_on: true,
        dur_ms: 1_000,
        samples: 1,
        pkg: None,
        profile: None,
    };
    let fresh = PowerBucket {
        t_ms: now.saturating_sub(1_000),
        ..old.clone()
    };
    let mut body = serde_json::to_string(&old).unwrap();
    body.push('\n');
    body.push_str(&serde_json::to_string(&fresh).unwrap());
    body.push('\n');
    std::fs::write(dir.join(super::model::HISTORY_FILE), body).unwrap();
    let h = PowerHistory::load_with(&dir, 1_000, 60_000);
    assert_eq!(
        h.bucket_count(),
        1,
        "retention 60 s keeps only the fresh row"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn clear_drops_history_peaks_and_files() {
    let dir = tmp("power-clear");
    let mut h = PowerHistory::load_with(&dir, 1_000, 3_600_000);
    let t0 = aligned(1_000);
    h.record(
        &sample(
            t0,
            reading(-1_500_000, 4_000_000, 35.0, 60),
            true,
            "app",
            "game",
        ),
        500,
    );
    h.flush();
    assert_eq!(h.bucket_count(), 1);
    assert!(h.peaks().charge.is_some());
    h.clear();
    assert_eq!(h.bucket_count(), 0);
    assert!(h.peaks().charge.is_none());
    assert!(!dir.join(super::model::HISTORY_FILE).exists());
    assert!(!dir.join(super::model::PEAKS_FILE).exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn window_clamps_and_filters() {
    let dir = tmp("power-window");
    let mut h = PowerHistory::load_with(&dir, 1_000, 3_600_000);
    let t0 = aligned(1_000);
    h.record(
        &sample(
            t0,
            reading(200_000, 4_000_000, 30.0, 70),
            true,
            "app",
            "balance",
        ),
        500,
    );
    h.flush();
    let snap = h.snapshot(1);
    assert_eq!(snap.window_s, super::model::MIN_WINDOW_S);
    let snap = h.snapshot(u64::MAX);
    assert_eq!(snap.window_s, super::model::MAX_WINDOW_S);
    let _ = std::fs::remove_dir_all(&dir);
}
