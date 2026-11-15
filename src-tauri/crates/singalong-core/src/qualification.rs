use crate::model::*;
/// Process-local sequence; the coordinator prefixes it with a unique runtime ID.
#[derive(Default)]
pub struct Qualifier {
    previous: Option<PlaybackSnapshot>,
    at: u64,
    listened: u64,
    sequence: u64,
    qualified: bool,
}
impl Qualifier {
    pub fn interrupt(&mut self) {
        self.previous = None;
        self.listened = 0;
        self.qualified = false;
    }
    pub fn observe(&mut self, snapshot: Option<&PlaybackSnapshot>, now: u64) -> Option<u64> {
        let Some(s) =
            snapshot.filter(|s| s.recording.is_some() && s.status != PlaybackStatus::Stopped)
        else {
            self.interrupt();
            return None;
        };
        let recording = s.recording.as_ref().unwrap();
        let changed = self.previous.as_ref().is_none_or(|p| {
            p.session_id != s.session_id
                || p.recording.as_ref().map(|r| &r.id) != Some(&recording.id)
        });
        let repeated = self.previous.as_ref().is_some_and(|p| {
            match (&p.position_ms, &s.position_ms, &recording.duration_ms) {
                (Some(old), Some(new), Some(duration)) => {
                    old.value() >= duration.value().saturating_sub(1500)
                        && new.value() <= 1500
                        && old.value().saturating_sub(new.value()) > 5000
                }
                _ => false,
            }
        });
        if changed || repeated {
            self.listened = 0;
            self.qualified = false;
            self.sequence += 1;
        } else if self.previous.as_ref().is_some_and(|p| {
            p.status == PlaybackStatus::Playing && p.rate.is_none_or(|r| r.is_finite() && r > 0.0)
        }) {
            let elapsed = now.saturating_sub(self.at);
            if elapsed <= 10000 {
                self.listened = self.listened.saturating_add(elapsed);
            }
        }
        self.previous = Some(s.clone());
        self.at = now;
        if !self.qualified && self.listened >= 10000 {
            self.qualified = true;
            Some(self.sequence)
        } else {
            None
        }
    }
}
