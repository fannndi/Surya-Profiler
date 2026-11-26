//! Hold state-machine tests.

use super::*;

#[test]
fn saver_hold_turns_on_then_restores() {
    let s = Holds::default();
    let (n, a) = s.request_saver(false, true);
    assert!(matches!(a, SaverAction::TurnOn));
    assert!(n.saver_held);
    assert!(!n.saver_saved);

    let (n2, a2) = n.request_saver(true, false);
    assert!(matches!(a2, SaverAction::Restore));
    assert!(!n2.saver_held);
}

#[test]
fn held_saver_reasserts_after_a_live_change() {
    let (held, _) = Holds::default().request_saver(false, true);
    // The user (or MIUI) turned it off while the mapped app is in front:
    // the follow must turn it back on and keep the original restore point.
    let (next, action) = held.request_saver(false, true);
    assert!(matches!(action, SaverAction::TurnOn));
    assert!(!next.saver_saved);
}

#[test]
fn held_input_suspend_reengages_after_a_live_change() {
    let (held, _) = Holds::default().request_input_suspend(Some("0".into()), true);
    let (next, action) = held.request_input_suspend(Some("0".into()), true);
    assert!(matches!(action, InputSuspendAction::Engage));
    assert_eq!(next.input_suspend_saved.as_deref(), Some("0"));
}

#[test]
fn user_saver_attribution_our_hold_is_invisible_to_arbiter() {
    let s = Holds::default();
    let (n, _) = s.request_saver(false, true);
    // while held, the arbiter must NOT see our write as user intent
    assert!(!n.user_saver(true));
    // after release it sees the live flag again
    let (n2, _) = n.request_saver(true, false);
    assert!(n2.user_saver(true));
}

#[test]
fn user_saver_own_choice_passes_through() {
    let s = Holds::default();
    assert!(s.user_saver(true));
    assert!(!s.user_saver(false));
}

#[test]
fn recovered_saver_hold_restores_captured_value_not_live() {
    // daemon died while holding (user's saver was off, ours is on)
    let s = Holds {
        saver_held: true,
        saver_saved: false,
        ..Default::default()
    };
    let (n, a) = s.request_saver(true, false);
    assert!(matches!(a, SaverAction::Restore));
    assert!(!n.saver_saved);
}

#[test]
fn input_suspend_hold_engages_and_releases() {
    let s = Holds::default();
    let (n, a) = s.request_input_suspend(Some("0".into()), true);
    assert!(matches!(a, InputSuspendAction::Engage));
    assert!(n.input_suspend_held);
    assert_eq!(n.input_suspend_saved.as_deref(), Some("0"));

    // still wanted -> keep (no repeated writes, capture frozen)
    let (n2, a2) = n.request_input_suspend(Some("1".into()), true);
    assert!(matches!(a2, InputSuspendAction::Keep));
    assert_eq!(n2.input_suspend_saved.as_deref(), Some("0"));

    // release -> restore the captured 0
    let (n3, a3) = n2.request_input_suspend(Some("1".into()), false);
    assert!(matches!(a3, InputSuspendAction::Release(Some(ref v)) if v == "0"));
    assert!(!n3.input_suspend_held);
    assert!(n3.input_suspend_saved.is_none());
}

#[test]
fn input_suspend_hold_assumes_stock_when_capture_missing() {
    let s = Holds::default();
    let (n, a) = s.request_input_suspend(None, true);
    assert!(matches!(a, InputSuspendAction::Engage));
    assert!(n.input_suspend_saved.is_none());
    let (_, a2) = n.request_input_suspend(Some("1".into()), false);
    assert!(matches!(a2, InputSuspendAction::Release(None)));
}

#[test]
fn unowned_live_input_suspend_one_is_released_before_a_new_hold() {
    let s = Holds::default();
    let (next, action) = s.request_input_suspend(Some("1".into()), true);
    assert!(
        matches!(action, InputSuspendAction::Release(Some(ref v)) if v == "0"),
        "an orphan must be treated as stock 0, never captured as stock 1"
    );
    assert!(!next.input_suspend_held);
    assert!(next.input_suspend_saved.is_none());

    // The following sync can now capture the released stock value and engage.
    let (held, engage) = next.request_input_suspend(Some("0".into()), true);
    assert!(matches!(engage, InputSuspendAction::Engage));
    assert_eq!(held.input_suspend_saved.as_deref(), Some("0"));
}

#[test]
fn unowned_live_input_suspend_one_is_released_even_when_not_wanted() {
    let (next, action) = Holds::default().request_input_suspend(Some("1".into()), false);
    assert!(matches!(action, InputSuspendAction::Release(Some(ref v)) if v == "0"));
    assert!(!next.input_suspend_held);
}

#[test]
fn persisted_input_suspend_hold_keeps_its_restore_point() {
    let held = Holds {
        input_suspend_held: true,
        input_suspend_saved: Some("0".into()),
        ..Default::default()
    };
    let (next, action) = held.request_input_suspend(Some("1".into()), true);
    assert!(matches!(action, InputSuspendAction::Keep));
    assert_eq!(next.input_suspend_saved.as_deref(), Some("0"));
}
