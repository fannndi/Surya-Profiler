//! Host E2E for opt-in thread placement: a Game-mapped app gets its main and
//! render threads pinned, and the captured masks are restored on leave.

mod common;

use common::{tmp, Daemon};
use serde_json::json;
use std::path::Path;
use std::time::{Duration, Instant};

fn executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn fake_logcat(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("logcat.sh");
    std::fs::write(
        &path,
        "#!/bin/sh\ncase \"$*\" in *\"-d\"*) exit 0 ;; esac\nwhile true; do sleep 1; done\n",
    )
    .unwrap();
    executable(&path);
    path
}

fn fake_dumpsys(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("dumpsys.sh");
    std::fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
    executable(&path);
    path
}

fn fake_proc(dir: &Path) -> std::path::PathBuf {
    let root = dir.join("proc");
    let pid = 4242u32;
    let p = root.join(pid.to_string());
    std::fs::create_dir_all(p.join("task")).unwrap();
    let mut cmd = b"com.test.game".to_vec();
    cmd.push(0);
    std::fs::write(p.join("cmdline"), cmd).unwrap();
    std::fs::write(
        p.join("status"),
        "Name:\tcom.test.game\nUid:\t10123\t10123\t10123\t10123\nCpus_allowed_list:\t0-7\n",
    )
    .unwrap();
    let mut stat = format!("{pid} (com.test.game) S");
    for i in 0..25 {
        stat.push_str(&format!(" {i}"));
    }
    std::fs::write(p.join("stat"), stat).unwrap();
    for (tid, comm) in [(4242u32, "UnityMain"), (4243, "UnityGfxDeviceW")] {
        let t = p.join("task").join(tid.to_string());
        std::fs::create_dir_all(&t).unwrap();
        std::fs::write(t.join("comm"), format!("{comm}\n")).unwrap();
        std::fs::write(
            t.join("status"),
            format!("Name:\t{comm}\nCpus_allowed_list:\t0-7\n"),
        )
        .unwrap();
    }
    root
}

fn fake_taskset(dir: &Path, proc_root: &Path) -> std::path::PathBuf {
    let path = dir.join("taskset.sh");
    let log = dir.join("taskset.log");
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
    *) list=0-7 ;;
  esac
  printf 'Name:\tX\nCpus_allowed_list:\t%s\n' "$list" > "$f"
done
exit 0
"#,
        log = log.display(),
        root = proc_root.display()
    );
    std::fs::write(&path, body).unwrap();
    executable(&path);
    path
}

fn wait_for_log(path: &Path, needle: &str) -> String {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        if text.contains(needle) {
            return text;
        }
        assert!(
            Instant::now() < deadline,
            "timeout waiting for {needle}: {text}"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn game_threads_are_pinned_and_restored() {
    let dir = tmp("affinity-e2e");
    let proc_root = fake_proc(&dir);
    let taskset = fake_taskset(&dir, &proc_root);
    let logcat = fake_logcat(&dir);
    let dumpsys = fake_dumpsys(&dir);
    std::fs::write(
        dir.join("config.json"),
        r#"{"schema":2,"enabled":true,"dynamic":true,"base_profile":"balance",
            "app_map":{"com.test.game":"game"},"thread_affinity":true}"#,
    )
    .unwrap();

    let mut d = Daemon::spawn_env(
        &dir.join("state"),
        &dir.join("config.json"),
        &[
            ("SURYAPROFILER_LOGCAT_BIN", logcat.to_str().unwrap()),
            ("SURYAPROFILER_DUMPSYS_BIN", dumpsys.to_str().unwrap()),
            ("SURYAPROFILER_PROC_ROOT", proc_root.to_str().unwrap()),
            ("SURYAPROFILER_TASKSET_BIN", taskset.to_str().unwrap()),
        ],
    );
    d.send(json!({"cmd":"hello"}));
    d.wait_for(|v| v["event"] == "state", Duration::from_secs(10));
    d.send(json!({"cmd":"screen","on":true,"locked":false}));
    d.send(json!({"cmd":"user_present"}));
    d.send(json!({"cmd":"fg","pkg":"com.test.game"}));

    let applied = d.wait_for(
        |v| v["event"] == "applied" && v["profile"] == "game",
        Duration::from_secs(20),
    );
    assert_eq!(applied["reason"], "app");

    let log = dir.join("taskset.log");
    let text = wait_for_log(&log, "c0 4242");
    assert!(text.contains("40 4243"), "{text}");

    // Leaving the game returns to the base and restores every captured mask.
    d.send(json!({"cmd":"fg","pkg":"com.android.chrome"}));
    d.wait_for(
        |v| v["event"] == "applied" && v["profile"] == "balance",
        Duration::from_secs(20),
    );
    let text = wait_for_log(&log, "ff 4242");
    assert!(text.contains("ff 4243"), "{text}");

    // Service OFF must also restore placement (fail-closed report).
    d.send(json!({"cmd":"fg","pkg":"com.test.game"}));
    d.wait_for(
        |v| v["event"] == "applied" && v["profile"] == "game",
        Duration::from_secs(20),
    );
    wait_for_log(&log, "c0 4242");
    d.send(json!({"cmd":"restore"}));
    let restored = d.wait_for(|v| v["event"] == "restored", Duration::from_secs(30));
    // Host simulation has no stock snapshot; affinity is the surface under test.
    assert_eq!(restored["affinity_ok"], true, "{restored}");
    let text = wait_for_log(&log, "ff 4242");
    assert!(text.contains("ff 4243"), "{text}");

    d.send(json!({"cmd":"shutdown"}));
    d.wait_for(|v| v["event"] == "bye", Duration::from_secs(5));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn placement_stays_off_when_the_switch_is_false() {
    let dir = tmp("affinity-off");
    let proc_root = fake_proc(&dir);
    let taskset = fake_taskset(&dir, &proc_root);
    let logcat = fake_logcat(&dir);
    let dumpsys = fake_dumpsys(&dir);
    std::fs::write(
        dir.join("config.json"),
        r#"{"schema":2,"enabled":true,"dynamic":true,"base_profile":"balance",
            "app_map":{"com.test.game":"game"}}"#,
    )
    .unwrap();

    let mut d = Daemon::spawn_env(
        &dir.join("state"),
        &dir.join("config.json"),
        &[
            ("SURYAPROFILER_LOGCAT_BIN", logcat.to_str().unwrap()),
            ("SURYAPROFILER_DUMPSYS_BIN", dumpsys.to_str().unwrap()),
            ("SURYAPROFILER_PROC_ROOT", proc_root.to_str().unwrap()),
            ("SURYAPROFILER_TASKSET_BIN", taskset.to_str().unwrap()),
        ],
    );
    d.send(json!({"cmd":"hello"}));
    d.wait_for(|v| v["event"] == "state", Duration::from_secs(10));
    d.send(json!({"cmd":"screen","on":true,"locked":false}));
    d.send(json!({"cmd":"user_present"}));
    d.send(json!({"cmd":"fg","pkg":"com.test.game"}));
    d.wait_for(
        |v| v["event"] == "applied" && v["profile"] == "game",
        Duration::from_secs(20),
    );
    std::thread::sleep(Duration::from_millis(800));
    assert!(
        !dir.join("taskset.log").exists(),
        "switch off must never call taskset"
    );
    d.send(json!({"cmd":"shutdown"}));
    d.wait_for(|v| v["event"] == "bye", Duration::from_secs(5));
    let _ = std::fs::remove_dir_all(&dir);
}
