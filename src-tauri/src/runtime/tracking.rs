use crate::{
    platform::Observation,
    storage::{Settings, Storage},
};
use singalong_core::{identity, model::*, qualification::Qualifier, selection::Selector};
use std::collections::HashMap;
pub struct Engine {
    pub snapshot: RuntimeSnapshot,
    pub settings: Settings,
    db: Storage,
    selector: Selector,
    qualifier: Qualifier,
    records: HashMap<String, Recording>,
    saved_ids: HashMap<String, String>,
    run_id: String,
    pending: Option<(Recording, String, u64)>,
    last_selected: Option<PlaybackSnapshot>,
    resuming: bool,
}
impl Engine {
    pub fn new(db: Storage) -> Result<Self, String> {
        let settings = db.settings().map_err(|e| e.to_string())?;
        let mut snapshot = RuntimeSnapshot::foundation();
        snapshot.capabilities.tracking_available = cfg!(windows);
        snapshot.capabilities.reason="Enable players to follow their exposed Windows media sessions. Lyrics lookup is the next phase.".into();
        snapshot.tracking_enabled = settings.tracking_enabled;
        snapshot.selection = settings.selection.clone();
        snapshot.start_at_sign_in = settings.start_at_sign_in;
        snapshot.auto_overlay = settings.auto_overlay;
        snapshot.players = settings
            .enabled_players
            .iter()
            .map(|id| PlayerInfo {
                id: id.clone(),
                name: id
                    .native_id
                    .rsplit(['/', '\\'])
                    .next()
                    .unwrap_or(&id.native_id)
                    .into(),
                enabled: true,
                available: false,
            })
            .collect();
        Ok(Self {
            snapshot,
            settings,
            db,
            selector: Selector::default(),
            qualifier: Qualifier::default(),
            records: HashMap::new(),
            saved_ids: HashMap::new(),
            run_id: uuid::Uuid::new_v4().to_string(),
            pending: None,
            last_selected: None,
            resuming: false,
        })
    }
    pub fn apply(&mut self, action: RuntimeAction) -> Result<(), String> {
        let mut next = self.settings.clone();
        match action {
            RuntimeAction::Tracking { enabled } => next.tracking_enabled = enabled,
            RuntimeAction::EnablePlayer { player_id, enabled } => {
                if !self.snapshot.players.iter().any(|p| p.id == player_id) {
                    return Err("Discover this player first.".into());
                }
                next.enabled_players.retain(|p| p != &player_id);
                if enabled {
                    next.enabled_players.push(player_id);
                }
            }
            RuntimeAction::Select { mode } => {
                match &mode {
                    SelectionMode::Automatic => {}
                    SelectionMode::AppPin { player_id } => {
                        if !self.snapshot.players.iter().any(|p| &p.id == player_id) {
                            return Err("Discover this player first.".into());
                        }
                    }
                    SelectionMode::StreamPin {
                        player_id,
                        session_id,
                    } => {
                        if !self.snapshot.sessions.iter().any(|s| {
                            &s.player_id == player_id
                                && &s.session_id == session_id
                                && s.stream_pin_available
                                && next.enabled_players.contains(player_id)
                        }) {
                            return Err("This stream cannot be pinned reliably. Use an explicit app pin instead.".into());
                        }
                    }
                }
                next.selection = mode;
            }
            RuntimeAction::AutoOverlay { enabled } => next.auto_overlay = enabled,
            RuntimeAction::Startup { enabled } => next.start_at_sign_in = enabled,
            RuntimeAction::CloseToTray => next.close_notice_ack = true,
            RuntimeAction::Discover => return Ok(()),
            RuntimeAction::Quit => return Err("Quit is handled by the native shell.".into()),
        }
        self.db
            .save_settings(next.clone())
            .map_err(|e| e.to_string())?;
        self.settings = next;
        self.snapshot.tracking_enabled = self.settings.tracking_enabled;
        self.snapshot.selection = self.settings.selection.clone();
        self.snapshot.start_at_sign_in = self.settings.start_at_sign_in;
        self.snapshot.auto_overlay = self.settings.auto_overlay;
        if self.settings.close_notice_ack {
            self.snapshot.close_notice = false;
        }
        for p in &mut self.snapshot.players {
            p.enabled = self.settings.enabled_players.contains(&p.id);
        }
        // Reconcile immediately: no stale selection after allowlist or pin changes.
        self.snapshot
            .sessions
            .retain(|s| self.settings.enabled_players.contains(&s.player_id));
        if !self.settings.tracking_enabled {
            self.snapshot.sessions.clear();
            self.selector.reset();
            self.qualifier.suspend();
            self.resuming = true;
            self.pending = None;
        }
        self.choose();
        Ok(())
    }
    fn choose(&mut self) {
        self.snapshot.selected_session_id = self.selector.select(
            &self.snapshot.sessions,
            &self.settings.enabled_players,
            &self.settings.selection,
        );
        self.snapshot.status_message = if !self.settings.tracking_enabled {
            "Tracking stopped"
        } else if self.snapshot.selected_session_id.is_none()
            && !matches!(self.settings.selection, SelectionMode::Automatic)
        {
            "Waiting for pinned player"
        } else if self.snapshot.selected_session_id.is_none() {
            "Waiting for an enabled player"
        } else {
            ""
        }
        .into();
    }
    pub fn unavailable(&mut self, message: &str) {
        self.snapshot.sessions.clear();
        self.snapshot.selected_session_id = None;
        self.snapshot.status_message = message.into();
        self.qualifier.suspend();
        self.resuming = true;
        self.pending = None;
    }
    pub fn observe(&mut self, observation: Observation, now: u64, wall: u64) -> Result<(), String> {
        self.observe_checked(observation, now, wall, &|| false)
    }
    pub fn observe_checked(
        &mut self,
        observation: Observation,
        now: u64,
        wall: u64,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<(), String> {
        if cancelled() {
            return Ok(());
        }
        let mut players = observation.players;
        for p in &mut players {
            p.enabled = self.settings.enabled_players.contains(&p.id);
        }
        for old in &self.snapshot.players {
            if !players.iter().any(|p| p.id == old.id) {
                let mut p = old.clone();
                p.available = false;
                players.push(p);
            }
        }
        players.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then(a.id.native_id.cmp(&b.id.native_id))
        });
        self.snapshot.players = players;
        self.snapshot.sessions = if self.settings.tracking_enabled {
            observation
                .sessions
                .into_iter()
                .filter(|s| self.settings.enabled_players.contains(&s.player_id))
                .collect()
        } else {
            vec![]
        };
        self.records
            .retain(|id, _| self.snapshot.sessions.iter().any(|s| &s.session_id.0 == id));
        for session in &mut self.snapshot.sessions {
            if let Some(recording) = &mut session.recording {
                if let Some(old) = self
                    .records
                    .get(&session.session_id.0)
                    .filter(|old| identity::same_observation(old, recording))
                {
                    *recording = identity::enrich_missing(old, recording);
                } else {
                    recording.id = uuid::Uuid::new_v4().to_string();
                }
                self.records
                    .insert(session.session_id.0.clone(), recording.clone());
            } else {
                self.records.remove(&session.session_id.0);
            }
        }
        self.choose();
        if !self.settings.tracking_enabled {
            return Ok(());
        }
        if self.resuming {
            if let Some(old) = &self.last_selected {
                let matches = self
                    .snapshot
                    .sessions
                    .iter()
                    .filter(|s| s.player_id == old.player_id)
                    .count();
                if matches == 1
                    && let Some(selected) =
                        self.snapshot.sessions.iter_mut().find(|s| {
                            Some(&s.session_id) == self.snapshot.selected_session_id.as_ref()
                        })
                    && selected.player_id == old.player_id
                    && let (Some(current), Some(previous)) =
                        (&mut selected.recording, &old.recording)
                    && identity::same_observation(previous, current)
                {
                    *current = identity::enrich_missing(previous, current);
                    self.records
                        .insert(selected.session_id.0.clone(), current.clone());
                }
            }
            self.resuming = false;
        }
        let selected = self
            .snapshot
            .sessions
            .iter()
            .find(|s| Some(&s.session_id) == self.snapshot.selected_session_id.as_ref());
        // Logical observation identity survives a stopped observer; this never reconnects a UI stream pin.
        let mut logical = selected.cloned();
        if let Some(s) = &mut logical
            && let Some(r) = &s.recording
        {
            s.session_id = SessionId(r.id.clone());
        }
        self.last_selected = selected.cloned();
        if let Some(sequence) = self.qualifier.observe(logical.as_ref(), now) {
            self.pending = selected
                .and_then(|s| s.recording.clone())
                .map(|r| (r, format!("{}-{sequence}", self.run_id), wall));
        }
        if cancelled() {
            return Ok(());
        }
        if let Some((recording, id, wall)) = &self.pending {
            let saved = self
                .db
                .resolve(
                    recording.clone(),
                    self.saved_ids.get(&recording.id).cloned(),
                )
                .map_err(|e| e.to_string())?;
            self.saved_ids
                .insert(recording.id.clone(), saved.id.clone());
            if cancelled() {
                return Ok(());
            }
            self.db
                .record_play(saved.id, id.clone(), *wall)
                .map_err(|e| e.to_string())?;
            self.pending = None;
        }
        Ok(())
    }
}
