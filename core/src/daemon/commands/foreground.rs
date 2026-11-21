//! Foreground/peek plumbing (inherent impl for `Runtime`).

use super::*;

impl Runtime {
    /// Foreground package event (the daemon's own watcher).
    pub(crate) fn on_fg(&mut self, pkg: String) {
        if !arbiter::is_transient(Some(&pkg)) {
            self.last_real = Some(pkg.clone());
        }
        if self.last_fg.as_deref() == Some(pkg.as_str()) {
            return;
        }
        self.last_fg = Some(pkg);
        self.last_fg_at = Some(Instant::now());
        self.emit_state(false);
        // A split transition may not produce a foreground event; probe the
        // authoritative window stack on every real foreground change.
        self.peek_mw_now();
        if self.screen_on && !self.locked {
            // Debounce: a rapid burst of foreground events (launcher flashes
            // between two registered apps) coalesces into one smooth switch.
            let debounce = std::env::var("SURYAPROFILER_DEBOUNCE_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(crate::daemon::FG_DEBOUNCE_MS);
            self.fg_debounce = Some(Instant::now() + Duration::from_millis(debounce));
        }
    }

    /// Wake/unlock seed from the daemon's own peek (None = nothing found).
    /// `triggers`: "unlock" always evaluates (wake re-asserts the decision);
    /// "peek" (stream-down fallback) only evaluates when the package changed.
    pub(crate) fn on_seed(&mut self, peeked: Option<String>, trigger: &str, started: Instant) {
        if self.last_fg_at.is_some_and(|live| started < live) {
            self.log("seed: stale peek ignored (a live event arrived)");
            return;
        }
        // A real peeked app (including Surya Profiler) is authoritative;
        // only an overlay falls back to the app underneath.
        let best = arbiter::decision_foreground(peeked.as_deref(), self.last_real.as_deref());
        self.log(&format!("seed: peeked={peeked:?} best={best:?}"));
        let changed = best.is_some() && best != self.last_real;
        if let Some(b) = best {
            // The seeded app is the daemon's best real foreground; keep the
            // saver follow (which reads last_real) in sync after wake/restart.
            self.last_real = Some(b.clone());
            self.last_fg = Some(b);
        }
        self.emit_state(false);
        if trigger == "unlock" || changed {
            self.evaluate(trigger, None);
        }
    }

    /// Multi-window state change from the `dumpsys window` probe.
    pub(crate) fn on_mw(&mut self, active: bool, other: Option<String>, started: Instant) {
        if self.last_fg_at.is_some_and(|live| started < live) {
            self.log("mw: stale split probe ignored (a live event arrived)");
            return;
        }
        if self.multi_window != active || self.second_window != other {
            self.multi_window = active;
            self.second_window = other.clone();
            let msg = if active {
                format!("multi-window ON ({})", other.unwrap_or_default())
            } else {
                "multi-window off".into()
            };
            self.log(&format!("bridge: {msg}"));
            self.emit_state(false);
            self.evaluate("multiwindow", None);
        }
    }

    /// One-shot split-screen probe on a short thread (never blocks the main
    /// loop). At most one in flight; the thread clears the flag on exit.
    pub(crate) fn peek_mw_now(&mut self) {
        if self.mw_probe.swap(true, Ordering::AcqRel) {
            return;
        }
        let tx = self.tx.clone();
        let flag = self.mw_probe.clone();
        let _ = std::thread::Builder::new()
            .name("mw-probe".into())
            .spawn(move || {
                let started = Instant::now();
                let res = watcher::peek_mw();
                flag.store(false, Ordering::Release);
                if let Some(res) = res {
                    match res {
                        Some((_primary, secondary)) => {
                            let _ = tx.send(Msg::Mw {
                                active: true,
                                other: Some(secondary),
                                started,
                            });
                        }
                        None => {
                            let _ = tx.send(Msg::Mw {
                                active: false,
                                other: None,
                                started,
                            });
                        }
                    }
                }
            });
    }

    /// One-shot foreground peek on a short thread (never blocks the loop).
    pub(crate) fn peek_now(&mut self, trigger: &'static str) {
        let tx = self.tx.clone();
        let _ = std::thread::Builder::new()
            .name("peek".into())
            .spawn(move || {
                let started = Instant::now();
                let pkg = watcher::peek_fg();
                let _ = tx.send(Msg::Peek {
                    pkg,
                    trigger,
                    started,
                });
            });
    }
}
