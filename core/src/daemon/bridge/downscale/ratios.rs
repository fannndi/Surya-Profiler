//! AOSP per-app downscale ratios (GameManagerService `DOWNSCALE_*`).
//!
//! Android 12 accepts exactly these strings; `0.70` is invalid, `0.7` is the
//! canonical form. The UI can display percentages, the wire value stays here.

/// Canonical factors accepted by `cmd game downscale`.
pub(crate) const RATIOS: &[&str] = &[
    "0.3", "0.35", "0.4", "0.45", "0.5", "0.55", "0.6", "0.65", "0.7", "0.75", "0.8", "0.85", "0.9",
];

/// Change IDs from `CompatModePackages` (Android 12) mapped to factors.
pub(crate) const SCALE_IDS: &[(i64, &str)] = &[
    (189970040, "0.3"),
    (189969749, "0.35"),
    (189970038, "0.4"),
    (189969782, "0.45"),
    (176926741, "0.5"),
    (189970036, "0.55"),
    (176926771, "0.6"),
    (189969744, "0.65"),
    (176926829, "0.7"),
    (189969779, "0.75"),
    (176926753, "0.8"),
    (189969734, "0.85"),
    (182811243, "0.9"),
];

/// Gatekeeper change ID (`CompatModePackages.DOWNSCALED`).
pub(crate) const GATE_ID: i64 = 168419799;

/// Normalizes `0.70`/`0.7`/`70` to a canonical factor, if valid.
pub(crate) fn canonical(raw: &str) -> Option<&'static str> {
    let trimmed = raw.trim();
    let percent = trimmed.strip_suffix('%').map(str::trim);
    let mut value = match percent {
        Some(p) => p.parse::<f64>().ok().map(|v| v / 100.0),
        None => trimmed.parse::<f64>().ok(),
    }?;
    if value > 1.0 {
        value /= 100.0;
    }
    RATIOS.iter().copied().find(|r| {
        r.parse::<f64>()
            .map(|v| (v - value).abs() < 0.0005)
            .unwrap_or(false)
    })
}

/// Label for UI/logs: `0.65` -> `65%`.
pub(crate) fn label(ratio: &str) -> String {
    ratio
        .parse::<f64>()
        .map(|v| format!("{:.0}%", v * 100.0))
        .unwrap_or_else(|_| ratio.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_height_percent_and_percent_sign() {
        assert_eq!(canonical("0.7"), Some("0.7"));
        assert_eq!(canonical("0.70"), Some("0.7"));
        assert_eq!(canonical("70"), Some("0.7"));
        assert_eq!(canonical("65%"), Some("0.65"));
        assert_eq!(canonical(" 0.5 "), Some("0.5"));
        assert_eq!(canonical("0.6"), Some("0.6"));
        assert_eq!(canonical("not-a-ratio"), None);
        assert_eq!(canonical("1.0"), None);
        assert_eq!(canonical("0.42"), None);
    }

    #[test]
    fn every_ratio_has_one_scale_id() {
        for ratio in RATIOS {
            assert_eq!(
                SCALE_IDS.iter().filter(|(_, r)| r == ratio).count(),
                1,
                "{ratio}"
            );
        }
        assert_eq!(SCALE_IDS.len(), RATIOS.len());
        assert_eq!(label("0.65"), "65%");
        assert_eq!(label("0.7"), "70%");
    }
}
