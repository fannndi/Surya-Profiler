//! `surya-prof` — CLI entry point.
//!
//! Commands: probe | profiles | plan | apply | restore | verify | status | serve | doctor | thermal-clean
//! Machine consumers (the Kotlin app) pass `--json`.
//!
//! `serve` is special: it does not return a JSON payload on exit — it runs
//! the stdio daemon (JSON-lines protocol) until stdin closes. See `daemon/`.

use std::path::PathBuf;
use std::process::ExitCode;
use surya_profiler_core::daemon;
use surya_profiler_core::diagnose;
use surya_profiler_core::engine::apply::{self, Store};
use surya_profiler_core::engine::catalog;
use surya_profiler_core::engine::plan::build_plan;
use surya_profiler_core::engine::probe;

struct Args {
    cmd: String,
    id: Option<String>,
    state_dir: PathBuf,
    profiles: Option<PathBuf>,
    /// `serve` only: config.json the app owns (default: <state-dir>/config.json).
    config: Option<PathBuf>,
    /// `doctor` only: remove foreign thermal ships after the scan.
    fix_thermal: bool,
}

fn parse_args(argv: &[String]) -> Result<Args, String> {
    let mut cmd: Option<String> = None;
    let mut id: Option<String> = None;
    let mut state_dir = apply::default_state_dir();
    let mut profiles: Option<PathBuf> = None;
    let mut config: Option<PathBuf> = None;
    let mut fix_thermal = false;
    let mut i = 0;
    while i < argv.len() {
        let a = &argv[i];
        match a.as_str() {
            "--json" => {} // JSON is the only output format (accepted for compatibility)
            "--state-dir" => {
                i += 1;
                let v = argv.get(i).ok_or("--state-dir needs a value")?;
                state_dir = PathBuf::from(v);
            }
            "--profiles" => {
                i += 1;
                let v = argv.get(i).ok_or("--profiles needs a value")?;
                profiles = Some(PathBuf::from(v));
            }
            "--config" => {
                i += 1;
                let v = argv.get(i).ok_or("--config needs a value")?;
                config = Some(PathBuf::from(v));
            }
            "--fix-thermal" => fix_thermal = true,
            "--help" | "-h" => {
                return Err(String::new()); // usage, not an error
            }
            other if other.starts_with('-') => return Err(format!("unknown flag {other}")),
            other => {
                if cmd.is_none() {
                    cmd = Some(other.to_string());
                } else if id.is_none() {
                    id = Some(other.to_string());
                } else {
                    return Err(format!("unexpected argument {other}"));
                }
            }
        }
        i += 1;
    }
    Ok(Args {
        cmd: cmd.ok_or_else(|| USAGE.to_string())?,
        id,
        state_dir,
        profiles,
        config,
        fix_thermal,
    })
}

const USAGE: &str =
    "usage: surya-prof <command> [--json] [--state-dir DIR] [--profiles FILE] [--config FILE]
commands:
  probe                 read device state (read-only)
  profiles              list bundled/available profiles
  plan <id>             validate a profile against the live device
  apply <id>            snapshot + write + verify (root)
  restore               write snapshot back + release all holds (root)
  clean                 release saver/input-suspend/thermal holds (root)
  verify <id>           check live values vs profile, detect drift
  status                active profile, snapshot, framework evidence
  serve                 stdio daemon (JSON-lines on stdin/stdout)
  doctor [--fix-thermal]
                        environment self-check; --fix-thermal also removes
                        foreign thermal ships (root, daemon must be stopped)
  thermal-clean         remove an orphaned Surya thermal takeover (root)
  catalog               dump the full parameter catalog as JSON
  apply runs one automatic re-plan pass when a governor switch reveals
                      previously hidden governor-specific nodes";

fn is_root() -> bool {
    static CACHED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *CACHED.get_or_init(|| {
        std::fs::read_to_string("/proc/self/status")
            .map(|s| {
                s.lines()
                    .find(|l| l.starts_with("Uid:"))
                    .and_then(|l| l.split_whitespace().nth(2))
                    .map(|euid| euid == "0")
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    })
}

fn require_root() -> Result<(), String> {
    // Host tests/simulations set SURYAPROFILER_DEBUG=1 and a fake sysfs root;
    // on device this is never set, so the root gate remains real there.
    let simulated = std::env::var("SURYAPROFILER_DEBUG").ok().as_deref() == Some("1");
    if is_root() || simulated {
        Ok(())
    } else {
        Err("root required for this command".into())
    }
}

mod commands;

pub fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if argv.iter().any(|a| a == "--version" || a == "-V") {
        println!("surya-prof {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    let args = match parse_args(&argv) {
        Ok(a) => a,
        Err(e) => {
            if e.is_empty() || e == USAGE {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };

    match args.cmd.as_str() {
        "serve" => {
            // stdio daemon: streams JSON-lines until stdin closes; never
            // prints a one-shot payload (see daemon/mod.rs).
            let config = args
                .config
                .clone()
                .unwrap_or_else(|| args.state_dir.join("config.json"));
            match surya_profiler_core::daemon::run(&args.state_dir, &config) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::from(1)
                }
            }
        }
        _ => match commands::run(&args) {
            Ok((code, payload)) => {
                // JSON is the canonical format for every command (the app parses
                // it; humans get the same pretty-printed payload).
                println!("{payload}");
                ExitCode::from(code.clamp(0, 255) as u8)
            }
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::from(1)
            }
        },
    }
}
