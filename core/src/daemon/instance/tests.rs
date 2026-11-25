//! PID lock tests.

use super::*;

#[test]
fn instance_lock_rejects_a_second_writer_and_ignores_stale_pid() {
    let dir = crate::daemon::test_util::tmpdir("daemon-instance");
    std::fs::write(dir.join("daemon.pid"), "999999\n").unwrap();
    let guard = claim_instance(&dir).expect("stale pid must not block");
    assert!(running_pid(&dir).is_some());
    assert!(
        claim_instance(&dir).is_err(),
        "second daemon must be rejected"
    );
    drop(guard);
    assert!(running_pid(&dir).is_none());
    let _ = std::fs::remove_dir_all(&dir);
}
