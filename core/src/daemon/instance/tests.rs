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
    assert!(
        dir.join("daemon.lock").exists(),
        "the OS lock file is persistent"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn concurrent_startups_have_exactly_one_winner() {
    use std::sync::{mpsc, Arc, Barrier};

    let dir = crate::daemon::test_util::tmpdir("daemon-instance-race");
    let barrier = Arc::new(Barrier::new(8));
    let (tx, rx) = mpsc::channel();
    let mut threads = Vec::new();
    for _ in 0..8 {
        let dir = dir.clone();
        let barrier = Arc::clone(&barrier);
        let tx = tx.clone();
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            tx.send(claim_instance(&dir)).unwrap();
        }));
    }
    for thread in threads {
        thread.join().unwrap();
    }
    let results: Vec<_> = (0..8).map(|_| rx.recv().unwrap()).collect();
    assert_eq!(
        results.iter().filter(|r| r.is_ok()).count(),
        1,
        "flock must admit exactly one concurrent writer"
    );
    assert!(running_pid(&dir).is_some());
    drop(results);
    assert!(running_pid(&dir).is_none());
    assert!(
        claim_instance(&dir).is_ok(),
        "drop must release the OS lock"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn drop_never_deletes_a_pid_file_it_does_not_own() {
    let dir = crate::daemon::test_util::tmpdir("daemon-instance-pid-owner");
    let guard = claim_instance(&dir).unwrap();
    std::fs::write(dir.join("daemon.pid"), "999999\n").unwrap();
    drop(guard);
    assert_eq!(
        std::fs::read_to_string(dir.join("daemon.pid"))
            .unwrap()
            .trim(),
        "999999"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
