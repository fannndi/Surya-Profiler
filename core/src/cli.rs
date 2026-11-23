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
    /// `log` only: print the last N lines (default all).
    last: Option<usize>,
    /// `log` only: keep tailing the file.
    follow: bool,
    /// `log` only: minimum level to print.
    level: Option<String>,
}

fn parse_args(argv: &[String]) -> Result<Args, String> {
    let mut cmd: Option<String> = None;
    let mut id: Option<String> = None;
    let mut state_dir = apply::default_state_dir();
    let mut profiles: Option<PathBuf> = None;
    let mut config: Option<PathBuf> = None;
    let mut fix_thermal = false;
    let mut last: Option<usize> = None;
    let mut follow = false;
    let mut level: Option<String> = None;
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
            "--last" => {
                i += 1;
                let v = argv.get(i).ok_or("--last needs a value")?;
                last = Some(v.parse().map_err(|_| "--last needs a number")?);
            }
            "--follow" => follow = true,
            "--level" => {
                i += 1;
                let v = argv.get(i).ok_or("--level needs a value")?;
                level = Some(v.clone());
            }
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
        last,
        follow,
        level,
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
  debug env             read-only live freq/thermal report (floor drift)
  log [--last N] [--follow] [--level debug]
                        daemon log file (read-only)
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

/// `surya-prof log [--last N] [--follow] [--level debug]` — print the
/// daemon's log file (read-only; the newest rotation `.log.1` first).
fn run_log(args: &Args) -> ExitCode {
    use std::io::BufRead;
    let min_level = args
        .level
        .as_deref()
        .and_then(surya_profiler_core::logging::parse_level);
    let keep = |line: &str| -> bool {
        match min_level {
            // line format: `<date> <time> <tag> [component] message`
            Some(l) => line
                .split_whitespace()
                .nth(2)
                .and_then(|t| t.chars().next())
                .map(|tag| match tag {
                    'E' => surya_profiler_core::logging::Level::Error,
                    'W' => surya_profiler_core::logging::Level::Warn,
                    'D' => surya_profiler_core::logging::Level::Debug,
                    _ => surya_profiler_core::logging::Level::Info,
                })
                .is_some_and(|lv| lv <= l),
            None => true,
        }
    };
    let print_all = |lines: Vec<String>| -> usize {
        let kept: Vec<&String> = lines.iter().filter(|l| keep(l)).collect();
        let skip = kept.len().saturating_sub(args.last.unwrap_or(usize::MAX));
        for l in &kept[skip..] {
            println!("{l}");
        }
        kept.len() - skip
    };
    let mut files = vec![
        surya_profiler_core::logging::log_path(&args.state_dir),
        surya_profiler_core::logging::log_path(&args.state_dir).with_extension("log.1"),
    ];
    files.reverse(); // oldest generation first
    let mut shown = 0usize;
    for f in &files {
        if let Ok(fh) = std::fs::File::open(f) {
            shown = print_all(
                std::io::BufReader::new(fh)
                    .lines()
                    .map_while(Result::ok)
                    .collect(),
            );
        }
    }
    if shown == 0 {
        eprintln!(
            "no log yet at {} (the daemon writes it while the service runs)",
            surya_profiler_core::logging::log_path(&args.state_dir).display()
        );
        return ExitCode::from(1);
    }
    if args.follow {
        let path = surya_profiler_core::logging::log_path(&args.state_dir);
        let mut pos = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let Ok(fh) = std::fs::File::open(&path) else {
                continue;
            };
            let len = fh.metadata().map(|m| m.len()).unwrap_or(0);
            if len < pos {
                pos = 0; // rotated
            }
            use std::io::{Seek, SeekFrom};
            let mut fh = fh;
            if fh.seek(SeekFrom::Start(pos)).is_err() {
                continue;
            }
            let mut delta = String::new();
            let read = std::io::Read::read_to_string(&mut fh, &mut delta).unwrap_or(0);
            pos += read as u64;
            for l in delta.lines().filter(|l| keep(l)) {
                println!("{l}");
            }
        }
    }
    ExitCode::SUCCESS
}

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
        "log" => {
            // human text output (not JSON): tail/follow the daemon log file
            run_log(&args)
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
