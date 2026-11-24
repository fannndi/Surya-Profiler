//! Daemon startup, thread wiring, main loop and clean-exit restore.
//!

use super::*;

mod exit;
mod threads;

/// Entry point for `surya-prof serve`.
pub fn run(state_dir: &Path, config_path: &Path) -> Result<(), String> {
    // Logging before the first chatter: level from SURYAPROFILER_LOG, file
    // sink under <state_dir>/logs/. The app's config may override below.
    crate::logging::init(Some(state_dir), crate::logging::level_from_env());
    let publisher = Publisher::new();
    publisher.emit(&Event::Hello {
        version: VERSION,
        pid: std::process::id(),
    });
    publisher.log(&format!("daemon {} starting", env!("CARGO_PKG_VERSION")));

    if let Err(e) = std::fs::create_dir_all(state_dir) {
        publisher.log(&format!("state dir: {e}"));
    }
    let _instance = match super::claim_instance(state_dir) {
        Ok(g) => g,
        Err(e) => {
            publisher.log(&format!("instance: {e}"));
            return Err(e);
        }
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(state_dir, std::fs::Permissions::from_mode(0o700));
    }

    let (mut cfg, cfg_err) = ConfigFile::load(config_path);
    if let Some(e) = cfg_err {
        publisher.log(&format!("config: {e}"));
    }
    cfg.reload_if_changed();
    crate::logging::set_level_str(cfg.get().log_level.as_deref());

    let flags = LiveFlags::spawn();
    let bridge = Bridge::recover(state_dir, publisher.clone(), Some(flags.clone()));
    let initial_active = Store::new(state_dir).load_state().active;

    // A SIGKILL cannot run the exit restore. Undo any thread placement left
    // by the previous run before the first decision; the first sync will
    // re-apply it if the game is still foreground and the switch is on.
    bridge.reconcile_orphan_affinity();

    // R&D input-suspend is app-only and stock 0. If a crash lost holds.json
    // while the node stayed at 1, release it before the first decision.
    if !cfg.get().rnd_input_suspend && !bridge.holds_info().input_suspend_held {
        bridge.reconcile_orphan_input_suspend();
    }

    // Crash recovery when the service was already disabled: `evaluate` will
    // not run, so a leftover saver/input-suspend/thermal hold would otherwise
    // survive forever. Release it once here; on failure keep the gate closed
    // (safe) and leave the repair to `surya-prof clean`.
    if !cfg.get().enabled && bridge.has_holds() {
        publisher.log("bridge: config disabled; releasing stale holds");
        let rep = bridge.release_all();
        if rep.ok() {
            bridge.resume();
        } else {
            publisher.log(&format!(
                "bridge: stale hold release failed: {:?}",
                rep.failures()
            ));
        }
    }

    let (msg_tx, msg_rx): (Sender<Msg>, Receiver<Msg>) = mpsc::channel();
    let (work_tx, work_rx) = mpsc::channel::<Work>();
    let (sync_tx, sync_rx) = mpsc::channel::<SyncCtx>();

    threads::spawn_stdin(msg_tx.clone(), publisher.clone())?;
    let worker_handle = threads::spawn_worker(
        state_dir.to_path_buf(),
        bridge.clone(),
        msg_tx.clone(),
        work_rx,
    )?;
    threads::spawn_bridge(bridge.clone(), sync_rx)?;

    // --- main loop -------------------------------------------------------------
    let mut rt = Runtime {
        publisher,
        work_tx,
        sync_tx,
        tx: msg_tx.clone(),
        config: cfg,
        flags,
        bridge,
        state_dir: state_dir.to_path_buf(),
        config_path: config_path.to_path_buf(),
        started: Instant::now(),
        profiles_mtime: mtime(&state_dir.join("profiles.json")),
        env: EnvSnapshot::default(),
        stats: Stats::load(state_dir),
        reconciled: false,
        watchdog_tick: 0,
        floor_audit: freq_audit::FloorAudit::new(),
        restoring: false,
        screen_on: true,
        locked: false,
        multi_window: false,
        second_window: None,
        mw_probe: Arc::new(AtomicBool::new(false)),
        last_fg: None,
        last_real: None,
        last_fg_at: None,
        active: initial_active,
        reason: None,
        src_pkg: None,
        sleep_deadline: None,
        fg_debounce: None,
        fg_watcher: None,
        last_watcher_restart: Instant::now(),
        last_state: Snapshot::default(),
    };
    rt.fg_watcher = watcher::spawn_fg(rt.tx.clone());
    if rt.fg_watcher.is_none() {
        rt.log("foreground watcher: failed to spawn logcat");
    }
    // read-only telemetry (battery / thermal / GPU busy) for diagnostics
    env::spawn(msg_tx.clone());
    rt.emit_state(true);

    let mut next_sup = Instant::now() + Duration::from_millis(SUPERVISE_MS);
    let mut next_cfg = Instant::now() + Duration::from_millis(CONFIG_POLL_MS);
    let mut ticks: u64 = 0;

    loop {
        let now = Instant::now();
        let mut wake = next_sup.min(next_cfg);
        if let Some(s) = rt.sleep_deadline {
            wake = wake.min(s);
        }
        if let Some(d) = rt.fg_debounce {
            wake = wake.min(d);
        }
        let timeout = wake.saturating_duration_since(now);

        match msg_rx.recv_timeout(timeout) {
            Ok(Msg::Command(cmd)) => {
                if !rt.on_command(cmd) {
                    break;
                }
            }
            Ok(Msg::Applied(ev)) => rt.on_applied(ev),
            Ok(Msg::Restored(ev)) => rt.on_restored(ev),
            Ok(Msg::Fg(pkg)) => rt.on_fg(pkg),
            Ok(Msg::Mw {
                active,
                other,
                started,
            }) => rt.on_mw(active, other, started),
            Ok(Msg::Peek {
                pkg,
                trigger,
                started,
            }) => rt.on_seed(pkg, trigger, started),
            Ok(Msg::Env(snap)) => rt.on_env(*snap),
            Ok(Msg::WatcherDown) => {
                rt.log("foreground stream ended");
            }
            Ok(Msg::ShutdownDone(rep)) => {
                rt.log(&format!(
                    "worker shutdown: ok={} restored={} wrote={} verified={} failed={} holds={:?}",
                    rep.ok, rep.restored, rep.wrote, rep.verified, rep.failed, rep.release
                ));
            }
            Ok(Msg::StdinClosed) => {
                rt.log("stdin closed, exiting");
                break;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        let now = Instant::now();
        // fg-debounce expiry: fire the deferred foreground evaluation
        if let Some(d) = rt.fg_debounce {
            if now >= d {
                rt.fg_debounce = None;
                if rt.screen_on && !rt.locked {
                    rt.evaluate("event", None);
                }
            }
        }
        if now >= next_sup {
            next_sup = now + Duration::from_millis(SUPERVISE_MS);
            ticks += 1;
            rt.supervise(ticks);
        }
        if now >= next_cfg {
            next_cfg = now + Duration::from_millis(CONFIG_POLL_MS);
            if rt.config.reload_if_changed() {
                rt.log("config reloaded (mtime)");
                crate::logging::set_level_str(rt.config.get().log_level.as_deref());
                let fg =
                    arbiter::decision_foreground(rt.last_fg.as_deref(), rt.last_real.as_deref());
                rt.evaluate("config", fg);
            }
            // profile-pack deploy (app upgrade): a changed profiles.json must
            // re-evaluate, or new catalog keys would wait for the next switch
            let pm = mtime(&rt.state_dir.join("profiles.json"));
            if pm != rt.profiles_mtime {
                rt.profiles_mtime = pm;
                if pm.is_some() {
                    rt.log("profiles reloaded (mtime)");
                    let fg = arbiter::decision_foreground(
                        rt.last_fg.as_deref(),
                        rt.last_real.as_deref(),
                    );
                    rt.evaluate("profiles", fg);
                }
            }
        }
        if let Some(s) = rt.sleep_deadline {
            if now >= s {
                rt.sleep_deadline = None;
                if !rt.screen_on {
                    rt.log("sleep timer fired");
                    rt.evaluate("sleep", None);
                }
            }
        }
    }

    if let Some(w) = rt.fg_watcher.as_mut() {
        w.stop();
    }
    // The worker owns every write; it quiesces, restores and reports.
    exit::shutdown_worker(&mut rt, &msg_rx, worker_handle);
    rt.publisher.emit(&Event::Bye);
    rt.publisher.shutdown();
    Ok(())
}
