use crate::model::PlaybackStatus;
#[derive(Default)]
pub struct PlaybackClock {
    raw: Option<(u64, i64)>,
    anchor: u64,
    at: u64,
    status: Option<PlaybackStatus>,
    rate: Option<f64>,
    unverified_transition: bool,
}
impl PlaybackClock {
    /// Called after a native refresh. UTC age is used only at receipt; elapsed advancement is monotonic.
    pub fn refresh(
        &mut self,
        raw: Option<(u64, i64)>,
        status: PlaybackStatus,
        rate: Option<f64>,
        now: u64,
        age_ms: u64,
    ) -> Option<u64> {
        let Some((position, _)) = raw else {
            self.raw = None;
            return None;
        };
        if matches!(status, PlaybackStatus::Stopped | PlaybackStatus::Unknown)
            || (status == PlaybackStatus::Playing && age_ms > 15000)
        {
            self.raw = None;
            return None;
        }
        if self.raw == raw
            && (self.unverified_transition
                || self.status.as_ref().is_some_and(|old| old != &status))
        {
            self.unverified_transition = true;
            return None;
        }
        self.unverified_transition = false;
        let rate = rate.filter(|r| r.is_finite() && *r > 0.0 && *r <= 16.0);
        if self.raw != raw || self.status.as_ref() != Some(&status) || self.rate != rate {
            self.anchor = position.saturating_add(if status == PlaybackStatus::Playing {
                rate.map_or(0, |r| (age_ms as f64 * r) as u64)
            } else {
                0
            });
            self.at = now;
            self.raw = raw;
            self.status = Some(status.clone());
            self.rate = rate;
        }
        Some(
            self.anchor
                .saturating_add(if status == PlaybackStatus::Playing {
                    rate.map_or(0, |r| (now.saturating_sub(self.at) as f64 * r) as u64)
                } else {
                    0
                }),
        )
    }
}
