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
