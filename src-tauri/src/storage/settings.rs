use super::{Result, StorageError};
use serde::{Deserialize, Serialize};
use singalong_core::model::{Appearance, OverlayMode, PlayerId, SelectionMode, WindowGeometry};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct Settings {
    pub tracking_enabled: bool,
    pub enabled_players: Vec<PlayerId>,
    pub selection: SelectionMode,
    pub close_notice_ack: bool,
    pub start_at_sign_in: bool,
    pub auto_overlay: bool,
    pub offline: bool,
    pub translation_consent: bool,
    pub text_size: u32,
    pub opacity: f64,
    pub appearance: Appearance,
    pub overlay_mode: OverlayMode,
    pub overlay_geometry: Option<WindowGeometry>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            tracking_enabled: false,
            enabled_players: vec![],
            selection: SelectionMode::Automatic,
            close_notice_ack: false,
            start_at_sign_in: false,
            auto_overlay: false,
            offline: false,
            translation_consent: false,
            text_size: 24,
            opacity: 0.95,
            appearance: Appearance::default(),
            overlay_mode: OverlayMode::default(),
            overlay_geometry: None,
        }
    }
}
impl Settings {
    pub(super) fn sanitized(mut self) -> Result<Self> {
        if !self.opacity.is_finite() {
            return Err(StorageError::Invalid("opacity must be finite"));
        }
        if !self.appearance.opacity.is_finite() {
            return Err(StorageError::Invalid("opacity must be finite"));
        }
        self.appearance.opacity = self.appearance.opacity.clamp(0.2, 1.0);
        self.appearance.main.text_size = self.appearance.main.text_size.clamp(14, 64);
        self.appearance.floating.text_size = self.appearance.floating.text_size.clamp(14, 64);
        self.overlay_geometry = self.overlay_geometry.filter(|g| {
            [g.x, g.y, g.width, g.height].iter().all(|n| n.is_finite())
                && g.width >= 160.0
                && g.height >= 80.0
                && g.width <= 10000.0
                && g.height <= 10000.0
        });
        self.text_size = self.text_size.clamp(14, 64);
        self.opacity = self.opacity.clamp(0.2, 1.0);
        Ok(self)
    }
}
