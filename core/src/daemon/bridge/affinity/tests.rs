//! Affinity state-machine tests with a fake `/proc` tree and `taskset` script.

use super::*;
use crate::daemon::bridge::SyncCtx;
use std::collections::BTreeMap;
use std::path::Path;

fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn fake_host(dir: &Path) -> Host {
    let proc_root = dir.join("proc");
    std::fs::create_dir_all(&proc_root).unwrap();
    let log = dir.join("taskset.log");
    let script = dir.join("taskset.sh");
    let body = format!(
        r#"#!/bin/sh
mask="$2"
tid="$3"
echo "$mask $tid" >> "{log}"
for f in "{root}"/*/task/"$tid"/status; do
  [ -f "$f" ] || continue
  case "$mask" in
    c0) list=6-7 ;;
    40) list=6 ;;
    ff) list=0-7 ;;
    3f) list=0-5 ;;
    *) list=0-7 ;;
  esac
  printf 'Name:\tX\nCpus_allowed_list:\t%s\n' "$list" > "$f"
done
exit 0
"#,
        log = log.display(),
        root = proc_root.display()
    );
    std::fs::write(&script, body).unwrap();
    make_executable(&script);
    Host {
        proc_root,
        taskset: script,
    }
}

fn thread(dir: &Path, pid: u32, tid: u32, comm: &str, list: &str) {
    let p = dir
        .join("proc")
        .join(pid.to_string())
        .join("task")
        .join(tid.to_string());
    std::fs::create_dir_all(&p).unwrap();
    std::fs::write(p.join("comm"), format!("{comm}\n")).unwrap();
    std::fs::write(
        p.join("status"),
        format!("Name:\t{comm}\nCpus_allowed_list:\t{list}\n"),
    )
    .unwrap();
}

fn process(dir: &Path, pid: u32, pkg: &str, uid: u32) {
    let p = dir.join("proc").join(pid.to_string());
    std::fs::create_dir_all(&p).unwrap();
    let mut cmd = pkg.as_bytes().to_vec();
    cmd.push(0);
    std::fs::write(p.join("cmdline"), cmd).unwrap();
    std::fs::write(
        p.join("status"),
        format!("Name:\t{pkg}\nUid:\t{uid}\t{uid}\t{uid}\t{uid}\nCpus_allowed_list:\t0-7\n"),
    )
    .unwrap();
    // starttime is token 19 after ')' -> 19
    let mut stat = format!("{pid} ({pkg}) S");
    for i in 0..25 {
        stat.push_str(&format!(" {i}"));
    }
    std::fs::write(p.join("stat"), stat).unwrap();
}

fn ctx(pkg: &str, profile: &str, enabled: bool) -> SyncCtx {
    let mut app_map = BTreeMap::new();
    app_map.insert(pkg.to_string(), profile.to_string());
    SyncCtx {
        enabled: true,
        last_real: Some(pkg.to_string()),
        screen_on: true,
        locked: false,
        dynamic: true,
        app_map,
        multi_window: false,
        live_saver: false,
        profile: Some(profile.to_string()),
        custom_thermal: true,
        rnd_input_suspend: false,
        thread_affinity: enabled,
    }
}

#[test]
fn gate_requires_game_app_and_screen() {
    let c = ctx("com.test.game", "game", true);
    assert_eq!(want(&c), Some("com.test.game"));

    let mut c = ctx("com.test.game", "balance", true);
    assert_eq!(want(&c), None);
    c.profile = Some("game".into());
    c.screen_on = false;
    assert_eq!(want(&c), None);
    c.screen_on = true;
    c.locked = true;
    assert_eq!(want(&c), None);
    c.locked = false;
    c.thread_affinity = false;
    assert_eq!(want(&c), None);
    c.thread_affinity = true;
    c.dynamic = false;
    assert_eq!(want(&c), None);
    c.dynamic = true;
    c.multi_window = true;
    assert_eq!(want(&c), None);
    c.multi_window = false;
    c.last_real = Some("com.other".into());
    assert_eq!(want(&c), None);
}

#[test]
fn applies_and_restores_the_render_threads() {
    let dir = crate::daemon::test_util::tmpdir("affinity-apply");
    let host = fake_host(&dir);
    process(&dir, 4242, "com.test.game", 10_123);
    thread(&dir, 4242, 4242, "UnityMain", "0-7");
    thread(&dir, 4242, 4243, "UnityGfxDeviceW", "0-7");
    thread(&dir, 4242, 4244, "Binder:4242_1", "0-7");

    let mut st = AffinityState::default();
    assert!(apply(&mut st, "com.test.game", &host));
    assert_eq!(st.threads.len(), 2);
    assert_eq!(st.pid, Some(4242));
    assert!(st
        .threads
        .iter()
        .any(|t| t.tid == 4242 && t.new_mask == 0xC0));
    assert!(st
        .threads
        .iter()
        .any(|t| t.tid == 4243 && t.new_mask == 0x40));
    assert!(!st.threads.iter().any(|t| t.tid == 4244));

    let log = std::fs::read_to_string(dir.join("taskset.log")).unwrap();
    assert!(log.contains("c0 4242"), "{log}");
    assert!(log.contains("40 4243"), "{log}");
    assert!(!log.contains("4244"), "{log}");

    // Idempotent: a second pass does not re-run taskset.
    let before = log.lines().count();
    assert!(!apply(&mut st, "com.test.game", &host));
    assert_eq!(
        std::fs::read_to_string(dir.join("taskset.log"))
            .unwrap()
            .lines()
            .count(),
        before
    );

    assert!(release_with(&mut st, &host).unwrap());
    assert!(st.threads.is_empty());
    assert_eq!(st.target, None);
    let log = std::fs::read_to_string(dir.join("taskset.log")).unwrap();
    assert!(log.contains("ff 4242"), "{log}");
    assert!(log.contains("ff 4243"), "{log}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn skips_a_target_outside_the_cgroup_mask() {
    let dir = crate::daemon::test_util::tmpdir("affinity-cpuset");
    let host = fake_host(&dir);
    process(&dir, 5252, "com.test.game", 10_123);
    thread(&dir, 5252, 5252, "UnityMain", "0-5");

    let mut st = AffinityState::default();
    assert!(apply(&mut st, "com.test.game", &host)); // target change
    assert!(st.threads.is_empty());
    assert!(!dir.join("taskset.log").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn apply_yields_to_an_app_owned_mask() {
    let dir = crate::daemon::test_util::tmpdir("affinity-yield");
    let host = fake_host(&dir);
    process(&dir, 6263, "com.test.game", 10_123);
    thread(&dir, 6263, 6263, "UnityGfx", "0-7");

    let mut st = AffinityState::default();
    assert!(apply(&mut st, "com.test.game", &host));
    assert_eq!(st.threads.len(), 1);
    let calls = std::fs::read_to_string(dir.join("taskset.log"))
        .unwrap()
        .lines()
        .count();

    // The engine re-pins its gfx thread to both gold cores: stop managing it.
    thread(&dir, 6263, 6263, "UnityGfx", "6-7");
    assert!(apply(&mut st, "com.test.game", &host)); // record dropped
    assert!(st.threads.is_empty());
    assert_eq!(st.yielded.len(), 1);
    assert_eq!(
        std::fs::read_to_string(dir.join("taskset.log"))
            .unwrap()
            .lines()
            .count(),
        calls,
        "must not fight the app's own affinity"
    );

    // The yield is sticky for this app session: even a later wide mask is
    // not re-pinned on the next supervisor tick.
    thread(&dir, 6263, 6263, "UnityGfx", "0-7");
    assert!(!apply(&mut st, "com.test.game", &host));
    assert_eq!(
        std::fs::read_to_string(dir.join("taskset.log"))
            .unwrap()
            .lines()
            .count(),
        calls
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn never_widens_a_mask_changed_by_someone_else() {
    let dir = crate::daemon::test_util::tmpdir("affinity-external");
    let host = fake_host(&dir);
    process(&dir, 6262, "com.test.game", 10_123);
    thread(&dir, 6262, 6262, "UnityMain", "0-7");

    let mut st = AffinityState::default();
    assert!(apply(&mut st, "com.test.game", &host));
    assert_eq!(st.threads.len(), 1);

    // MIUI/user re-pinned the thread: our applied mask is gone.
    thread(&dir, 6262, 6262, "UnityMain", "0-3");
    let calls_before = std::fs::read_to_string(dir.join("taskset.log"))
        .unwrap()
        .lines()
        .count();
    assert!(release_with(&mut st, &host).unwrap());
    assert!(st.threads.is_empty());
    let calls_after = std::fs::read_to_string(dir.join("taskset.log"))
        .unwrap()
        .lines()
        .count();
    assert_eq!(
        calls_before, calls_after,
        "must not restore over external pin"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn release_drops_a_reused_tid() {
    let dir = crate::daemon::test_util::tmpdir("affinity-reuse");
    let host = fake_host(&dir);
    process(&dir, 7272, "com.test.game", 10_123);
    thread(&dir, 7272, 7272, "UnityMain", "0-7");

    let mut st = AffinityState::default();
    apply(&mut st, "com.test.game", &host);
    thread(&dir, 7272, 7272, "OtherThread", "6-7");
    assert!(release_with(&mut st, &host).unwrap());
    assert!(st.threads.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn release_clears_a_target_without_threads() {
    let dir = crate::daemon::test_util::tmpdir("affinity-target-only");
    let host = fake_host(&dir);
    let mut st = AffinityState {
        target: Some("com.test.game".into()),
        pid: Some(4242),
        starttime: Some(19),
        ..AffinityState::default()
    };
    assert!(release_with(&mut st, &host).unwrap());
    assert_eq!(st.target, None);
    assert_eq!(st.pid, None);
    assert!(!st.managed());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn persisted_state_roundtrips() {
    let st = AffinityState {
        target: Some("com.test.game".into()),
        pid: Some(4242),
        starttime: Some(19),
        threads: vec![AppliedThread {
            pid: 4242,
            starttime: 19,
            tid: 4242,
            comm: "UnityMain".into(),
            old_mask: 0xFF,
            new_mask: 0xC0,
        }],
        yielded: vec![],
        no_pid_logged: true,
    };
    let json = serde_json::to_string(&st).unwrap();
    let back: AffinityState = serde_json::from_str(&json).unwrap();
    assert_eq!(back.target.as_deref(), Some("com.test.game"));
    assert_eq!(back.threads.len(), 1);
    assert!(!back.no_pid_logged, "transient flag must not persist");
}

#[test]
fn missing_pid_is_logged_once_and_waits() {
    let dir = crate::daemon::test_util::tmpdir("affinity-nopid");
    let host = fake_host(&dir);
    let mut st = AffinityState::default();
    assert!(apply(&mut st, "com.test.game", &host)); // target change
    assert_eq!(st.target.as_deref(), Some("com.test.game"));
    assert!(st.no_pid_logged);
    // A second pass keeps waiting without changing state again.
    assert!(!apply(&mut st, "com.test.game", &host));
    let _ = std::fs::remove_dir_all(&dir);
}
