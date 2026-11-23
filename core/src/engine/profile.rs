//! profiles.json model + parser.
//!
//! Responsibility: parse + shape validation of the profile pack.
//! Non-goals: value validation (`validate.rs`), plan building (`plan.rs`).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub desc: String,
    /// One-line row subtitle (data-driven; UI must not duplicate it).
    #[serde(default)]
    pub short: String,
    /// Hidden from the home cards (e.g. screen-off `sleep`).
    #[serde(default)]
    pub hidden: bool,
    pub params: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilesFile {
    pub schema: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub device: String,
    #[serde(default)]
    pub rom: String,
    pub profiles: Vec<Profile>,
}

pub fn parse_profiles(json: &str) -> Result<ProfilesFile, String> {
    let file: ProfilesFile =
        serde_json::from_str(json).map_err(|e| format!("profiles.json: {e}"))?;
    if file.schema != 1 {
        return Err(format!("unsupported profiles schema {}", file.schema));
    }
    if file.profiles.is_empty() {
        return Err("profiles.json has no profiles".into());
    }
    let mut ids = std::collections::HashSet::new();
    for p in &file.profiles {
        if p.id.is_empty() || p.params.is_empty() {
            return Err(format!("profile '{}' is empty", p.id));
        }
        if !ids.insert(p.id.as_str()) {
            return Err(format!("duplicate profile id '{}'", p.id));
        }
    }
    Ok(file)
}

#[cfg(test)]
mod tests;
