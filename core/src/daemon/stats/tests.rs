//! stats unit tests (split from stats.rs).

use super::*;

fn env() -> EnvSnapshot {
    EnvSnapshot {
        battery_pct: Some(85),
        cpu_temp_c: Some(38.5),
        ..Default::default()
    }
}

#[test]
fn record_appends_with_env_context() {
    let dir = crate::daemon::test_util::tmpdir("stats-rec");
    let mut s = Stats::load(&dir);
    s.record(None, "balance", "base", &env());
    s.record(Some("balance"), "game", "app", &env());
    assert_eq!(s.entries.len(), 2);
    assert_eq!(s.entries[0].from, None);
    assert_eq!(s.entries[1].from.as_deref(), Some("balance"));
    assert_eq!(s.entries[1].to, "game");
    assert_eq!(s.entries[1].battery, Some(85));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn history_survives_a_reload() {
    let dir = crate::daemon::test_util::tmpdir("stats-reload");
    {
        let mut s = Stats::load(&dir);
        s.record(None, "game", "app", &env());
    }
    let s = Stats::load(&dir);
    assert_eq!(s.entries.len(), 1);
    assert_eq!(s.entries[0].to, "game");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn ring_caps_at_max_keeping_newest() {
    let dir = crate::daemon::test_util::tmpdir("stats-cap");
    let mut s = Stats::load(&dir);
    for i in 0..(MAX_ENTRIES + 10) {
        s.record(None, &format!("p{i}"), "base", &EnvSnapshot::default());
    }
    assert_eq!(s.entries.len(), MAX_ENTRIES);
    assert_eq!(s.entries.first().unwrap().to, format!("p{}", 10));
    assert_eq!(
        s.entries.last().unwrap().to,
        format!("p{}", MAX_ENTRIES + 9)
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn corrupt_file_starts_empty_not_panic() {
    let dir = crate::daemon::test_util::tmpdir("stats-corrupt");
    fs::write(dir.join("stats.json"), "{broken").unwrap();
    let s = Stats::load(&dir);
    assert!(s.entries.is_empty());
    assert_eq!(s.heals_total, 0);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn heal_counter_persists_across_reload() {
    let dir = crate::daemon::test_util::tmpdir("stats-heal");
    {
        let mut s = Stats::load(&dir);
        s.record_heal(3);
        s.record_heal(1);
        assert_eq!(s.heals_total, 2);
        assert_eq!(s.heals_last_keys, 1);
    }
    let s = Stats::load(&dir);
    assert_eq!(s.heals_total, 2);
    assert_eq!(s.heals_last_keys, 1);
    assert!(s.heals_last_t > 0);
    let _ = fs::remove_dir_all(&dir);
}
