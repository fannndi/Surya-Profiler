use super::*;
use std::thread::sleep;

/// Real-clock harness: small durations make the grace/backoff math concrete.
fn harness(grace_ms: u64) -> (FloorAudit, Instant) {
    (
        FloorAudit::with(Duration::from_millis(1), Duration::from_millis(grace_ms)),
        Instant::now(),
    )
}

fn allowed(high: bool) -> Observe {
    Observe {
        high,
        allowed: true,
    }
}

#[test]
fn first_high_observation_waits_for_grace_then_reasserts() {
    let (mut a, t0) = harness(20);
    assert_eq!(a.observe(allowed(true), t0), FloorAction::None);
    sleep(Duration::from_millis(25));
    assert_eq!(
        a.observe(allowed(true), Instant::now()),
        FloorAction::Reassert
    );
    assert_eq!(a.attempts(), 1);
}

#[test]
fn low_after_a_reassert_reports_restored_and_resets() {
    let (mut a, t0) = harness(5);
    a.observe(allowed(true), t0);
    sleep(Duration::from_millis(8));
    assert_eq!(
        a.observe(allowed(true), Instant::now()),
        FloorAction::Reassert
    );
    assert_eq!(
        a.observe(allowed(false), Instant::now()),
        FloorAction::Restored
    );
    assert_eq!(a.attempts(), 0);
    // a fresh episode needs its own grace again
    sleep(Duration::from_millis(8));
    assert_eq!(
        a.observe(allowed(true), Instant::now()),
        FloorAction::None,
        "first observation of a new episode only arms the timer"
    );
    sleep(Duration::from_millis(8));
    assert_eq!(
        a.observe(allowed(true), Instant::now()),
        FloorAction::Reassert
    );
}

#[test]
fn disallowed_observation_never_acts_and_resets() {
    let (mut a, t0) = harness(5);
    a.observe(allowed(true), t0);
    sleep(Duration::from_millis(8));
    // a mapped app took the foreground: the audit must stand down
    assert_eq!(
        a.observe(
            Observe {
                high: true,
                allowed: false
            },
            Instant::now()
        ),
        FloorAction::None
    );
    assert_eq!(a.attempts(), 0);
}

#[test]
fn repeated_high_backs_off_instead_of_looping() {
    let (mut a, t0) = harness(5);
    a.observe(allowed(true), t0);
    sleep(Duration::from_millis(8));
    assert_eq!(
        a.observe(allowed(true), Instant::now()),
        FloorAction::Reassert
    );
    // immediately after the first attempt the 30 s backoff gates the next
    assert_eq!(a.observe(allowed(true), Instant::now()), FloorAction::None);
}

#[test]
fn backoff_caps_at_five_minutes() {
    assert_eq!(FloorAudit::backoff(1), Duration::from_secs(30));
    assert_eq!(FloorAudit::backoff(2), Duration::from_secs(60));
    assert_eq!(FloorAudit::backoff(3), Duration::from_secs(120));
    assert_eq!(FloorAudit::backoff(4), Duration::from_secs(240));
    assert_eq!(FloorAudit::backoff(9), Duration::from_secs(300));
}

#[test]
fn yield_note_logs_once_per_stretch() {
    let (mut a, _) = harness(5);
    assert!(a.mark_yield(), "first yield must log");
    assert!(
        !a.mark_yield(),
        "second yield in the stretch must be silent"
    );
    a.reset();
    assert!(a.mark_yield(), "reset must re-arm the yield note");
}

#[test]
fn screen_on_holder_note_logs_once_then_releases() {
    let (mut a, _) = harness(5);
    assert!(a.note_holder(), "first hold must be noted");
    assert!(!a.note_holder(), "repeat holds stay silent");
    assert!(a.holder_noted());
    a.reset();
    assert!(a.note_holder(), "reset re-arms the note");
}

#[test]
fn due_rate_limits_sampling() {
    let (mut a, t0) = harness(5);
    assert!(a.due(t0), "first call is always due");
    assert!(!a.due(t0), "same instant must not re-sample");
    sleep(Duration::from_millis(3));
    assert!(a.due(Instant::now()), "after the interval it samples again");
}
