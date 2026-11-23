use super::*;

/// Ring-only test logger (no stderr spam, no file sink).
fn test_logger(level: Level) -> Logger {
    Logger {
        level: AtomicU8::new(level as u8),
        ring: Mutex::new(VecDeque::new()),
        sink: Mutex::new(None),
    }
}

#[test]
fn ring_keeps_the_newest_lines_only() {
    // a dedicated logger: the global one is shared with concurrent tests
    let l = test_logger(Level::Debug);
    for i in 0..(RING_LINES + 50) {
        l.push(format!("line-{i}"));
    }
    let ring = l.ring.lock().unwrap();
    let tail: Vec<String> = ring.iter().rev().take(3).rev().cloned().collect();
    assert_eq!(
        tail,
        vec![
            format!("line-{}", RING_LINES + 47),
            format!("line-{}", RING_LINES + 48),
            format!("line-{}", RING_LINES + 49),
        ]
    );
}

#[test]
fn parse_level_accepts_aliases_and_rejects_garbage() {
    assert_eq!(parse_level("warn"), Some(Level::Warn));
    assert_eq!(parse_level("WARNING"), Some(Level::Warn));
    assert_eq!(parse_level("trace"), Some(Level::Debug));
    assert_eq!(parse_level("nope"), None);
}

#[test]
fn file_sink_writes_and_rotates() {
    let dir = std::env::temp_dir().join(format!("sp-logging-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let logs = dir.join("logs");
    std::fs::create_dir_all(&logs).unwrap();

    let l = test_logger(Level::Debug);
    {
        let mut slot = l.sink.lock().unwrap();
        *slot = Some(Sink {
            path: logs.join("daemon.log"),
            file: None,
            written: 0,
            rotate_at: 40,
        });
    }
    for i in 0..10 {
        // ~46 bytes each: crosses the 40-byte rotate threshold repeatedly
        l.push(format!("rot-line-{i}-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
    }
    let cur = std::fs::read_to_string(logs.join("daemon.log")).unwrap();
    let gen = std::fs::read_to_string(logs.join("daemon.log.1")).unwrap_or_default();
    assert!(!cur.is_empty(), "current file must exist");
    assert!(
        cur.contains("rot-line-9"),
        "newest line must be in the current generation"
    );
    assert!(!gen.is_empty(), "rotation must have produced daemon.log.1");
    assert!(
        !gen.contains("rot-line-9"),
        "the old generation must hold older lines only"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn civil_from_days_matches_known_dates() {
    // 1970-01-01, 2026-11-24, 2000-02-29
    assert_eq!(civil_from_days(0), (1970, 1, 1));
    assert_eq!(civil_from_days(20_781), (2026, 11, 24));
    assert_eq!(civil_from_days(11_016), (2000, 2, 29));
}
