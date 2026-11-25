//! Serialized JSON-line writer shared by every daemon thread.
//!
//! A dedicated writer thread owns stdout. Producers use `try_send`: if the
//! app stops reading and the bounded queue fills, events are dropped and
//! logged instead of freezing the daemon's decision loop. `shutdown` flushes
//! the queue before process exit.

use super::Event;
use std::io::Write;
use std::sync::Arc;

enum OutMsg {
    Line(String),
    Shutdown(std::sync::mpsc::SyncSender<()>),
}

pub struct Publisher {
    tx: std::sync::mpsc::SyncSender<OutMsg>,
}

impl Publisher {
    pub fn new() -> Arc<Self> {
        let (tx, rx) = std::sync::mpsc::sync_channel::<OutMsg>(256);
        let spawned = std::thread::Builder::new()
            .name("stdout-writer".into())
            .spawn(move || {
                let mut out = std::io::stdout();
                while let Ok(msg) = rx.recv() {
                    match msg {
                        OutMsg::Line(line) => {
                            if writeln!(out, "{line}").is_err() {
                                break;
                            }
                            let _ = out.flush();
                        }
                        OutMsg::Shutdown(ack) => {
                            let _ = out.flush();
                            let _ = ack.send(());
                            break;
                        }
                    }
                }
            });
        if let Err(e) = spawned {
            let _ = writeln!(std::io::stderr(), "publisher: writer thread failed: {e}");
        }
        Arc::new(Publisher { tx })
    }

    pub fn emit(&self, ev: &Event) {
        self.emit_with_wait(ev, 0);
    }

    /// Command replies / restore results must not be silently dropped by the
    /// bounded queue. Wait briefly, then give up (never block forever).
    pub fn emit_reliable(&self, ev: &Event) {
        self.emit_with_wait(ev, 2_000);
    }

    fn emit_with_wait(&self, ev: &Event, wait_ms: u64) {
        let line = match serde_json::to_string(ev) {
            Ok(l) => l,
            Err(e) => {
                self.log(&format!("event serialize failed: {e}"));
                return;
            }
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(wait_ms);
        loop {
            match self.tx.try_send(OutMsg::Line(line.clone())) {
                Ok(()) => return,
                Err(std::sync::mpsc::TrySendError::Full(_)) => {
                    if std::time::Instant::now() >= deadline {
                        self.log("publisher: stdout backpressure; dropped event");
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(std::sync::mpsc::TrySendError::Disconnected(_)) => return,
            }
        }
    }

    /// Flushes queued events and stops the writer thread. Call once at exit.
    pub fn shutdown(&self) {
        let (ack_tx, ack_rx) = std::sync::mpsc::sync_channel(0);
        let mut msg = Some(OutMsg::Shutdown(ack_tx));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            match self
                .tx
                .try_send(msg.take().expect("shutdown message is present"))
            {
                Ok(()) => {
                    let _ = ack_rx.recv_timeout(std::time::Duration::from_secs(2));
                    return;
                }
                Err(std::sync::mpsc::TrySendError::Full(m)) => {
                    if std::time::Instant::now() >= deadline {
                        return;
                    }
                    msg = Some(m);
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(std::sync::mpsc::TrySendError::Disconnected(_)) => return,
            }
        }
    }

    /// Human log line: levelled logger (ring + file) with a stderr mirror
    /// that the app relays to logcat.
    ///
    /// Never use `eprintln!`: the crate is built with `panic = "abort"`, and
    /// an app death closes stderr at the same time as stdin — an EPIPE panic
    /// would abort before the exit restore. A failed log write is ignored.
    pub fn log(&self, msg: &str) {
        crate::logging::write(crate::logging::Level::Info, "daemon", msg);
    }
}
