//! Read-only power sampler. The daemon owns all aggregation and persistence.

mod aggregate;
mod history;
mod model;
mod store;

#[cfg(test)]
mod tests;

use model::probe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub use history::PowerHistory;
pub use model::{
    now_ms, PowerBucket, PowerSample, PowerSnapshot, PowerTick, DEFAULT_WINDOW_S, MAX_WINDOW_S,
    MIN_WINDOW_S,
};

struct SamplerState {
    stopped: bool,
    generation: u64,
}

type SharedSampler = Arc<(Mutex<SamplerState>, Condvar)>;

/// Joinable sampler handle. `wake` interrupts the current cadence sleep so a
/// screen-state change takes effect immediately; `stop` joins before flush.
pub struct PowerSampler {
    shared: SharedSampler,
    thread: Option<JoinHandle<()>>,
}

impl PowerSampler {
    pub fn wake(&self) {
        let (lock, cv) = &*self.shared;
        let mut state = lock.lock().unwrap_or_else(|p| p.into_inner());
        state.generation = state.generation.wrapping_add(1);
        cv.notify_one();
    }

    pub fn stop(&mut self) {
        let (lock, cv) = &*self.shared;
        {
            let mut state = lock.lock().unwrap_or_else(|p| p.into_inner());
            state.stopped = true;
            cv.notify_one();
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Fast sampler: 3 s screen-on / 30 s screen-off by default. Both cadences
/// are test-overridable. Actual elapsed time travels with every tick.
pub fn spawn(tx: Sender<super::Msg>, screen: Arc<AtomicBool>) -> PowerSampler {
    let shared: SharedSampler = Arc::new((
        Mutex::new(SamplerState {
            stopped: false,
            generation: 0,
        }),
        Condvar::new(),
    ));
    let worker_state = Arc::clone(&shared);
    let probe = probe();
    let thread = std::thread::Builder::new()
        .name("power".into())
        .spawn(move || {
            let mut last_capture: Option<Instant> = None;
            let mut last_generation = 0;
            loop {
                let (lock, cv) = &*worker_state;
                let generation = {
                    let state = lock.lock().unwrap_or_else(|p| p.into_inner());
                    if state.stopped {
                        break;
                    }
                    state.generation
                };
                let interval = if screen.load(Ordering::Relaxed) {
                    sample_ms()
                } else {
                    sample_off_ms()
                };
                let reading = probe.read();
                let captured = Instant::now();
                let tick = PowerTick {
                    reading,
                    interval_ms: interval,
                    captured_at_ms: now_ms(),
                    elapsed_ms: if last_capture.is_none() || generation != last_generation {
                        0
                    } else {
                        last_capture
                            .map(|last| captured.duration_since(last).as_millis() as u64)
                            .unwrap_or(0)
                    },
                };
                last_capture = Some(captured);
                last_generation = generation;
                if tx.send(super::Msg::Power(tick)).is_err() {
                    break;
                }
                let state = lock.lock().unwrap_or_else(|p| p.into_inner());
                if !state.stopped && state.generation == generation {
                    let _ = cv.wait_timeout_while(state, Duration::from_millis(interval), |s| {
                        !s.stopped && s.generation == generation
                    });
                }
            }
        })
        .ok();
    PowerSampler { shared, thread }
}

pub fn sample_ms() -> u64 {
    env_ms("SURYAPROFILER_POWER_SAMPLE_MS", 3_000).max(50)
}

pub fn sample_off_ms() -> u64 {
    env_ms("SURYAPROFILER_POWER_SAMPLE_OFF_MS", 30_000).max(100)
}

fn env_ms(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
