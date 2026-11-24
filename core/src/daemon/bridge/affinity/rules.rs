//! Thread-name -> CPU-mask rules for surya (6 little + 2 big cores).
//!
//! Conservative on purpose: only the two critical render-path threads are
//! moved, everything else keeps whatever MIUI assigned. The masks come from
//! Scene's sm6150 Genshin mapping (2026-11-24 reference audit): `UnityMain`
//! gets cpu6-7, `UnityGfx`/render gets cpu6 so cpu7 stays free for the main
//! thread. `GameThread`/`RenderThread`/`RHIThread` extend the same split to
//! Unreal-style engines. Worker/audio threads are deliberately **not** moved
//! until an A/B run proves it helps.

/// cpu6-7 (both gold cores).
pub(crate) const BIG: u64 = 0xC0;
/// cpu6 only (reserve cpu7 for the main thread).
pub(crate) const BIG_RENDER: u64 = 0x40;

/// All cores present on surya; a mask outside this set is rejected.
#[cfg(test)]
pub(crate) const CPU_MASK: u64 = 0xFF;

/// Mask for a thread `comm` (truncated to 15 chars by the kernel).
pub(crate) fn mask_for(comm: &str) -> Option<u64> {
    match comm {
        "UnityMain" => Some(BIG),
        "GameThread" => Some(BIG),
        "RenderThread" | "RHIThread" => Some(BIG_RENDER),
        // `UnityGfxDeviceW` is the kernel-truncated `UnityGfxDeviceWorker`.
        _ if comm.starts_with("UnityGfx") => Some(BIG_RENDER),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_threads_get_a_mask() {
        assert_eq!(mask_for("UnityMain"), Some(BIG));
        assert_eq!(mask_for("GameThread"), Some(BIG));
        assert_eq!(mask_for("UnityGfxDeviceW"), Some(BIG_RENDER));
        assert_eq!(mask_for("UnityGfx"), Some(BIG_RENDER));
        assert_eq!(mask_for("RenderThread"), Some(BIG_RENDER));
        assert_eq!(mask_for("RHIThread"), Some(BIG_RENDER));
    }

    #[test]
    fn unrelated_threads_are_left_alone() {
        assert_eq!(mask_for("Binder:1234_1"), None);
        assert_eq!(mask_for("Worker Thread"), None);
        assert_eq!(mask_for("AudioTrack"), None);
        assert_eq!(mask_for("Thread-12"), None);
    }

    #[test]
    fn masks_stay_inside_surya_topology() {
        assert_eq!(BIG & !CPU_MASK, 0);
        assert_eq!(BIG_RENDER & !CPU_MASK, 0);
        assert_eq!(BIG_RENDER & !BIG, 0, "render mask must be a subset of big");
    }
}
