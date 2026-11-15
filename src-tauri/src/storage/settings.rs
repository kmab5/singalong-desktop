use super::{Result, StorageError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub tracking_enabled: bool,
    pub start_at_sign_in: bool,
    pub auto_overlay: bool,
    pub offline: bool,
    pub translation_consent: bool,
    pub text_size: u32,
    pub opacity: f64,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            tracking_enabled: false,
            start_at_sign_in: false,
            auto_overlay: false,
            offline: false,
            translation_consent: false,
            text_size: 24,
            opacity: 0.95,
        }
    }
}
impl Settings {
    pub(super) fn sanitized(mut self) -> Result<Self> {
        if !self.opacity.is_finite() {
            return Err(StorageError::Invalid("opacity must be finite"));
        }
        self.text_size = self.text_size.clamp(14, 64);
        self.opacity = self.opacity.clamp(0.2, 1.0);
        Ok(self)
    }
}
