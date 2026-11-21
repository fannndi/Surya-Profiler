//! The writable catalog table for surya (POCO X3 NFC, MIUI 14).
//!
//! Data only: types + guard live in `mod.rs`, forbidden lists in
//! `forbidden.rs`. Every entry here must pass `guard_path`.
//!
//! Split by domain so adding a parameter is a small, local edit:
//! `cpu`, `gpu_io`, `kernel`, `misc`.

#[macro_use]
mod macros;

mod cpu;
mod gpu_io;
mod kernel;
mod misc;

use super::Entry;
use std::sync::OnceLock;

/// The full writable catalog for surya (POCO X3 NFC, MIUI 14).
pub fn catalog() -> &'static [Entry] {
    static ENTRIES: OnceLock<Vec<Entry>> = OnceLock::new();
    ENTRIES.get_or_init(|| {
        let mut out = Vec::new();
        out.extend(cpu::entries());
        out.extend(gpu_io::entries());
        out.extend(kernel::entries());
        out.extend(misc::entries());
        out
    })
}
