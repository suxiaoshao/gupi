//! Conversation/source split preference.
use serde::{Deserialize, Serialize};
use std::path::Path;
#[derive(Serialize, Deserialize)]
struct Preference {
    ratio: f32,
}
pub fn load(path: &Path) -> f32 {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| toml::from_str::<Preference>(&s).ok())
        .map(|p| p.ratio)
        .filter(|r| r.is_finite() && *r > 0. && *r < 1.)
        .unwrap_or(0.4)
}
pub fn save(path: &Path, ratio: f32) -> Result<(), String> {
    let bytes = toml::to_string(&Preference { ratio }).map_err(|e| e.to_string())?;
    gupi_resources::persistence::write_atomic(path, bytes.as_bytes()).map_err(|e| e.to_string())
}
