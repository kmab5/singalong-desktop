use super::{LookupOutcome, ProviderClient};
use crate::platform::Cancellation;
use singalong_core::model::Recording;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
};
type Results = Arc<Mutex<Option<(u64, LookupOutcome)>>>;
struct Job {
    id: u64,
    recording: Recording,
    token: Cancellation,
    results: Results,
}
struct Owner {
    sender: Option<mpsc::SyncSender<Job>>,
    thread: Option<JoinHandle<()>>,
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
/// Independent cancellation contexts share one bounded queue, HTTP client and cooldowns.
pub struct Worker {
    owner: Arc<Owner>,
    results: Results,
    epoch: Arc<AtomicU64>,
}
impl Worker {
    pub fn new() -> Result<Self, String> {
        Self::with_client(ProviderClient::new().map_err(|_| "Could not initialize lyrics lookup.")?)
    }
    pub(super) fn with_client(mut client: ProviderClient) -> Result<Self, String> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| "Could not initialize lyrics worker.")?;
        let (sender, jobs) = mpsc::sync_channel::<Job>(4);
        let thread = thread::Builder::new()
            .name("singalong-lyrics".into())
            .spawn(move || {
                while let Ok(job) = jobs.recv() {
                    if job.token.cancelled() {
                        continue;
                    }
                    let result = runtime.block_on(client.lookup(&job.recording, &job.token));
                    if !job.token.cancelled() {
                        *job.results.lock().unwrap() = Some((job.id, result));
                    }
                }
            })
            .map_err(|_| "Could not start lyrics worker.")?;
        Ok(Self {
            owner: Arc::new(Owner {
                sender: Some(sender),
                thread: Some(thread),
            }),
            results: Arc::new(Mutex::new(None)),
            epoch: Arc::new(AtomicU64::new(0)),
        })
    }
    pub fn fork(&self) -> Self {
        Self {
            owner: self.owner.clone(),
            results: Arc::new(Mutex::new(None)),
            epoch: Arc::new(AtomicU64::new(0)),
        }
    }
    pub fn cancel(&self) {
        self.epoch.fetch_add(1, Ordering::SeqCst);
        self.results.lock().unwrap().take();
    }
    pub fn start(&self, recording: Recording) -> Option<u64> {
        let id = self.epoch.fetch_add(1, Ordering::SeqCst) + 1;
        self.owner
            .sender
            .as_ref()?
            .try_send(Job {
                id,
                recording,
                token: Cancellation {
                    epoch: self.epoch.clone(),
                    expected: id,
                },
                results: self.results.clone(),
            })
            .ok()
            .map(|_| id)
    }
    pub fn poll(&self) -> Option<(u64, LookupOutcome)> {
        if let Some((id, result)) = self.results.lock().unwrap().take()
            && id == self.epoch.load(Ordering::SeqCst)
        {
            return Some((id, result));
        }
        None
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel();
    }
}
