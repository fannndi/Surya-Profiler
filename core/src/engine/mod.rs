//! Engine — the tuning core. Everything that decides or touches parameters.
//!
//! File index:
//! - `catalog/`    writable parameter registry + forbidden-path guard
//! - `probe.rs`    read-only device capture (values + options + evidence)
//! - `env.rs`      read-only telemetry (battery / thermal / GPU busy)
//! - `profile.rs`  profiles.json model + parser
//! - `plan.rs`     profile + probe -> ordered Plan (statuses, invariants)
//! - `validate.rs` per-kind value validation (clamps to real device options)
//! - `readback.rs` read-back comparison rules (harmony: framework wins)
//! - `apply/`      snapshot -> guarded writes -> verify; restore; drift
//! - `crypt.rs`    AES helper shared with the thermal takeover
//! - `thermal_*`   MIUI scenario/log parsers
//! - `testutil.rs` shared test fixtures (cfg(test) only)
//!
//! Invariant: every *profile* write passes `catalog::guard_path`; the
//! daemon's thermal takeover uses its own explicit thermal allowlist.

pub mod apply;
pub mod catalog;
pub mod crypt;
pub mod env;
pub mod plan;
pub mod probe;
pub mod profile;
pub mod readback;
pub mod thermal_log;
pub mod thermal_scenario;
pub mod validate;

#[cfg(test)]
pub(crate) mod testutil;
