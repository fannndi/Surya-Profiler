//! Exact 20:9 display presets for surya (1080x2400 physical panel).
//!
//! Density is part of the preset: it controls the dp layout, not performance.
//! `hd` keeps the vendor 720p density (320, larger/readable UI); `hd-compact`
//! uses the layout-preserving 267 (same 431dp as native 400/1080).

pub(crate) struct Preset {
    pub id: &'static str,
    pub width: u32,
    pub height: u32,
    pub density: u32,
}

pub(crate) const PRESETS: &[Preset] = &[
    Preset {
        id: "80",
        width: 864,
        height: 1920,
        density: 320,
    },
    Preset {
        id: "75",
        width: 810,
        height: 1800,
        density: 300,
    },
    Preset {
        id: "70",
        width: 756,
        height: 1680,
        density: 280,
    },
    Preset {
        id: "hd",
        width: 720,
        height: 1600,
        density: 320,
    },
    Preset {
        id: "hd-compact",
        width: 720,
        height: 1600,
        density: 267,
    },
    Preset {
        id: "60",
        width: 648,
        height: 1440,
        density: 240,
    },
    Preset {
        id: "50",
        width: 540,
        height: 1200,
        density: 200,
    },
];

/// `native`/`off`/empty -> `"off"`; valid ids are returned canonical.
pub(crate) fn canonical(raw: &str) -> Option<&'static str> {
    let t = raw.trim();
    if t.is_empty() || t.eq_ignore_ascii_case("off") || t.eq_ignore_ascii_case("native") {
        return Some("off");
    }
    PRESETS.iter().map(|p| p.id).find(|id| *id == t)
}

pub(crate) fn find(id: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.id == id)
}

/// Both dimensions must scale the 20:9 panel exactly.
#[cfg(test)]
pub(crate) fn is_exact_aspect(p: &Preset) -> bool {
    p.width * 20 == p.height * 9
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_preserve_the_20_9_aspect() {
        for p in PRESETS {
            assert!(is_exact_aspect(p), "{}", p.id);
            assert_eq!(p.width % 9, 0, "{}", p.id);
            assert!((160..=640).contains(&p.density), "{}", p.id);
        }
    }

    #[test]
    fn ids_are_canonical_and_off_is_supported() {
        assert_eq!(canonical("off"), Some("off"));
        assert_eq!(canonical("native"), Some("off"));
        assert_eq!(canonical(""), Some("off"));
        assert_eq!(canonical(" hd "), Some("hd"));
        assert_eq!(canonical("hd-compact"), Some("hd-compact"));
        assert_eq!(canonical("nope"), None);
        assert!(find("hd").is_some());
        assert!(find("nope").is_none());
    }

    #[test]
    fn hd_densities_match_the_r_and_d_choice() {
        assert_eq!(find("hd").unwrap().density, 320);
        assert_eq!(find("hd-compact").unwrap().density, 267);
        assert_eq!(find("80").unwrap().density, 320);
        assert_eq!(find("50").unwrap().density, 200);
    }
}
