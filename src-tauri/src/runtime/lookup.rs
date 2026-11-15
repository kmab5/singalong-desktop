use crate::{
    platform::Cancellation,
    providers::{LookupOutcome, Worker},
    storage::Storage,
};
use singalong_core::{lyrics::timeline, model::*};

pub trait Jobs: Send {
    fn start(&mut self, recording: Recording) -> Option<u64>;
    fn cancel(&mut self);
    fn poll(&mut self) -> Option<(u64, LookupOutcome)>;
}
impl Jobs for Worker {
    fn start(&mut self, r: Recording) -> Option<u64> {
        Worker::start(self, r)
    }
    fn cancel(&mut self) {
        Worker::cancel(self)
    }
    fn poll(&mut self) -> Option<(u64, LookupOutcome)> {
        Worker::poll(self)
    }
}
struct CachedOutcome {
    key: String,
    status: LookupStatus,
    message: String,
    until: u64,
    retry_after_ms: Option<Counter>,
}
pub struct Coordinator {
    pub state: ReaderState,
    db: Storage,
    jobs: Box<dyn Jobs>,
    target: String,
    job: Option<u64>,
    versions: Vec<LyricVersion>,
    persisted: bool,
    delay: i64,
    retry_at: u64,
    recent: std::collections::VecDeque<CachedOutcome>,
    cache_key: String,
}
impl Coordinator {
    pub fn new(db: Storage, jobs: Box<dyn Jobs>) -> Self {
        Self {
            state: ReaderState::default(),
            db,
            jobs,
            target: String::new(),
            job: None,
            versions: vec![],
            persisted: false,
            delay: 0,
            retry_at: 0,
            recent: std::collections::VecDeque::new(),
            cache_key: String::new(),
        }
    }
    pub fn retry(&mut self) {
        if self.state.version.is_none() {
            self.recent.retain(|entry| entry.key != self.cache_key);
            self.jobs.cancel();
            self.job = None;
            self.state.status = LookupStatus::Waiting;
            self.retry_at = 0;
        }
    }
    pub fn reset(&mut self) {
        self.jobs.cancel();
        self.job = None;
        self.target.clear();
        self.versions.clear();
        self.persisted = false;
        self.delay = 0;
        self.retry_at = 0;
        self.state = ReaderState::default();
    }
    fn saved(&mut self, recording: &Recording, qualified: Option<&str>) -> Result<(), String> {
        if let Some(saved) = self
            .db
            .find(recording.clone(), qualified.map(str::to_owned))
            .map_err(|e| e.to_string())?
            && let Some(version) = self.db.selected(saved.id).map_err(|e| e.to_string())?
        {
            self.delay = self
                .db
                .adjustment(version.id.clone())
                .map_err(|e| e.to_string())?
                .0;
            self.state.version = Some(version);
            self.state.status = LookupStatus::Ready;
            self.state.cached = true;
            self.persisted = true;
        }
        Ok(())
    }
    fn failure(&mut self, message: String, now: u64) {
        self.state.status = LookupStatus::Failed;
        self.state.message = message;
        self.retry_at = now.saturating_add(2000);
    }
    /// Called only by the tracking coordinator. No window owns independent lookup.
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        session: Option<&PlaybackSnapshot>,
        qualified: Option<&str>,
        reader_open: bool,
        offline: bool,
        now: u64,
        token: &Cancellation,
    ) -> bool {
        if token.cancelled() {
            return false;
        }
        let before = self.display_key();
        let Some(recording) = session.and_then(|s| s.recording.as_ref()) else {
            if !self.target.is_empty() {
                self.reset();
            }
            return before != self.display_key();
        };
        let key =
            serde_json::to_string(recording).expect("recording contains only serializable values");
        if self.target != key {
            self.reset();
            self.target = key;
            let mut identity = recording.clone();
            identity.id.clear();
            self.cache_key = serde_json::to_string(&identity).expect("serializable recording");
            self.recent.retain(|entry| entry.until > now);
            self.state.status = LookupStatus::Waiting;
            if let Err(error) = self.saved(recording, qualified) {
                self.failure(error, now);
            }
            if self.state.version.is_none()
                && let Some(entry) = self.recent.iter().find(|entry| entry.key == self.cache_key)
            {
                self.state.status = entry.status.clone();
                self.state.message = entry.message.clone();
                self.state.retry_after_ms = entry.retry_after_ms.clone();
                self.retry_at = entry.until;
            }
        }
        if offline {
            if self.job.take().is_some() {
                self.jobs.cancel();
            }
            if self.state.version.is_none() {
                self.state.status = LookupStatus::Offline;
                self.state.message = "Offline · no saved lyrics for this recording".into();
            }
        } else if self.state.status == LookupStatus::Offline {
            self.state.status = LookupStatus::Waiting;
            self.state.message.clear();
        }
        if let Some((id, outcome)) = self.jobs.poll()
            && !offline
            && self.job == Some(id)
            && !token.cancelled()
        {
            self.job = None;
            self.state.message.clear();
            self.state.retry_after_ms = None;
            match outcome {
                LookupOutcome::Found(versions) => {
                    self.state.version = versions.first().cloned();
                    self.versions = versions;
                    self.state.status = LookupStatus::Ready;
                    if let Err(error) = self.saved(recording, qualified) {
                        self.state.message = error;
                    }
                }
                LookupOutcome::Instrumental => {
                    self.state.status = LookupStatus::Instrumental;
                    self.state.message = "This recording is marked instrumental".into();
                    self.retry_at = now.saturating_add(600000);
                }
                LookupOutcome::NotFound => {
                    self.state.status = LookupStatus::NotFound;
                    self.state.message = "No matching lyrics found".into();
                    self.retry_at = now.saturating_add(600000);
                }
                LookupOutcome::InsufficientMetadata => {
                    self.state.status = LookupStatus::InsufficientMetadata;
                    self.state.message =
                        "More title or artist information is needed to find lyrics".into();
                    self.retry_at = u64::MAX;
                }
                LookupOutcome::Failed(failure) => {
                    self.failure("Lyrics provider unavailable · try again later".into(), now);
                    self.state.retry_after_ms = failure.retry_after_ms.map(Counter::new);
                    self.retry_at =
                        now.saturating_add(failure.retry_after_ms.unwrap_or(2000).max(2000));
                }
                LookupOutcome::Cancelled => self.state.status = LookupStatus::Waiting,
            }
            if matches!(
                self.state.status,
                LookupStatus::NotFound
                    | LookupStatus::Instrumental
                    | LookupStatus::InsufficientMetadata
                    | LookupStatus::Failed
            ) {
                self.recent.retain(|entry| entry.key != self.cache_key);
                while self.recent.len() >= 256 {
                    self.recent.pop_front();
                }
                self.recent.push_back(CachedOutcome {
                    key: self.cache_key.clone(),
                    status: self.state.status.clone(),
                    message: self.state.message.clone(),
                    until: self.retry_at,
                    retry_after_ms: self.state.retry_after_ms.clone(),
                });
            }
        }
        if let Some(id) = qualified
            && !self.persisted
            && self.state.version.is_some()
            && !token.cancelled()
        {
            // Re-read at the save boundary. A selection made while HTTP ran wins.
            let save = (|| -> Result<(), String> {
                self.saved(recording, Some(id))?;
                if self.persisted || token.cancelled() {
                    return Ok(());
                }
                let versions = self
                    .versions
                    .iter()
                    .cloned()
                    .map(|mut v| {
                        v.recording_id = id.into();
                        v.id = format!(
                            "{}:{}:{}:{}",
                            id,
                            v.source_name
                                .as_deref()
                                .unwrap_or("provider")
                                .to_lowercase(),
                            v.provider_id.as_deref().unwrap_or("0"),
                            v.content_hash
                        );
                        v
                    })
                    .collect();
                self.db
                    .save_versions_checked(id.into(), versions, token.clone())
                    .map_err(|e| e.to_string())?;
                self.saved(recording, Some(id))
            })();
            if let Err(error) = save {
                self.state.message = error;
            }
        }
        if !offline
            && !token.cancelled()
            && self.state.version.is_none()
            && self.job.is_none()
            && (reader_open || qualified.is_some())
            && now >= self.retry_at
            && let Some(job) = self.jobs.start(recording.clone())
        {
            self.job = Some(job);
            self.state.status = LookupStatus::Loading;
            self.state.message = "Finding lyrics…".into();
        }
        self.state.position_available = false;
        self.state.active_line = None;
        self.state.countdown_seconds = None;
        if let (Some(version), Some(session)) = (&self.state.version, session) {
            let position = session
                .position_ms
                .as_ref()
                .filter(|_| now.saturating_sub(session.observed_at_ms.value()) <= 3000)
                .map(|p| {
                    let elapsed = now.saturating_sub(session.observed_at_ms.value()).min(3000);
                    let extra = if session.status == PlaybackStatus::Playing {
                        session
                            .rate
                            .filter(|r| r.is_finite() && *r > 0.0)
                            .map_or(0, |r| (elapsed as f64 * r) as u64)
                    } else {
                        0
                    };
                    let position = p.value().saturating_add(extra);
                    recording
                        .duration_ms
                        .as_ref()
                        .map_or(position, |d| position.min(d.value()))
                });
            self.state.position_available = position.is_some();
            let moment = timeline::at(&version.lines, position, self.delay);
            self.state.active_line = moment.index.map(|i| i as u32);
            self.state.countdown_seconds = moment.countdown_seconds.map(Counter::new);
        }
        before != self.display_key()
    }
    fn display_key(&self) -> String {
        format!(
            "{:?}:{:?}:{:?}:{:?}:{}:{}:{}:{:?}",
            self.state.status,
            self.state.version.as_ref().map(|v| &v.id),
            self.state.active_line,
            self.state.countdown_seconds,
            self.state.position_available,
            self.state.cached,
            self.state.message,
            self.state.retry_after_ms
        )
    }
}
