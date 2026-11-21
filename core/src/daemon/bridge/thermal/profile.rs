//! Thermal profile registry and embedded plaintext configs.

pub(crate) const BALANCE_TXT: &str = include_str!("../../../../assets/thermal/thermal-balance.txt");
pub(crate) const GAME_TXT: &str = include_str!("../../../../assets/thermal/thermal-game.txt");
pub(crate) const POWERSAVE_TXT: &str =
    include_str!("../../../../assets/thermal/thermal-powersave.txt");
pub(crate) const SLEEP_TXT: &str = include_str!("../../../../assets/thermal/thermal-sleep.txt");

/// Private `sconfig` indices. MIUI uses 0–25 plus offsets +30/+100/+200/+500;
/// 60–63 is outside every value PowerKeeper 4.2.00 produces on surya and
/// stays below the mi_thermald map limit (IDs 0–64).
pub(crate) const CUSTOM_INDICES: [i32; 4] = [60, 61, 62, 63];
/// v0.5.0 preview indices (never released); kept only for cleanup/migration.
pub(crate) const PREVIEW_INDICES: [i32; 4] = [70, 71, 72, 73];
/// v0.4.x indices. Recognized only while the marker exists, for migration.
pub(crate) const LEGACY_INDICES: [i32; 4] = [20, 21, 22, 23];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ThermalProfile {
    Balance,
    Game,
    Powersave,
    Sleep,
}

impl ThermalProfile {
    pub(crate) fn from_id(id: &str) -> Option<Self> {
        match id {
            "balance" => Some(Self::Balance),
            "game" => Some(Self::Game),
            "powersave" => Some(Self::Powersave),
            "sleep" => Some(Self::Sleep),
            _ => None,
        }
    }

    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Balance => "balance",
            Self::Game => "game",
            Self::Powersave => "powersave",
            Self::Sleep => "sleep",
        }
    }

    pub(crate) fn sconfig(self) -> i32 {
        match self {
            Self::Balance => 60,
            Self::Game => 61,
            Self::Powersave => 62,
            Self::Sleep => 63,
        }
    }

    pub(crate) fn config_name(self) -> &'static str {
        match self {
            Self::Balance => "thermal-balance.conf",
            Self::Game => "thermal-game.conf",
            Self::Powersave => "thermal-powersave.conf",
            Self::Sleep => "thermal-sleep.conf",
        }
    }

    pub(crate) fn plaintext(self) -> &'static str {
        match self {
            Self::Balance => BALANCE_TXT,
            Self::Game => GAME_TXT,
            Self::Powersave => POWERSAVE_TXT,
            Self::Sleep => SLEEP_TXT,
        }
    }

    pub(crate) fn all() -> [Self; 4] {
        [Self::Balance, Self::Game, Self::Powersave, Self::Sleep]
    }
}
pub(crate) fn is_custom_index(value: i32) -> bool {
    CUSTOM_INDICES.contains(&value)
}

pub(crate) fn is_legacy_index(value: i32) -> bool {
    LEGACY_INDICES.contains(&value) || PREVIEW_INDICES.contains(&value)
}
