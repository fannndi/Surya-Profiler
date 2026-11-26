//! OS-backed instance guard: one daemon/CLI writer per state dir.
//!
//! `daemon.lock` is persistent and never unlinked. `flock` closes the
//! create-new/unlink race; `daemon.pid` is retained only for diagnostics.

use fs2::FileExt;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Holds the exclusive OS lock and removes only its own diagnostic PID file.
pub struct InstanceGuard {
    path: PathBuf,
    lock: std::fs::File,
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        // Remove the PID while the lock is still ours. A replacement writer
        // cannot acquire the lock until this guard is dropped.
        let mine = std::process::id().to_string();
        let current = std::fs::read_to_string(&self.path)
            .ok()
            .map(|s| s.trim().to_string());
        if current.as_deref() == Some(mine.as_str()) {
            let _ = std::fs::remove_file(&self.path);
        }
        let _ = self.lock.unlock();
        // Never remove daemon.lock: another process may already have opened it.
    }
}

fn pid_file(state_dir: &Path) -> PathBuf {
    state_dir.join("daemon.pid")
}

fn lock_file(state_dir: &Path) -> PathBuf {
    state_dir.join("daemon.lock")
}

fn open_lock(state_dir: &Path) -> Result<(std::fs::File, PathBuf), String> {
    std::fs::create_dir_all(state_dir).map_err(|e| format!("{}: {e}", state_dir.display()))?;
    let path = lock_file(state_dir);
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok((file, path))
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

fn lock_busy_error(state_dir: &Path, lock_path: &Path) -> String {
    match running_pid(state_dir) {
        Some(pid) => format!("another daemon is running (pid {pid})"),
        None => format!("another writer holds {}", lock_path.display()),
    }
}

fn try_exclusive(file: &std::fs::File, lock_path: &Path) -> Result<(), String> {
    file.try_lock_exclusive().map_err(|e| {
        if e.kind() == std::io::ErrorKind::WouldBlock {
            lock_busy_error(
                lock_path.parent().unwrap_or_else(|| Path::new(".")),
                lock_path,
            )
        } else {
            format!("{}: lock: {e}", lock_path.display())
        }
    })
}

pub fn claim_instance(state_dir: &Path) -> Result<InstanceGuard, String> {
    let (lock, lock_path) = open_lock(state_dir)?;
    try_exclusive(&lock, &lock_path)?;

    // Also recognize a pre-flock daemon during a rolling upgrade. The OS lock
    // remains authoritative for all current writers.
    if let Some(pid) = running_pid(state_dir) {
        if pid != std::process::id() {
            return Err(format!("another daemon is running (pid {pid})"));
        }
    }

    let path = pid_file(state_dir);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    writeln!(file, "{}", std::process::id()).map_err(|e| format!("{}: {e}", path.display()))?;
    file.sync_all()
        .map_err(|e| format!("{}: fsync: {e}", path.display()))?;
    Ok(InstanceGuard { path, lock })
}

pub fn ensure_no_daemon(state_dir: &Path) -> Result<(), String> {
    let (lock, lock_path) = open_lock(state_dir)?;
    try_exclusive(&lock, &lock_path)?;
    if let Some(pid) = running_pid(state_dir) {
        if pid != std::process::id() {
            return Err(format!(
                "daemon is running (pid {pid}); use the app or stop the service first"
            ));
        }
    }
    // Dropping the file releases the probe lock; daemon.lock stays on disk.
    Ok(())
}

#[cfg(test)]
mod tests;
