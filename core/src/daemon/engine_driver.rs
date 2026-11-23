//! `EngineDriver` implementation: the in-process engine.
//!
//! The daemon IS the engine (same binary), so apply/restore are direct
//! function calls — no `su` round-trips, no CLI parsing.

use super::bridge::Bridge;
use super::worker::{EngineDriver, Outcome};
use crate::engine::apply::{self, Store};
use std::path::Path;
use std::sync::Arc;

pub struct EngineApplier {
    store: Store,
    bridge: Arc<Bridge>,
}

impl EngineApplier {
    pub fn new(state_dir: &Path, bridge: Arc<Bridge>) -> Self {
        EngineApplier {
            store: Store::new(state_dir),
            bridge,
        }
    }
}

impl EngineDriver for EngineApplier {
    fn active(&mut self) -> Option<String> {
        self.store.load_state().active
    }

    fn apply(&mut self, profile_id: &str, reconcile: bool) -> Outcome {
        let files = match self.store.load_profiles(None) {
            Ok(f) => f,
            Err(e) => return Outcome::err(e),
        };
        let Some(profile) = files.profiles.iter().find(|p| p.id == profile_id) else {
            return Outcome::err(format!("unknown profile '{profile_id}'"));
        };
        match apply::apply_with_pass2(&self.store, profile, reconcile) {
            Ok(rep) => {
                // Per-key detail for debugging: failed writes are warnings,
                // transient readback mismatches and locked keys stay Debug.
                for r in &rep.results {
                    if let Some(e) = &r.error {
                        crate::logging::write(
                            crate::logging::Level::Warn,
                            "apply",
                            &format!("{}: {e}", r.key),
                        );
                    } else if r.written && !r.verified {
                        crate::logging::write(
                            crate::logging::Level::Debug,
                            "apply",
                            &format!(
                                "{}: written={}, readback mismatch (transient)",
                                r.key, r.resolved
                            ),
                        );
                    }
                }
                if !rep.locked.is_empty() && crate::logging::enabled(crate::logging::Level::Debug) {
                    let list: Vec<String> = rep
                        .locked
                        .iter()
                        .map(|l| format!("{}: {}", l.key, l.reason))
                        .collect();
                    crate::logging::write(
                        crate::logging::Level::Debug,
                        "apply",
                        &format!("locked: {}", list.join("; ")),
                    );
                }
                // `locked` is not a failure by itself (optional/absent nodes),
                // but if every attempted write was rejected by the kernel,
                // the profile was never applied. Keep the host/simulation
                // "node missing" case successful; only hard rejection flips
                // `applied` to false.
                let hard_rejected = rep
                    .locked
                    .iter()
                    .any(|l| l.reason.starts_with("write rejected"));
                // Host simulations have no device nodes: an all-missing plan
                // is their normal success. On a real device an all-locked
                // profile is a broken pack/driver, not a successful apply.
                let applied = rep.wrote > 0
                    || rep.unchanged > 0
                    || (crate::engine::probe::allow_env_overrides() && !hard_rejected);
                Outcome {
                    ok: rep.ok && applied,
                    applied,
                    wrote: rep.wrote,
                    verified: rep.verified,
                    failed: rep.failed,
                    error: None,
                }
            }
            Err(e) => Outcome::err(e),
        }
    }

    fn release_holds(&mut self) -> crate::daemon::bridge::ReleaseReport {
        self.bridge.release_all()
    }

    fn restore(&mut self) -> Outcome {
        // Service off = true stock restore. The bridge holds were released by
        // the worker before this call; on failure resume so the live service
        // can keep working (the user can retry). Convergence lives in
        // `apply::restore`, not here.
        match apply::restore(&self.store) {
            Ok(rep) => {
                if !rep.ok {
                    self.bridge.resume();
                }
                Outcome {
                    ok: rep.ok,
                    applied: true,
                    wrote: rep.wrote,
                    verified: rep.verified,
                    failed: rep.failed,
                    error: None,
                }
            }
            Err(e) => {
                self.bridge.resume();
                Outcome::err(e)
            }
        }
    }
}
