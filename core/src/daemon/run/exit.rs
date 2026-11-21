//! Worker-owned exit cleanup (quiesce, restore, report).

use super::*;
use std::thread::JoinHandle;

pub(super) fn shutdown_worker(
    rt: &mut Runtime,
    msg_rx: &Receiver<Msg>,
    worker_handle: JoinHandle<()>,
) {
    let _ = rt.work_tx.send(worker::Work::Shutdown);
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut report: Option<worker::ShutdownReport> = None;
    while Instant::now() < deadline {
        let left = deadline.saturating_duration_since(Instant::now());
        match msg_rx.recv_timeout(left) {
            Ok(Msg::ShutdownDone(rep)) => {
                report = Some(rep);
                break;
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    // Join regardless: the worker returns immediately after sending its
    // report; if it is wedged, process exit below is the final backstop.
    let _ = worker_handle.join();
    match report {
        Some(rep) => rt.log(&format!(
            "exit cleanup: ok={} restored={} wrote={} verified={} failed={} holds={:?}",
            rep.ok, rep.restored, rep.wrote, rep.verified, rep.failed, rep.release
        )),
        None => rt.log("exit cleanup: worker did not report within 20s"),
    }
}
