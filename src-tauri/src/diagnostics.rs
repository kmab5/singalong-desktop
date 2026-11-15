use singalong_core::model::RuntimeSnapshot;
use std::sync::Mutex;
#[derive(Default)]
pub struct Reports(Mutex<Option<Vec<u8>>>);
impl Reports {
    pub fn preview(&self, state: &RuntimeSnapshot) -> Result<String, String> {
        let value = serde_json::json!({"schema":1,"applicationVersion":env!("CARGO_PKG_VERSION"),"os":std::env::consts::OS,"architecture":std::env::consts::ARCH,"trackingEnabled":state.tracking_enabled,"offline":state.offline,"sessionCount":state.sessions.len(),"enabledPlayerCount":state.players.iter().filter(|p|p.enabled).count(),"floatingVisible":state.overlay_visible,"floatingClickThrough":state.click_through,"readerStatus":state.reader.status,"positionAvailable":state.reader.position_available,"savedLyricsDisplayed":state.reader.cached});
        let report =
            serde_json::to_string_pretty(&value).map_err(|_| "Could not prepare diagnostics.")?;
        if report.len() > 16384 {
            return Err("Diagnostic report exceeds its size limit.".into());
        }
        *self.0.lock().map_err(|_| "Diagnostics unavailable.")? = Some(report.as_bytes().to_vec());
        Ok(report)
    }
    pub fn reviewed(&self) -> Result<Vec<u8>, String> {
        self.0
            .lock()
            .map_err(|_| "Diagnostics unavailable.")?
            .clone()
            .ok_or_else(|| "Preview diagnostics before exporting.".into())
    }
}
