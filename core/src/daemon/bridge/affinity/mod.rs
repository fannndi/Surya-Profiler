//! Per-app thread placement (Fase 6, opt-in).
//!
//! While an app mapped to the **Game** profile is foreground, the daemon pins
//! the two critical render-path threads to the gold cores and restores the
//! original affinity when the app leaves, the screen turns off, the user
//! locks the phone, or the service stops.
//!
//! Scope is deliberately small (reference audit 2026-11-24, Scene sm6150):
//! only `UnityMain`/`GameThread` -> cpu6-7 and `UnityGfx*`/`RenderThread`/
//! `RHIThread` -> cpu6. Worker/audio threads are not moved. The feature is
//! off by default, and every write is `sched_setaffinity` through `taskset`
//! (no sysfs/catalog surface).
//!
//! Safety:
//! - only the exact package's main process (uid >= 10000) is touched;
//! - a thread mask must be a subset of its current `Cpus_allowed_list`;
//! - a thread that someone else re-pinned is never widened on restore, and
//!   is yielded for the rest of the app session (no re-pin fight);
//! - the captured per-tid mask is persisted in `affinity.json` for crash
//!   recovery, and the daemon reconciles orphans at startup.

use super::SyncCtx;
use crate::logging::{self, Level};
use proc::Host;

mod proc;
mod rules;
mod state;

#[cfg(test)]
pub(crate) use state::release_with;
pub(crate) use state::{reconcile, release, AffinityState, AppliedThread, YieldedThread};

#[cfg(test)]
mod tests;

/// The package we should place, if the world allows it.
pub(crate) fn want(ctx: &SyncCtx) -> Option<&str> {
    if !(ctx.thread_affinity && ctx.dynamic && ctx.screen_on && !ctx.locked && !ctx.multi_window) {
        return None;
    }
    if ctx.profile.as_deref() != Some("game") {
        return None;
    }
    let fg = ctx.last_real.as_deref()?;
    (ctx.app_map.get(fg).map(String::as_str) == Some("game")).then_some(fg)
}

/// One placement sync; returns true when state changed (needs a persist).
pub(crate) fn sync(st: &mut AffinityState, ctx: &SyncCtx) -> bool {
    let Some(host) = Host::detect() else {
        return false;
    };
    match want(ctx) {
        Some(pkg) => apply(st, pkg, &host),
        None => match state::release_with(st, &host) {
            Ok(changed) => changed,
            Err(e) => {
                warn(format!("release failed: {e}"));
                false
            }
        },
    }
}

fn apply(st: &mut AffinityState, pkg: &str, host: &Host) -> bool {
    let mut changed = false;
    if st.target.as_deref() != Some(pkg) {
        if let Err(e) = state::release_with(st, host) {
            warn(format!("{pkg}: old target release failed: {e}"));
            return false;
        }
        st.target = Some(pkg.to_string());
        st.pid = None;
        st.starttime = None;
        st.threads.clear();
        st.yielded.clear();
        st.no_pid_logged = false;
        changed = true;
    }

    let Some((pid, start)) = proc::resolve_pid(host, pkg) else {
        if !st.no_pid_logged {
            info(format!("{pkg}: no main process yet; waiting"));
            st.no_pid_logged = true;
        }
        return changed;
    };
    if st.pid != Some(pid) || st.starttime != Some(start) {
        st.threads.clear();
        st.yielded.clear();
        st.pid = Some(pid);
        st.starttime = Some(start);
        st.no_pid_logged = false;
        changed = true;
    }

    let threads = proc::threads(host, pid);
    let before = st.threads.len();
    st.threads.retain(|r| {
        threads
            .iter()
            .any(|(tid, comm)| *tid == r.tid && comm == &r.comm)
    });
    if st.threads.len() != before {
        changed = true;
    }

    let mut newly = Vec::new();
    let mut yielded = Vec::new();
    for (tid, comm) in &threads {
        let Some(mask) = rules::mask_for(comm) else {
            continue;
        };
        if st
            .yielded
            .iter()
            .any(|y| y.pid == pid && y.tid == *tid && &y.comm == comm)
        {
            continue;
        }
        let Some(current) = proc::read_mask(host, pid, *tid) else {
            continue;
        };
        if let Some(rec) = st.threads.iter().find(|r| r.tid == *tid) {
            if current == rec.new_mask {
                continue;
            }
            // The app/another writer took the thread back: stop managing it
            // for this session instead of fighting it every supervisor tick.
            yielded.push(comm.clone());
            st.threads.retain(|r| r.tid != *tid);
            st.yielded.push(YieldedThread {
                pid,
                tid: *tid,
                comm: comm.clone(),
            });
            changed = true;
            continue;
        }
        // The cpuset/cgroup may be narrower than our target: never widen.
        if mask & !current != 0 || current == mask {
            continue;
        }
        match proc::set_mask(host, pid, *tid, mask) {
            Ok(()) => {
                st.threads.push(AppliedThread {
                    pid,
                    starttime: start,
                    tid: *tid,
                    comm: comm.clone(),
                    old_mask: current,
                    new_mask: mask,
                });
                newly.push(format!("{comm}->{}", proc::format_list(mask)));
                changed = true;
            }
            Err(e) => warn(format!("{pkg}: {e}")),
        }
    }
    if !newly.is_empty() {
        info(format!("{pkg} (pid {pid}): pinned {}", newly.join(", ")));
    }
    if !yielded.is_empty() {
        info(format!(
            "{pkg}: app re-pinned {}; leaving it alone",
            yielded.join(", ")
        ));
    }
    changed
}

fn info(msg: String) {
    logging::write(Level::Info, "affinity", &msg);
}

fn warn(msg: String) {
    logging::write(Level::Warn, "affinity", &msg);
}
