//! SuryaProfiler core — MIUI-harmonized profile switcher.
//!
//! Architecture: this crate is the single writer of tuning parameters.
//! The Kotlin app drives it two ways: one-shot CLI commands (`plan`, `apply`,
//! ...) and the `serve` stdio daemon (JSON-lines) used by the service.
//!
//! Module index:
//! - `engine/`   planning + apply/verify/restore (see engine/mod.rs)
//! - `daemon/`   stdio daemon (JSON-lines; see daemon/mod.rs)
//! - `diagnose/` read-only environment self-check (`doctor`)
//! - `logging.rs` levelled ring/file logger (daemon + CLI)
//!
//! Ownership model (see `engine::catalog`): FREE + BOOT-BASELINE parameters
//! only; runtime-framework nodes are rejected even if a profile names them.

pub mod daemon;
pub mod diagnose;
pub mod engine;
pub mod logging;
pub mod platform;

/// Default profile set, bundled in the binary and mirrored in assets.
pub const DEFAULT_PROFILES_JSON: &str = include_str!("../profiles.json");
