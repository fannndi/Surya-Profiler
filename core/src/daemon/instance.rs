//! PID-file guard: one daemon/CLI writer per state dir.
//!
//! Split from `mod.rs` so the daemon state type stays under the file budget.

use std::io::Write;
use std::path::{Path, PathBuf};

/// Removes the PID file on drop only when it still names this process; a
/// losing starter must never delete the winner's lock.
pub struct InstanceGuard {
    path: PathBuf,
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        let mine = std::process::id().to_string();
        let current = std::fs::read_to_string(&self.path)
            .ok()
            .map(|s| s.trim().to_string());
        if current.as_deref() == Some(mine.as_str()) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

fn pid_file(state_dir: &Path) -> PathBuf {
    state_dir.join("daemon.pid")
}

fn pid_is_live(pid: u32) -> bool {
    if pid == std::process::id() {
        return true;
    }
    let proc_path = format!("/proc/{pid}");
    let proc_dir = Path::new(&proc_path);
    if !proc_dir.exists() {
        return false;
    }
    // Guard against PID reuse by an unrelated process after a hard kill.
    std::fs::read_to_string(proc_dir.join("cmdline"))
        .map(|s| s.contains("surya-prof"))
        .unwrap_or(false)
}

pub fn running_pid(state_dir: &Path) -> Option<u32> {
    let raw = std::fs::read_to_string(pid_file(state_dir)).ok()?;
    let pid = raw.trim().parse::<u32>().ok()?;
    pid_is_live(pid).then_some(pid)
}

pub fn claim_instance(state_dir: &Path) -> Result<InstanceGuard, String> {
    let _ = std::fs::create_dir_all(state_dir);
    let path = pid_file(state_dir);
    for _ in 0..3 {
        if let Some(pid) = running_pid(state_dir) {
            return Err(format!("another daemon is running (pid {pid})"));
        }
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut f) => {
                writeln!(f, "{}", std::process::id())
                    .map_err(|e| format!("{}: {e}", path.display()))?;
                f.sync_all()
                    .map_err(|e| format!("{}: fsync: {e}", path.display()))?;
                return Ok(InstanceGuard { path });
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // A winner may have created the file but not written the pid
                // yet; give it a moment before treating the lock as stale.
                for _ in 0..5 {
                    if let Some(pid) = running_pid(state_dir) {
                        return Err(format!("another daemon is running (pid {pid})"));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                let _ = std::fs::remove_file(&path);
            }
            Err(e) => return Err(format!("{}: {e}", path.display())),
        }
    }
    Err(format!(
        "could not claim {} (another writer keeps racing)",
        path.display()
    ))
}

pub fn ensure_no_daemon(state_dir: &Path) -> Result<(), String> {
    if let Some(pid) = running_pid(state_dir) {
        Err(format!(
            "daemon is running (pid {pid}); use the app or stop the service first"
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
