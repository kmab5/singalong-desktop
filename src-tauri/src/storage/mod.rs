mod repository;
mod settings;
use repository::Repository;
pub use settings::Settings;
use singalong_core::model::{LyricVersion, Recording};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
};

#[derive(Debug)]
pub enum StorageError {
    Sql(rusqlite::Error),
    Json(serde_json::Error),
    Invalid(&'static str),
    UnsupportedSchema(u32),
    WorkerClosed,
    WorkerStart(std::io::Error),
}
impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(message) => write!(f, "{message}"),
            Self::UnsupportedSchema(version) => write!(
                f,
                "Unsupported database schema {version}; database was not reset"
            ),
            _ => write!(
                f,
                "Local storage is unavailable; existing data has not been reset"
            ),
        }
    }
}
impl std::error::Error for StorageError {}
impl From<rusqlite::Error> for StorageError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sql(value)
    }
}
impl From<serde_json::Error> for StorageError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}
pub type Result<T> = std::result::Result<T, StorageError>;

#[derive(Debug)]
pub struct LibraryRow {
    pub recording: Recording,
    pub favorite: bool,
    pub play_count: u64,
    pub last_played_ms: Option<u64>,
}
type Job = Box<dyn FnOnce(&mut Repository) + Send>;
struct Worker {
    sender: Option<mpsc::SyncSender<Job>>,
    thread: Mutex<Option<JoinHandle<()>>>,
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.sender.take(); // disconnect before join; queued work is drained
        if let Ok(thread) = self.thread.get_mut()
            && let Some(thread) = thread.take()
        {
            let _ = thread.join();
        }
    }
}
/// Only constructed by Rust with a private app-data path; never exposed as a path/SQL IPC bridge.
#[derive(Clone)]
pub struct Storage(Arc<Worker>);
impl Storage {
    pub fn open(path: PathBuf) -> Result<Self> {
        let (sender, receiver) = mpsc::sync_channel::<Job>(64);
        let (ready, result) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("singalong-database".into())
            .spawn(move || match Repository::open(&path) {
                Ok(mut repository) => {
                    if ready.send(Ok(())).is_err() {
                        return;
                    }
                    while let Ok(job) = receiver.recv() {
                        job(&mut repository);
                    }
                }
                Err(error) => {
                    let _ = ready.send(Err(error));
                }
            })
            .map_err(StorageError::WorkerStart)?;
        let status = result
            .recv()
            .map_err(|_| StorageError::WorkerClosed)
            .and_then(|value| value);
        if let Err(error) = status {
            let _ = thread.join();
            return Err(error);
        }
        Ok(Self(Arc::new(Worker {
            sender: Some(sender),
            thread: Mutex::new(Some(thread)),
        })))
    }
    fn call<T: Send + 'static>(
        &self,
        operation: impl FnOnce(&mut Repository) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let (sender, result) = mpsc::sync_channel(1);
        self.0
            .sender
            .as_ref()
            .ok_or(StorageError::WorkerClosed)?
            .send(Box::new(move |db| {
                let _ = sender.send(operation(db));
            }))
            .map_err(|_| StorageError::WorkerClosed)?;
        result.recv().map_err(|_| StorageError::WorkerClosed)?
    }
    pub fn resolve(&self, value: Recording, previous: Option<String>) -> Result<Recording> {
        self.call(move |db| db.resolve(value, previous))
    }
    pub fn save_versions(&self, id: String, versions: Vec<LyricVersion>) -> Result<()> {
        self.call(move |db| db.save_versions(id, versions))
    }
    pub fn versions(&self, id: String) -> Result<Vec<LyricVersion>> {
        self.call(move |db| db.versions(id))
    }
    pub fn select(&self, id: String, version: String) -> Result<()> {
        self.call(move |db| db.select(id, version))
    }
    pub fn selected(&self, id: String) -> Result<Option<LyricVersion>> {
        self.call(move |db| db.selected(id))
    }
    pub fn adjust(&self, id: String, delay: i64, estimated: bool) -> Result<()> {
        self.call(move |db| db.adjust(id, delay, estimated))
    }
    pub fn adjustment(&self, id: String) -> Result<(i64, bool)> {
        self.call(move |db| db.adjustment(id))
    }
    pub fn favorite(&self, id: String, value: bool) -> Result<()> {
        self.call(move |db| db.favorite(id, value))
    }
    pub fn list(&self, offset: u32, limit: u32) -> Result<Vec<LibraryRow>> {
        self.call(move |db| db.list(offset, limit))
    }
    pub fn settings(&self) -> Result<Settings> {
        self.call(|db| db.settings())
    }
    pub fn save_settings(&self, settings: Settings) -> Result<()> {
        self.call(move |db| db.save_settings(settings))
    }
}
