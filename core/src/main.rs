//! `surya-prof` binary entry point. The CLI lives in `cli.rs`.

mod cli;

fn main() -> std::process::ExitCode {
    cli::main()
}
