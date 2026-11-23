//! readback unit tests (split from readback.rs).

use super::*;

#[test]
fn repeat_int_readback_all_equal() {
    assert!(readback_matches(Kind::RepeatInt, "60", "60 60 60 60 60 60"));
    assert!(!readback_matches(
        Kind::RepeatInt,
        "60",
        "60 40 60 60 60 60"
    ));
    // a full-array resolved value compares element-wise (stock capture)
    assert!(readback_matches(
        Kind::RepeatInt,
        "60 60 60 60 60 60",
        "60 60 60 60 60 60"
    ));
    assert!(!readback_matches(
        Kind::RepeatInt,
        "60 60 60 60 60 60",
        "60 60 55 60 60 60"
    ));
}

#[test]
fn flagyn_readback_flexible() {
    assert!(readback_matches(Kind::FlagYN, "Y", "Y"));
    assert!(readback_matches(Kind::FlagYN, "N", "0"));
    assert!(!readback_matches(Kind::FlagYN, "Y", "N"));
}

#[test]
fn io_sched_readback_requires_active_bracket() {
    // only the BRACKETED token is the active scheduler; a bare token in
    // the offered list must NOT count (real bug: "deadline" matched while
    // cfq was active and the elevator never switched).
    assert!(readback_matches(
        Kind::IoSched,
        "cfq",
        "noop deadline [cfq]"
    ));
    assert!(readback_matches(
        Kind::IoSched,
        "deadline",
        "noop [deadline] cfq"
    ));
    assert!(!readback_matches(
        Kind::IoSched,
        "deadline",
        "noop deadline [cfq]"
    ));
    assert!(!readback_matches(Kind::IoSched, "cfq", "noop [deadline]"));
    assert!(!readback_matches(Kind::IoSched, "noop", "deadline [cfq]"));
    // bare (unbracketed) read-backs still compare directly
    assert!(readback_matches(Kind::IoSched, "cfq", "cfq"));
}

#[test]
fn snapshot_normalize_makes_values_node_writable() {
    assert_eq!(
        normalize_snapshot(Kind::IoSched, "noop deadline [cfq]"),
        "cfq"
    );
    assert_eq!(
        normalize_snapshot(Kind::Ints, "524288\t1048576\t5505024"),
        "524288 1048576 5505024"
    );
    assert_eq!(normalize_snapshot(Kind::Mask, "0-5"), "0-5");
    assert_eq!(normalize_snapshot(Kind::Mask, "0-2, 4-7"), "0-2,4-7");
    assert_eq!(normalize_snapshot(Kind::Mask, "0-2 4-7"), "0-2,4-7");
    assert_eq!(normalize_snapshot(Kind::FlagYN, "n"), "N");
    assert_eq!(normalize_snapshot(Kind::Int, " 65 "), "65");
    // uniform RepeatInt arrays collapse to the broadcast single value
    assert_eq!(
        normalize_snapshot(Kind::RepeatInt, "60 60 60 60 60 60"),
        "60"
    );
    assert_eq!(
        normalize_snapshot(Kind::RepeatInt, "60 40 60 60 60 60"),
        "60 40 60 60 60 60"
    );
}

#[test]
fn freq_max_accepts_stricter_external_cap() {
    // thermal cooling holds scaling_max below the requested cap -> thermal
    // wins (harmony rule), reported as in-sync, not as drift.
    assert!(readback_matches(Kind::FreqMax, "1555200", "1209600"));
    assert!(readback_matches(Kind::FreqMax, "1555200", "1555200"));
    assert!(!readback_matches(Kind::FreqMax, "1555200", "1804800"));
    // plain Freq (e.g. scaling_min_freq) stays exact
    assert!(!readback_matches(Kind::Freq, "1555200", "1209600"));
}

#[test]
fn freq_min_accepts_external_qos_floor() {
    // perf HAL msm_performance cpu_min_freq QoS holds the effective floor
    // ABOVE the requested value -> framework wins (harmony rule), in-sync,
    // not drift. Only live < want (floor lost) is drift.
    // Live case 2026-10-07: wrote 576000, read back 1248000 under QoS.
    assert!(readback_matches(Kind::FreqMin, "576000", "1248000"));
    assert!(readback_matches(Kind::FreqMin, "576000", "576000"));
    assert!(!readback_matches(Kind::FreqMin, "1094400", "768000"));
    // and a QoS-held floor must not leak into exact-match kinds
    assert!(!readback_matches(Kind::Freq, "576000", "1248000"));
}
