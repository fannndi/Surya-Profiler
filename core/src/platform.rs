//! Low-level process/OS helpers shared by the engine, daemon and doctor.
//!
//! `run` drains stdout/stderr on reader threads and kills the child after a
//! deadline. `Command::output()` alone has no timeout, so a hung `settings`
//! or `dumpsys` could otherwise block the bridge/decision loop forever.

use std::io::Read;
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

pub struct Output {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub timed_out: bool,
}

impl Output {
    pub fn success(&self) -> bool {
        !self.timed_out && self.status.success()
    }

    pub fn stdout_lossy(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    pub fn stderr_lossy(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }
}

/// Runs `bin args...` with a hard timeout; output is always drained.
pub fn run(bin: &str, args: &[&str], timeout: Duration) -> Option<Output> {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let mut stderr = child.stderr.take()?;
    let (out_tx, out_rx) = std::sync::mpsc::channel();
    let (err_tx, err_rx) = std::sync::mpsc::channel();
    if std::thread::Builder::new()
        .name("cmd-stdout".into())
        .spawn(move || {
            let mut buf = Vec::new();
            let _ = stdout.read_to_end(&mut buf);
            let _ = out_tx.send(buf);
        })
        .is_err()
    {
        let _ = child.kill();
        let _ = child.wait();
        return None;
    }
    if std::thread::Builder::new()
        .name("cmd-stderr".into())
        .spawn(move || {
            let mut buf = Vec::new();
            let _ = stderr.read_to_end(&mut buf);
            let _ = err_tx.send(buf);
        })
        .is_err()
    {
        let _ = child.kill();
        let _ = child.wait();
        return None;
    }

    let deadline = Instant::now() + timeout;
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                timed_out = true;
                let _ = child.kill();
                break child.wait().ok()?;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(_) => {
                let _ = child.kill();
                break child.wait().ok()?;
            }
        }
    };
    // A grandchild can keep the pipe open after the child exits; never let a
    // reader join block the daemon. Partial output is better than a hang.
    let stdout = out_rx
        .recv_timeout(Duration::from_secs(1))
        .unwrap_or_default();
    let stderr = err_rx
        .recv_timeout(Duration::from_secs(1))
        .unwrap_or_default();
    Some(Output {
        status,
        stdout,
        stderr,
        timed_out,
    })
}
