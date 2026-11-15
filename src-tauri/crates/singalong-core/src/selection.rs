use crate::model::*;
#[derive(Default)]
pub struct Selector {
    current: Option<SessionId>,
}
impl Selector {
    pub fn select(
        &mut self,
        sessions: &[PlaybackSnapshot],
        enabled: &[PlayerId],
        mode: &SelectionMode,
    ) -> Option<SessionId> {
        let mut eligible: Vec<_> = sessions
            .iter()
            .filter(|s| enabled.contains(&s.player_id) && s.status != PlaybackStatus::Stopped)
            .filter(|s| match mode {
                SelectionMode::Automatic => true,
                SelectionMode::AppPin { player_id } => &s.player_id == player_id,
                SelectionMode::StreamPin {
                    player_id,
                    session_id,
                } => {
                    &s.player_id == player_id
                        && &s.session_id == session_id
                        && s.stream_pin_available
                }
            })
            .collect();
        eligible.sort_by(|a, b| {
            (
                a.status != PlaybackStatus::Playing,
                &a.player_id.native_id,
                &a.session_id.0,
            )
                .cmp(&(
                    b.status != PlaybackStatus::Playing,
                    &b.player_id.native_id,
                    &b.session_id.0,
                ))
        });
        let selected = eligible
            .iter()
            .find(|s| Some(&s.session_id) == self.current.as_ref())
            .copied()
            .or_else(|| eligible.first().copied());
        self.current = selected.map(|s| s.session_id.clone());
        self.current.clone()
    }
    pub fn reset(&mut self) {
        self.current = None;
    }
}
