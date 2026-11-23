//! Shared integration-test harness: spawn the real `surya-prof serve` binary and
//! drive it over JSON-lines. Each test binary uses a subset of these helpers.

#![allow(dead_code)] // a subset is used per test binary

use serde_json::Value;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

pub struct Daemon {
    pub child: Child,
    pub stdin: ChildStdin,
    /// Reader thread -> test thread. `recv_timeout` makes every wait bounded,
    /// even when the daemon is alive but silent.
    events: Receiver<Value>,
}

impl Daemon {
    pub fn spawn(state_dir: &Path, config: &Path) -> Self {
        Self::spawn_env(state_dir, config, &[])
    }

    /// Spawn with extra environment (fake logcat/settings/sysfs overrides).
    /// Daemon stderr is captured in `<state_dir>/daemon-stderr.log`.
    ///
    /// Host tests always run with `SURYAPROFILER_DEBUG=1` and a fake sysfs
    /// root when the caller did not provide one: a test suite executed as
    /// root must never write to the real `/sys` or `/proc`.
    pub fn spawn_env(state_dir: &Path, config: &Path, envs: &[(&str, &str)]) -> Self {
        let bin = env!("CARGO_BIN_EXE_surya-prof");
        let _ = std::fs::create_dir_all(state_dir);
        let log = std::fs::File::create(state_dir.join("daemon-stderr.log")).ok();
        let mut cmd = Command::new(bin);
        cmd.args([
            "serve",
            "--state-dir",
            state_dir.to_str().unwrap(),
            "--config",
            config.to_str().unwrap(),
        ])
        .env("SURYAPROFILER_DEBUG", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(log.map(Stdio::from).unwrap_or_else(Stdio::null));
        if !envs.iter().any(|(k, _)| *k == "SURYAPROFILER_SYSFS_ROOT") {
            let root = state_dir.join("fake-root");
            let _ = std::fs::create_dir_all(&root);
            cmd.env("SURYAPROFILER_SYSFS_ROOT", &root);
        }
        for (k, v) in envs {
            cmd.env(k, v);
        }
        let mut child = cmd.spawn().expect("spawn daemon");
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("test-daemon-stdout".into())
            .spawn(move || {
                use std::io::BufRead;
                let reader = std::io::BufReader::new(stdout);
                for line in reader.lines() {
                    let Ok(line) = line else { break };
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    let v: Value = match serde_json::from_str(line) {
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("bad daemon JSON line: {e}: {line}");
                            continue;
                        }
                    };
                    if std::env::var("SURYAPROFILER_TEST_DEBUG").is_ok() {
                        eprintln!("EVENT: {v}");
                    }
                    if tx.send(v).is_err() {
                        break;
                    }
                }
            })
            .expect("spawn stdout reader");
        Daemon {
            child,
            stdin,
            events: rx,
        }
    }

    pub fn send(&mut self, v: Value) {
        writeln!(self.stdin, "{v}").unwrap();
        self.stdin.flush().unwrap();
    }

    /// Reads events until `pred` matches (or timeout). Returns the event.
    pub fn wait_for(&mut self, pred: impl Fn(&Value) -> bool, timeout: Duration) -> Value {
        let deadline = Instant::now() + timeout;
        let mut last = Value::Null;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(
                !remaining.is_zero(),
                "timeout waiting for event; last: {last}"
            );
            match self.events.recv_timeout(remaining) {
                Ok(v) => {
                    if pred(&v) {
                        return v;
                    }
                    last = v;
                }
                Err(RecvTimeoutError::Timeout) => {
                    panic!("timeout waiting for event; last: {last}");
                }
                Err(RecvTimeoutError::Disconnected) => {
                    panic!("daemon stdout closed unexpectedly; last: {last}");
                }
            }
        }
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.stdin.write_all(b"{\"cmd\":\"shutdown\"}\n");
        let _ = self.stdin.flush();
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if let Ok(Some(_)) = self.child.try_wait() {
                return;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub trait WaitTimeout {
    fn wait_timeout(&mut self, d: Duration) -> std::io::Result<Option<std::process::ExitStatus>>;
}

impl WaitTimeout for Child {
    fn wait_timeout(&mut self, d: Duration) -> std::io::Result<Option<std::process::ExitStatus>> {
        let deadline = Instant::now() + d;
        loop {
            if let Some(st) = self.try_wait()? {
                return Ok(Some(st));
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }
}

/// Fresh temp dir per test (process-id + tag tagged, recreated on entry).
///
/// Lives under `core/target/tmp/` so test artifacts (including
/// `daemon-stderr.log`) stay inside the workspace, not /tmp.
pub fn tmp(tag: &str) -> PathBuf {
    let d = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/tmp")
        .join(format!("{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}
