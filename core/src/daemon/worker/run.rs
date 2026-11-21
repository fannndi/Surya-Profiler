//! Worker loop implementation.

use super::*;

#[allow(clippy::too_many_arguments)]
pub fn run_with_shutdown(
    rx: Receiver<Work>,
    engine: &mut dyn EngineDriver,
    settle: Duration,
    retry: Duration,
    on_applied: &mut dyn FnMut(AppliedEvent),
    on_restored: &mut dyn FnMut(RestoredEvent),
    on_shutdown: &mut dyn FnMut(ShutdownReport),
) {
    while let Ok(first) = rx.recv() {
        match first {
            Work::Shutdown => {
                // Drop anything queued; the in-flight apply (if any) already
                // finished before this message was read.
                while let Ok(Work::Apply(_)) = rx.try_recv() {}
                finish(engine, on_shutdown);
                return;
            }
            Work::Restore => {
                // drop pending applies: a restore must be final
                while let Ok(Work::Apply(_)) = rx.try_recv() {}
                restore(engine, on_restored);
            }
            Work::Apply(first_job) => {
                let mut job = first_job;
                let batch_start = job.queued;

                // settle window: latest decision wins, no extension on new arrivals
                let deadline = Instant::now() + settle;
                let mut restore_deferred = false;
                let mut shutdown_deferred = false;
                loop {
                    let now = Instant::now();
                    if now >= deadline {
                        break;
                    }
                    match rx.recv_timeout(deadline - now) {
                        Ok(Work::Apply(next)) => {
                            // LATEST decision wins; a forced/reconcile job in
                            // the batch must not be swallowed by a plain
                            // one (startup screen-on + unlock race). An
                            // explicit user tap is sticky against foreground
                            // events until it has been applied.
                            if job.sticky && !next.sticky {
                                continue;
                            }
                            let was_force = job.force;
                            job = next;
                            job.force |= was_force;
                        }
                        Ok(Work::Restore) => {
                            restore_deferred = true;
                            break;
                        }
                        Ok(Work::Shutdown) => {
                            shutdown_deferred = true;
                            break;
                        }
                        Err(RecvTimeoutError::Timeout) => break,
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                }

                if shutdown_deferred {
                    finish(engine, on_shutdown);
                    return;
                }
                // a restore in the window cancels this batch (never apply
                // after a restore — it would re-tune a restored device).
                if restore_deferred {
                    restore(engine, on_restored);
                    continue;
                }

                let settle_ms = batch_start.elapsed().as_millis() as u64;
                let t0 = Instant::now();
                let mut out = run_once(engine, &job);
                let ms = if !out.ok && out.error.is_none() && out.failed > 0 {
                    // transient race with MIUI writes -> one retry, but keep
                    // observing the channel so a Restore/Apply arriving during
                    // the pause is not delayed (or stale-re-applied).
                    let retry_deadline = Instant::now() + retry;
                    let mut restored = false;
                    let mut shutdown = false;
                    while Instant::now() < retry_deadline {
                        let remaining = retry_deadline.saturating_duration_since(Instant::now());
                        let wait = remaining.min(Duration::from_millis(100));
                        match rx.recv_timeout(wait) {
                            Ok(Work::Restore) => {
                                restored = true;
                                break;
                            }
                            Ok(Work::Shutdown) => {
                                shutdown = true;
                                break;
                            }
                            Ok(Work::Apply(next)) => {
                                if job.sticky && !next.sticky {
                                    continue;
                                }
                                let was_force = job.force;
                                job = next;
                                job.force |= was_force;
                                break;
                            }
                            Err(RecvTimeoutError::Timeout) => continue,
                            Err(RecvTimeoutError::Disconnected) => break,
                        }
                    }
                    if shutdown {
                        finish(engine, on_shutdown);
                        return;
                    }
                    if restored {
                        restore(engine, on_restored);
                        // skip the on_applied for the original job
                        continue;
                    }
                    // Retry the (possibly superseding) job immediately.
                    let t1 = Instant::now();
                    out = run_once(engine, &job);
                    t1.elapsed().as_millis() as u64
                } else {
                    t0.elapsed().as_millis() as u64
                };
                on_applied(AppliedEvent {
                    profile: job.profile.clone(),
                    reason: reason_text(job.used_saver, &job.reason),
                    src_pkg: job.src_pkg.clone(),
                    ok: out.ok,
                    applied: out.applied,
                    wrote: out.wrote,
                    verified: out.verified,
                    failed: out.failed,
                    ms,
                    settle_ms,
                    watchdog: job.watchdog,
                });
            }
        }
    }
}

fn restore(engine: &mut dyn EngineDriver, on_restored: &mut dyn FnMut(RestoredEvent)) {
    let release = engine.release_holds();
    let out = engine.restore();
    on_restored(RestoredEvent {
        ok: out.ok && release.ok(),
        wrote: out.wrote,
        verified: out.verified,
        failed: out.failed,
        release,
    });
}

/// Exit cleanup. The worker is the only writer, so this is the only place
/// allowed to release holds + restore stock on shutdown. `engine.active()`
/// reads the persisted state, not a possibly-stale main-loop mirror.
fn finish(engine: &mut dyn EngineDriver, on_shutdown: &mut dyn FnMut(ShutdownReport)) {
    let active = engine.active();
    let release = engine.release_holds();
    let out = if active.is_some() {
        engine.restore()
    } else {
        Outcome {
            ok: true,
            applied: false,
            wrote: 0,
            verified: 0,
            failed: 0,
            error: None,
        }
    };
    on_shutdown(ShutdownReport {
        ok: out.ok && release.ok(),
        restored: active.is_some(),
        wrote: out.wrote,
        verified: out.verified,
        failed: out.failed,
        release,
    });
}

fn run_once(engine: &mut dyn EngineDriver, job: &Job) -> Outcome {
    let active = engine.active();
    let changing = active.as_deref() != Some(job.profile.as_str());
    if !job.force && !changing {
        return Outcome {
            ok: true,
            applied: true,
            wrote: 0,
            verified: 0,
            failed: 0,
            error: None,
        };
    }
    // Reconcile on a force AND on every profile change: without the stock
    // merge, keys owned by the OLD profile (e.g. core_ctl max_cpus left at
    // the sleep value) survive into the new profile's session.
    engine.apply(&job.profile, job.force || changing)
}

/// Raw reason labels the app resolves for display ("app" carries src_pkg).
pub(super) fn reason_text(used_saver: bool, reason: &str) -> String {
    if used_saver && reason == "base" {
        "MIUI saver".into()
    } else {
        reason.to_string()
    }
}
