//! Power history unit tests (bucket math, store, retention, peaks).

use super::aggregate::Aggregator;
use super::model::PowerPeaks;
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

#[allow(clippy::too_many_arguments)] // Compact fixture for persisted-row cases.
fn row(
    t_ms: u64,
    dur_ms: u64,
    avg_ua: i64,
    avg_uv: u32,
    avg_temp_c: Option<f32>,
    screen_on: bool,
    pkg: Option<&str>,
    profile: Option<&str>,
) -> PowerBucket {
    PowerBucket {
        t_ms,
        pct: Some(70),
        avg_ua,
        min_ua: avg_ua,
        max_ua: avg_ua,
        avg_uv,
        avg_temp_c,
        max_temp_c: avg_temp_c,
        screen_on,
        dur_ms,
        samples: 1,
        pkg: pkg.map(str::to_string),
        profile: profile.map(str::to_string),
        start_ms: None,
        end_ms: None,
        energy_mwh: None,
        segment_id: None,
    }
}

#[test]
fn optional_bucket_fields_round_trip_and_legacy_rows_default_to_none() {
    let dir = tmp("power-optional-fields");
    let mut a = Aggregator::new(1_000);
    a.add(
        &reading(-1_000_000, 4_000_000, 30.0, 80),
        true,
        Some("app"),
        Some("game"),
        60_000,
    );
    let b = a.to_bucket();
    assert_eq!(b.start_ms, Some(1_000));
    assert_eq!(b.end_ms, Some(61_000));
    assert!((b.energy_mwh.unwrap() + 66.666_666_7).abs() < 0.000_001);
    assert!(b.segment_id.as_deref().is_some_and(|id| !id.is_empty()));
    store::append(&dir, &b).unwrap();
    let loaded = store::load_history(&dir);
    assert_eq!(loaded, vec![b.clone()]);

    let mut legacy = serde_json::to_value(&b).unwrap();
    for key in ["start_ms", "end_ms", "energy_mwh", "segment_id"] {
        legacy.as_object_mut().unwrap().remove(key);
    }
    std::fs::write(
        dir.join(super::model::HISTORY_FILE),
        serde_json::to_string(&legacy).unwrap() + "\n",
    )
    .unwrap();
    let loaded = store::load_history(&dir);
    assert_eq!(loaded[0].start_ms, None);
    assert_eq!(loaded[0].end_ms, None);
    assert_eq!(loaded[0].energy_mwh, None);
    assert_eq!(loaded[0].segment_id, None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_history_merges_same_minute_and_keeps_unknown_duration() {
    let dir = tmp("power-merge-load");
    let t0 = aligned(1_000);
    let attributed = PowerBucket {
        min_ua: -2_000_000,
        max_ua: -500_000,
        samples: 18,
        ..row(
            t0,
            54_000,
            -1_000_000,
            4_000_000,
            Some(30.0),
            true,
            Some("app.a"),
            Some("game"),
        )
    };
    let unknown = PowerBucket {
        min_ua: 2_900_000,
        max_ua: 3_100_000,
        ..row(
            t0,
            3_000,
            3_000_000,
            4_200_000,
            Some(40.0),
            false,
            None,
            None,
        )
    };
    let body = format!(
        "{}\n{}\n",
        serde_json::to_string(&attributed).unwrap(),
        serde_json::to_string(&unknown).unwrap()
    );
    std::fs::write(dir.join(super::model::HISTORY_FILE), body).unwrap();

    let loaded = store::load_history(&dir);
    assert_eq!(loaded.len(), 1);
    let b = &loaded[0];
    assert_eq!(b.dur_ms, 57_000);
    assert_eq!(b.samples, 19);
    assert_eq!(b.avg_ua, -789_474);
    assert_eq!(b.avg_uv, 4_010_526);
    assert_eq!(b.avg_temp_c, Some(30.526_316));
    assert_eq!(b.min_ua, -2_000_000);
    assert_eq!(b.max_ua, 3_100_000);
    assert!(b.screen_on);
    assert_eq!(b.pkg.as_deref(), Some("app.a"));
    assert_eq!(b.profile.as_deref(), Some("game"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_history_deduplicates_the_same_segment_replay() {
    let dir = tmp("power-duplicate-segment");
    let t0 = aligned(60_000);
    let first = PowerBucket {
        segment_id: Some("same-run-segment".into()),
        ..row(
            t0,
            12_000,
            -1_000_000,
            4_000_000,
            Some(30.0),
            true,
            Some("app.a"),
            Some("game"),
        )
    };
    let replay = PowerBucket {
        segment_id: Some("same-run-segment".into()),
        ..row(
            t0,
            12_000,
            -900_000,
            4_100_000,
            Some(31.0),
            true,
            Some("app.a"),
            Some("game"),
        )
    };
    let body = format!(
        "{}\n{}\n",
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&replay).unwrap()
    );
    std::fs::write(dir.join(super::model::HISTORY_FILE), body).unwrap();

    let loaded = store::load_history(&dir);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].dur_ms, 12_000);
    assert_eq!(loaded[0].avg_ua, -900_000);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn unknown_attribution_wins_when_it_has_more_duration() {
    let dir = tmp("power-merge-unknown-majority");
    let t0 = aligned(1_000);
    let attributed = row(
        t0,
        3_000,
        100_000,
        4_000_000,
        Some(30.0),
        true,
        Some("app.a"),
        Some("game"),
    );
    let unknown = row(t0, 54_000, 300_000, 4_000_000, Some(30.0), true, None, None);
    let body = format!(
        "{}\n{}\n",
        serde_json::to_string(&attributed).unwrap(),
        serde_json::to_string(&unknown).unwrap()
    );
    std::fs::write(dir.join(super::model::HISTORY_FILE), body).unwrap();
    let b = store::load_history(&dir).remove(0);
    assert_eq!(b.dur_ms, 57_000);
    assert!(b.screen_on);
    assert_eq!(b.pkg, None);
    assert_eq!(b.profile, None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn aggregator_unknown_attribution_can_win() {
    let mut a = Aggregator::new(1_000);
    a.add(
        &reading(100_000, 4_000_000, 30.0, 80),
        true,
        Some("app"),
        Some("game"),
        3_000,
    );
    a.add(
        &reading(300_000, 4_000_000, 30.0, 80),
        true,
        None,
        None,
        54_000,
    );
    let b = a.to_bucket();
    assert_eq!(b.pkg, None);
    assert_eq!(b.profile, None);
}

#[test]
fn close_merges_same_minute_across_restart_without_double_counting() {
    let dir = tmp("power-merge-close");
    let t0 = aligned(60_000);
    {
        let mut h = PowerHistory::load_with(&dir, 60_000, 3_600_000);
        h.record(
            &sample(
                t0,
                reading(-1_000_000, 4_000_000, 30.0, 80),
                true,
                "app.a",
                "game",
            ),
            12_000,
        );
        h.flush();
    }
    {
        let mut h = PowerHistory::load_with(&dir, 60_000, 3_600_000);
        h.record(
            &sample(
                t0,
                reading(3_000_000, 4_200_000, 40.0, 79),
                false,
                "app.b",
                "sleep",
            ),
            48_000,
        );
        h.flush();
        let b = &h.snapshot(3600).history[0];
        assert_eq!(b.dur_ms, 60_000);
        assert_eq!(b.avg_ua, 2_200_000);
        assert_eq!(b.avg_uv, 4_160_000);
        assert_eq!(b.avg_temp_c, Some(38.0));
        assert!(!b.screen_on);
        assert_eq!(b.pkg, None);
        assert_eq!(b.profile.as_deref(), Some("sleep"));
    }
    let text = std::fs::read_to_string(dir.join(super::model::HISTORY_FILE)).unwrap();
    assert_eq!(
        text.lines().count(),
        1,
        "same-minute close rewrites one merged row"
    );
    let loaded = store::load_history(&dir);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].dur_ms, 60_000);
    assert_eq!(loaded[0].avg_ua, 2_200_000);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sampler_wake_recalculates_cadence_and_stop_joins() {
    use std::sync::mpsc;
    let screen = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();
    let mut sampler = spawn(tx, screen.clone());
    let recv_tick = || match rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap() {
        super::super::Msg::Power(tick) => tick,
        _ => panic!("expected power tick"),
    };
    let first = recv_tick();
    assert!(first.captured_at_ms > 0);
    assert_eq!(first.elapsed_ms, 0);
    assert_eq!(first.interval_ms, sample_off_ms());

    screen.store(true, Ordering::Relaxed);
    sampler.wake();
    let second = rx.recv_timeout(std::time::Duration::from_secs(1)).unwrap();
    let second = match second {
        super::super::Msg::Power(tick) => tick,
        _ => panic!("expected power tick"),
    };
    assert_eq!(
        second.interval_ms,
        sample_ms(),
        "wake must recalculate cadence"
    );

    sampler.stop();
    while rx.try_recv().is_ok() {}
    assert!(matches!(
        rx.recv_timeout(std::time::Duration::from_secs(1)),
        Err(mpsc::RecvTimeoutError::Disconnected)
    ));
}

#[test]
fn record_ignores_elapsed_gaps_larger_than_two_buckets() {
    let dir = tmp("power-suspend-gap");
    let mut h = PowerHistory::load_with(&dir, 60_000, 3_600_000);
    let t0 = aligned(60_000);
    let s = sample(
        t0,
        reading(-1_000_000, 4_000_000, 30.0, 80),
        true,
        "app",
        "game",
    );
    assert!(
        h.record(&s, 130_000).is_none(),
        "a suspend gap must not create a bucket"
    );
    assert!(h.snapshot(3600).live.is_some());
    assert!(h.snapshot(3600).current_bucket.is_none());
    assert!(h.record(&s, 3_000).is_some());
    assert!(h.snapshot(3600).current_bucket.is_some());
    assert_eq!(h.snapshot(3600).current_bucket.unwrap().dur_ms, 3_000);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn aggregator_weights_mixed_cadence_by_elapsed_interval() {
    let mut a = Aggregator::new(1_000);
    assert!(a.add(
        &reading(-1_000_000, 4_000_000, 20.0, 80),
        true,
        Some("app.fast"),
        Some("game"),
        3_000,
    ));
    assert!(a.add(
        &reading(3_000_000, 4_200_000, 40.0, 79),
        true,
        Some("app.slow"),
        Some("balance"),
        30_000,
    ));
    let b = a.to_bucket();
    assert_eq!(b.dur_ms, 33_000);
    assert_eq!(b.samples, 2);
    assert_eq!(b.avg_ua, 2_636_364);
    assert_eq!(b.avg_uv, 4_181_818);
    assert!((b.avg_temp_c.unwrap() - 38.181_818).abs() < 0.000_01);
    assert_eq!(b.pkg.as_deref(), Some("app.slow"));
    assert_eq!(b.profile.as_deref(), Some("balance"));
}

#[test]
fn zero_elapsed_first_reading_is_kept_for_extrema_without_a_bucket() {
    let dir = tmp("power-zero-first");
    let mut h = PowerHistory::load_with(&dir, 1_000, 3_600_000);
    let t0 = aligned(1_000);
    let first = sample(
        t0,
        reading(-1_000_000, 4_000_000, 30.0, 80),
        true,
        "app",
        "game",
    );
    assert!(h.record(&first, 0).is_none());
    assert!(h.snapshot(3600).current_bucket.is_none());

    let next = sample(
        t0 + 500,
        reading(500_000, 4_100_000, 31.0, 79),
        true,
        "app",
        "game",
    );
    let current = h.record(&next, 500).unwrap();
    assert_eq!(current.dur_ms, 500);
    assert_eq!(current.samples, 2);
    assert_eq!(current.min_ua, -1_000_000);
    assert_eq!(current.max_ua, 500_000);
    assert_eq!(current.avg_ua, 500_000);

    h.flush();
    assert_eq!(h.bucket_count(), 1);
    assert_eq!(h.peaks().charge.as_ref().map(|p| p.ua), Some(-1_000_000));
    assert_eq!(h.peaks().discharge.as_ref().map(|p| p.ua), Some(500_000));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn zero_duration_close_records_peaks_but_no_history_row() {
    let dir = tmp("power-zero-close");
    let mut h = PowerHistory::load_with(&dir, 1_000, 3_600_000);
    let s = sample(
        aligned(1_000),
        reading(-2_000_000, 4_000_000, 30.0, 80),
        true,
        "app",
        "game",
    );
    assert!(h.record(&s, 0).is_none());
    h.flush();
    assert_eq!(h.bucket_count(), 0);
    assert_eq!(h.peaks().charge.as_ref().map(|p| p.ua), Some(-2_000_000));
    assert!(!dir.join(super::model::HISTORY_FILE).exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn zero_elapsed_reading_updates_extrema_without_weighting_averages() {
    let mut a = Aggregator::new(1_000);
    a.add(
        &reading(-1_000_000, 4_000_000, 30.0, 80),
        true,
        Some("app"),
        Some("game"),
        3_000,
    );
    a.add(
        &reading(30_000_000, 4_200_000, 45.0, 79),
        true,
        Some("other"),
        Some("sleep"),
        0,
    );
    let b = a.to_bucket();
    assert_eq!(b.dur_ms, 3_000);
    assert_eq!(b.samples, 2);
    assert_eq!(b.avg_ua, -1_000_000);
    assert_eq!(b.avg_uv, 4_000_000);
    assert_eq!(b.avg_temp_c, Some(30.0));
    assert_eq!(b.max_temp_c, Some(45.0));
    assert_eq!(b.max_ua, 30_000_000);
    assert_eq!(b.pkg.as_deref(), Some("app"));
    assert_eq!(b.profile.as_deref(), Some("game"));
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
        start_ms: None,
        end_ms: None,
        energy_mwh: None,
        segment_id: None,
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
        start_ms: None,
        end_ms: None,
        energy_mwh: None,
        segment_id: None,
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
fn store_reports_write_and_clear_failures() {
    let dir = tmp("power-store-errors");
    let b = row(
        1_000,
        1_000,
        -1_000_000,
        4_000_000,
        Some(30.0),
        true,
        Some("app"),
        Some("game"),
    );
    std::fs::create_dir(dir.join(super::model::HISTORY_FILE)).unwrap();
    assert!(store::append(&dir, &b).is_err());

    std::fs::remove_dir(dir.join(super::model::HISTORY_FILE)).unwrap();
    std::fs::create_dir(dir.join("power-history.jsonl.tmp")).unwrap();
    assert!(store::rewrite(&dir, &[b.clone()].into()).is_err());
    std::fs::create_dir(dir.join("power-peaks.json.tmp")).unwrap();
    assert!(store::save_peaks(&dir, &PowerPeaks::default()).is_err());
    assert!(
        store::clear(&dir).is_err(),
        "a leftover tmp directory is not cleared"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn clear_removes_tmp_files_before_reporting_success() {
    let dir = tmp("power-clear-tmp");
    for name in [
        super::model::HISTORY_FILE,
        super::model::PEAKS_FILE,
        "power-history.jsonl.tmp",
        "power-peaks.json.tmp",
    ] {
        std::fs::write(dir.join(name), "stale").unwrap();
    }
    assert!(store::clear(&dir).is_ok());
    for name in [
        super::model::HISTORY_FILE,
        super::model::PEAKS_FILE,
        "power-history.jsonl.tmp",
        "power-peaks.json.tmp",
    ] {
        assert!(!dir.join(name).exists(), "{name} survives clear");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn failed_retention_rewrite_does_not_postpone_the_next_retry() {
    let dir = tmp("power-rewrite-failure");
    let mut h = PowerHistory::load_with(&dir, 1_000, 60_000);
    h.buckets.push_front(row(
        now_ms().saturating_sub(120_000),
        1_000,
        100,
        4_000_000,
        None,
        false,
        None,
        None,
    ));
    std::fs::create_dir(dir.join("power-history.jsonl.tmp")).unwrap();
    h.last_rewrite_ms = 0;
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
    h.record(
        &sample(
            t0 + 1_000,
            reading(200_000, 4_000_000, 30.0, 70),
            true,
            "app",
            "balance",
        ),
        500,
    );
    assert_eq!(h.bucket_count(), 1, "expired row is trimmed in memory");
    assert_eq!(h.last_rewrite_ms, 0, "failed rewrite must remain retryable");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn failed_startup_retention_rewrite_remains_retryable_after_restart() {
    let dir = tmp("power-startup-rewrite-failure");
    let old = row(
        now_ms().saturating_sub(120_000),
        60_000,
        100,
        4_000_000,
        None,
        false,
        None,
        None,
    );
    std::fs::write(
        dir.join(super::model::HISTORY_FILE),
        serde_json::to_string(&old).unwrap() + "\n",
    )
    .unwrap();
    std::fs::create_dir(dir.join("power-history.jsonl.tmp")).unwrap();
    let h = PowerHistory::load_with(&dir, 60_000, 60_000);
    assert_eq!(h.bucket_count(), 0);
    assert_eq!(
        h.last_rewrite_ms, 0,
        "startup rewrite failure must retry on close"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn startup_reconciles_missing_peaks_from_loaded_history() {
    let dir = tmp("power-peaks-reconcile");
    let b = PowerBucket {
        min_ua: -2_000_000,
        max_ua: 3_000_000,
        max_temp_c: Some(41.0),
        ..row(
            now_ms().saturating_sub(1_000),
            60_000,
            500_000,
            4_000_000,
            Some(39.0),
            true,
            Some("app"),
            Some("game"),
        )
    };
    std::fs::write(
        dir.join(super::model::HISTORY_FILE),
        serde_json::to_string(&b).unwrap() + "\n",
    )
    .unwrap();
    let h = PowerHistory::load_with(&dir, 1_000, 3_600_000);
    assert_eq!(h.peaks().charge.as_ref().map(|p| p.ua), Some(-2_000_000));
    assert_eq!(h.peaks().discharge.as_ref().map(|p| p.ua), Some(3_000_000));
    assert_eq!(h.peaks().temp.as_ref().map(|p| p.c), Some(41.0));
    assert!(dir.join(super::model::PEAKS_FILE).exists());
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
    h.clear().unwrap();
    assert_eq!(h.bucket_count(), 0);
    assert!(h.peaks().charge.is_none());
    assert!(!dir.join(super::model::HISTORY_FILE).exists());
    assert!(!dir.join(super::model::PEAKS_FILE).exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn power_history_clear_keeps_memory_when_disk_clear_fails() {
    let dir = tmp("power-clear-failure");
    let mut h = PowerHistory::load_with(&dir, 1_000, 3_600_000);
    h.record(
        &sample(
            aligned(1_000),
            reading(-1_000_000, 4_000_000, 30.0, 80),
            true,
            "app",
            "game",
        ),
        500,
    );
    h.flush();
    std::fs::create_dir(dir.join("power-history.jsonl.tmp")).unwrap();
    assert!(h.clear().is_err());
    assert_eq!(h.bucket_count(), 1);
    assert!(h.peaks().charge.is_some());
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
