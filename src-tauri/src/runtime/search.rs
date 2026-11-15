use super::lookup::Jobs;
use crate::{providers::LookupOutcome, storage::Storage};
use singalong_core::model::*;
/// One main-window search, independent of the live reader but sharing provider pacing.
pub struct Search {
    db: Storage,
    jobs: Box<dyn Jobs>,
    token: String,
    context: String,
    recording: Option<Recording>,
    target: Option<String>,
    job: Option<u64>,
    status: LookupStatus,
    message: String,
    versions: Vec<LyricVersion>,
}
impl Search {
    pub fn new(db: Storage, jobs: Box<dyn Jobs>) -> Self {
        Self {
            db,
            jobs,
            token: String::new(),
            context: String::new(),
            recording: None,
            target: None,
            job: None,
            status: LookupStatus::Idle,
            message: String::new(),
            versions: vec![],
        }
    }
    pub fn cancel(&mut self) {
        self.jobs.cancel();
        self.job = None;
        self.recording = None;
        self.token.clear();
        self.versions.clear();
        self.status = LookupStatus::Idle;
    }
    pub fn cancel_context(&mut self, context: &str) {
        if context == self.context {
            self.cancel();
        }
    }
    pub fn start(&mut self, query: SearchQuery, offline: bool) -> Result<ProductReply, String> {
        if query.context.is_empty() || query.context.len() > 128 {
            return Err("Invalid search context.".into());
        }
        self.context = query.context.clone();
        self.cancel();
        if offline {
            return Err(
                "Go online to search providers. Saved lyrics are available in Library.".into(),
            );
        }
        if query.title.trim().is_empty()
            || query.artist.trim().is_empty()
            || query.title.len() > 4096
            || query.artist.len() > 4096
            || query.album.as_ref().is_some_and(|v| v.len() > 4096)
        {
            return Err("Enter a title and artist within the supported size.".into());
        }
        if let Some(id) = &query.target {
            self.db.detail(id.clone()).map_err(|e| e.to_string())?;
        }
        self.token = uuid::Uuid::new_v4().to_string();
        self.target = query.target;
        self.recording = Some(Recording {
            id: format!("search:{}", self.token),
            title: query.title.trim().into(),
            artists: vec![query.artist.trim().into()],
            album: query.album.filter(|a| !a.trim().is_empty()),
            duration_ms: None,
            version_label: None,
            file_name: None,
        });
        self.status = LookupStatus::Loading;
        self.message = "Searching providers…".into();
        self.poll_current();
        Ok(self.reply())
    }
    fn reply(&self) -> ProductReply {
        ProductReply::Search {
            token: self.token.clone(),
            status: self.status.clone(),
            message: self.message.clone(),
            versions: self.versions.clone(),
        }
    }
    fn poll_current(&mut self) {
        if self.job.is_none() && self.status == LookupStatus::Loading {
            if let Some(recording) = &self.recording {
                self.job = self.jobs.start(recording.clone());
            }
        }
        if let Some((id, result)) = self.jobs.poll()
            && self.job == Some(id)
        {
            self.job = None;
            match result {
                LookupOutcome::Found(versions) => {
                    self.versions = versions;
                    self.status = LookupStatus::Ready;
                    self.message = "Choose a version to save.".into();
                }
                LookupOutcome::Instrumental => {
                    self.status = LookupStatus::Instrumental;
                    self.message = "This result is marked instrumental.".into();
                }
                LookupOutcome::NotFound => {
                    self.status = LookupStatus::NotFound;
                    self.message =
                        "No matching lyrics found. Try correcting the title or artist.".into();
                }
                LookupOutcome::InsufficientMetadata => {
                    self.status = LookupStatus::InsufficientMetadata;
                    self.message = "More title or artist information is needed.".into();
                }
                LookupOutcome::Failed(f) => {
                    self.status = LookupStatus::Failed;
                    self.message = if f.retry_after_ms.is_some() {
                        "Provider cooldown active. Try again later."
                    } else {
                        "Provider unavailable. Try again later."
                    }
                    .into();
                }
                LookupOutcome::Cancelled => {
                    self.status = LookupStatus::Idle;
                    self.message = "Search cancelled.".into();
                }
            }
        }
    }
    pub fn poll(&mut self, token: &str, offline: bool) -> Result<ProductReply, String> {
        if offline {
            self.cancel();
            return Err("Search cancelled in Offline mode.".into());
        }
        if token.is_empty() || token != self.token {
            return Err("This search is no longer current.".into());
        }
        self.poll_current();
        Ok(self.reply())
    }
    pub fn save(&mut self, token: &str, version_id: &str) -> Result<ProductReply, String> {
        if token.is_empty() || token != self.token {
            return Err("This search is no longer current.".into());
        }
        let mut version = self
            .versions
            .iter()
            .find(|v| v.id == version_id)
            .cloned()
            .ok_or("Choose a result from the current search.")?;
        let recording = if let Some(target) = &self.target {
            self.db
                .detail(target.clone())
                .map_err(|e| e.to_string())?
                .item
                .recording
        } else {
            self.db
                .resolve(self.recording.clone().ok_or("Search expired.")?, None)
                .map_err(|e| e.to_string())?
        };
        version.recording_id = recording.id.clone();
        version.id = super::lookup::saved_version_id(&recording.id, &version);
        let existing = self
            .db
            .versions(recording.id.clone())
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|v| v.id == version.id);
        if let Some(existing) = existing {
            version = existing;
        } else {
            self.db
                .save_versions(recording.id.clone(), vec![version.clone()])
                .map_err(|e| e.to_string())?;
        }
        self.db
            .select(recording.id.clone(), version.id)
            .map_err(|e| e.to_string())?;
        Ok(ProductReply::Detail {
            detail: self.db.detail(recording.id).map_err(|e| e.to_string())?,
        })
    }
}
