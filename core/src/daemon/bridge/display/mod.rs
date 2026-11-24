//! Optional global display scale (`wm size` + `wm density`).
//!
//! Exact 20:9 presets only; density is part of the preset and controls the
//! dp layout (not performance). Default OFF, system-wide, restored on Service
//! OFF/exit. Per-app render scale still applies on top (product of buffers).

use super::SyncCtx;

mod presets;
mod state;
mod wm;

pub(crate) use state::{release, sync, DisplayState};

/// Sync from the daemon bridge context.
pub(crate) fn sync_ctx(st: &mut DisplayState, ctx: &SyncCtx) -> bool {
    sync(st, &ctx.display_scale)
}
