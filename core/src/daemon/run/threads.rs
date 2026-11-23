//! Thread wiring: stdin reader, apply worker, bridge sync.
//!
//! `run.rs` keeps startup + the main loop; these are the spawned helpers.
//! Every spawn failure aborts startup with a descriptive error — half a
//! daemon (no stdin, no worker) would be worse than none.

use super::*;

/// Reads JSON-lines from stdin; EOF (app death) becomes `Msg::StdinClosed`.
pub(super) fn spawn_stdin(tx: Sender<Msg>, pubr: Arc<Publisher>) -> Result<(), String> {
    std::thread::Builder::new()
        .name("stdin".into())
        .spawn(move || {
            use std::io::BufRead;
            let stdin = std::io::stdin();
            for line in stdin.lock().lines() {
                let Ok(line) = line else { break };
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<Command>(&line) {
                    Ok(cmd) => {
                        if tx.send(Msg::Command(cmd)).is_err() {
                            break;
                        }
                    }
                    Err(e) => pubr.log(&format!("ipc: bad command: {e}")),
                }
            }
            let _ = tx.send(Msg::StdinClosed);
        })
        .map(|_| ())
        .map_err(|e| format!("stdin thread: {e}"))
}

/// The apply worker owns every write (apply/restore/exit cleanup).
pub(super) fn spawn_worker(
    state_dir: PathBuf,
    bridge: Arc<Bridge>,
    tx: Sender<Msg>,
    work_rx: Receiver<Work>,
) -> Result<std::thread::JoinHandle<()>, String> {
    std::thread::Builder::new()
        .name("apply-worker".into())
        .spawn(move || {
            let mut engine = engine_driver::EngineApplier::new(&state_dir, bridge);
            let mut on_applied = |ev: worker::AppliedEvent| {
                let _ = tx.send(Msg::Applied(ev));
            };
            let mut on_restored = |ev: RestoredEvent| {
                let _ = tx.send(Msg::Restored(ev));
            };
            let mut on_shutdown = |rep: worker::ShutdownReport| {
                let _ = tx.send(Msg::ShutdownDone(rep));
            };
            worker::run_with_shutdown(
                work_rx,
                &mut engine,
                Duration::from_millis(worker::settle_ms()),
                Duration::from_millis(worker::RETRY_MS),
                &mut on_applied,
                &mut on_restored,
                &mut on_shutdown,
            );
        })
        .map_err(|e| format!("worker thread: {e}"))
}

/// Bridge sync thread (settings IO; never blocks decisions). Latest wins:
/// pending contexts are drained before syncing, so a burst of decisions
/// costs ONE settings read/write round.
pub(super) fn spawn_bridge(bridge: Arc<Bridge>, sync_rx: Receiver<SyncCtx>) -> Result<(), String> {
    std::thread::Builder::new()
        .name("bridge".into())
        .spawn(move || {
            while let Ok(first) = sync_rx.recv() {
                let mut latest = first;
                while let Ok(next) = sync_rx.try_recv() {
                    latest = next;
                }
                bridge.sync(&latest);
            }
        })
        .map(|_| ())
        .map_err(|e| format!("bridge thread: {e}"))
}
