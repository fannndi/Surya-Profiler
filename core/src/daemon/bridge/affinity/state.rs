//! Persisted placement state + capture/restore.
//!
//! Split from `mod.rs` to keep each production file inside the 300-line
//! budget. The rule table and the apply loop live in the parent module.

use super::proc::{self, Host};
use super::{info, warn};
use serde::{Deserialize, Serialize};

/// One thread whose affinity we changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AppliedThread {
    pub pid: u32,
    pub starttime: u64,
    pub tid: u32,
    pub comm: String,
    /// Mask before our first write (the restore point).
    pub old_mask: u64,
    /// Mask we last wrote.
    pub new_mask: u64,
}

/// One thread the app (or another writer) re-pinned after us. We stop
/// managing it for the rest of the app session instead of fighting it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct YieldedThread {
    pub pid: u32,
    pub tid: u32,
    pub comm: String,
}

/// Persisted placement state (`<state_dir>/affinity.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct AffinityState {
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub pid: Option<u32>,
    #[serde(default)]
    pub starttime: Option<u64>,
    #[serde(default)]
    pub threads: Vec<AppliedThread>,
    /// Threads the app took back; never re-pin them until the target changes.
    #[serde(default)]
    pub yielded: Vec<YieldedThread>,
    /// Suppress the repeated "process not started yet" line.
    #[serde(skip)]
    pub(crate) no_pid_logged: bool,
}

impl AffinityState {
    pub(crate) fn active(&self) -> bool {
        !self.threads.is_empty()
    }

    /// True when there is anything to reconcile at startup (a target with no
    /// matched threads or a yielded set included).
    pub(crate) fn managed(&self) -> bool {
        self.target.is_some() || self.active() || !self.yielded.is_empty()
    }
}

/// Releases every recorded thread (Service OFF / daemon exit).
pub(crate) fn release(st: &mut AffinityState) -> Result<bool, String> {
    if st.threads.is_empty() {
        let changed = st.target.is_some() || st.pid.is_some() || !st.yielded.is_empty();
        st.target = None;
        st.pid = None;
        st.starttime = None;
        st.yielded.clear();
        st.no_pid_logged = false;
        return Ok(changed);
    }
    let Some(host) = Host::detect() else {
        return Err("no /proc root (host simulation without SURYAPROFILER_PROC_ROOT)".into());
    };
    release_with(st, &host)
}

/// Startup reconciliation: undo a placement left by a crashed daemon.
pub(crate) fn reconcile(st: &mut AffinityState) -> bool {
    if !st.managed() {
        return false;
    }
    match release(st) {
        Ok(changed) => changed,
        Err(e) => {
            warn(format!("orphan release failed: {e}"));
            false
        }
    }
}

pub(crate) fn release_with(st: &mut AffinityState, host: &Host) -> Result<bool, String> {
    if st.threads.is_empty() {
        let changed = st.target.is_some() || st.pid.is_some() || !st.yielded.is_empty();
        st.target = None;
        st.pid = None;
        st.starttime = None;
        st.yielded.clear();
        st.no_pid_logged = false;
        return Ok(changed);
    }
    let target = st.target.clone();
    let before = st.threads.len();
    let mut keep = Vec::new();
    let mut released = 0usize;
    let mut first_err: Option<String> = None;
    for rec in st.threads.drain(..) {
        let Some((_, comm)) = proc::threads(host, rec.pid)
            .into_iter()
            .find(|(tid, _)| *tid == rec.tid)
        else {
            continue; // thread gone
        };
        if comm != rec.comm {
            continue; // tid reused
        }
        let Some(current) = proc::read_mask(host, rec.pid, rec.tid) else {
            continue;
        };
        if current != rec.new_mask {
            continue; // someone else changed it; nothing of ours to undo
        }
        // Restore the full captured mask first. If a narrowed cgroup now
        // rejects it, fall back to the intersection (or drop when empty).
        let restored = match proc::set_mask(host, rec.pid, rec.tid, rec.old_mask) {
            Ok(()) => true,
            Err(e) => {
                let clamped = rec.old_mask & current;
                let ok = if clamped == 0 {
                    true // nothing we can restore under the new cgroup
                } else {
                    clamped != rec.old_mask
                        && proc::set_mask(host, rec.pid, rec.tid, clamped).is_ok()
                };
                if !ok && first_err.is_none() {
                    first_err = Some(e);
                }
                ok
            }
        };
        if restored {
            released += 1;
        } else {
            keep.push(rec);
        }
    }
    st.threads = keep;
    if st.threads.is_empty() {
        st.target = None;
        st.pid = None;
        st.starttime = None;
        st.yielded.clear();
        st.no_pid_logged = false;
    }
    if released > 0 {
        info(format!(
            "released {released} thread(s) of {}",
            target.as_deref().unwrap_or("?")
        ));
    }
    let changed = released > 0 || st.threads.len() != before;
    match first_err {
        Some(e) => Err(e),
        None => Ok(changed),
    }
}
