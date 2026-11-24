//! Per-app render scale via Android 12's GameManagerService shell interface.
//!
//! `cmd game downscale <ratio> <pkg>` toggles the `CompatModePackages`
//! `DOWNSCALED`/`DOWNSCALE_xx` change IDs. The platform renders the app at a
//! uniformly smaller buffer and scales it back to the window: aspect ratio
//! and dp layout stay untouched (only DPR changes). This module captures the
//! prior override (MIUI Game Turbo may already set one) and restores it on
//! Service OFF / package removal.
//!
//! Scope: user apps only; ratios are the AOSP set. No sysfs/catalog writes.

use super::SyncCtx;

mod platform_compat;
mod ratios;
mod state;

pub(crate) use state::{release, sync, DownscaleState};

/// Sync from the daemon bridge context.
pub(crate) fn sync_ctx(st: &mut DownscaleState, ctx: &SyncCtx) -> bool {
    sync(st, &ctx.app_downscale, ctx.last_real.as_deref())
}
