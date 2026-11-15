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

/// Bounded, serial HTTP owner. Results are proposals; only the coordinator may save.
pub struct Worker {
    sender: Option<mpsc::SyncSender<(u64, Recording)>>,
    results: Arc<Mutex<Option<(u64, LookupOutcome)>>>,
    epoch: Arc<AtomicU64>,
    thread: Option<JoinHandle<()>>,
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
        let (sender, jobs) = mpsc::sync_channel::<(u64, Recording)>(1);
        let results = Arc::new(Mutex::new(None));
        let completed = results.clone();
        let epoch = Arc::new(AtomicU64::new(0));
        let shared = epoch.clone();
        let thread = thread::Builder::new()
            .name("singalong-lyrics".into())
            .spawn(move || {
                while let Ok((id, recording)) = jobs.recv() {
                    let token = Cancellation {
                        epoch: shared.clone(),
                        expected: id,
                    };
                    if token.cancelled() {
                        continue;
                    }
                    let result = runtime.block_on(client.lookup(&recording, &token));
                    if !token.cancelled() {
                        *completed.lock().unwrap() = Some((id, result));
                    }
                }
            })
            .map_err(|_| "Could not start lyrics worker.")?;
        Ok(Self {
            sender: Some(sender),
            results,
            epoch,
            thread: Some(thread),
        })
    }
    pub fn cancel(&self) {
        self.epoch.fetch_add(1, Ordering::SeqCst);
    }
    /// A full queue is retried by the coordinator; no unbounded stale jobs.
    pub fn start(&self, recording: Recording) -> Option<u64> {
        let id = self.epoch.fetch_add(1, Ordering::SeqCst) + 1;
        self.sender
            .as_ref()?
            .try_send((id, recording))
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
        self.sender.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
