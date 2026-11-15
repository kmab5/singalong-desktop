pub mod tracking;
use crate::{
    platform::{Adapter, Cancellation, NativeAdapter},
    storage::Storage,
};
use singalong_core::model::*;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tracking::Engine;
type Reply = mpsc::SyncSender<Result<(), String>>;
enum Request {
    Action(u64, RuntimeAction, Reply),
    Close,
    Suspend(u64, bool),
    Quit,
}
pub type Factory = Box<dyn Fn(&Cancellation) -> Result<Box<dyn Adapter>, &'static str> + Send>;
pub type Publish = Arc<dyn Fn(RuntimeSnapshot) + Send + Sync>;
struct Inner {
    sender: mpsc::Sender<Request>,
    snapshot: Arc<Mutex<RuntimeSnapshot>>,
    epoch: Arc<AtomicU64>,
    thread: Mutex<Option<JoinHandle<()>>>,
}
impl Drop for Inner {
    fn drop(&mut self) {
        self.epoch.fetch_add(1, Ordering::SeqCst);
        let _ = self.sender.send(Request::Quit);
        if let Ok(t) = self.thread.get_mut()
            && let Some(t) = t.take()
        {
            let _ = t.join();
        }
    }
}
#[derive(Clone)]
pub struct Runtime(Arc<Inner>);
impl Runtime {
    pub fn native(db: Storage, publish: Publish) -> Result<Self, String> {
        Self::spawn(
            db,
            publish,
            Box::new(|c| NativeAdapter::new(c).map(|a| Box::new(a) as Box<dyn Adapter>)),
        )
    }
    pub fn spawn(db: Storage, publish: Publish, factory: Factory) -> Result<Self, String> {
        let engine = Engine::new(db)?;
        let snapshot = Arc::new(Mutex::new(engine.snapshot.clone()));
        let shared = snapshot.clone();
        let epoch = Arc::new(AtomicU64::new(0));
        let cancel_epoch = epoch.clone();
        let (sender, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("singalong-tracking".into())
            .spawn(move || worker(engine, receiver, shared, cancel_epoch, publish, factory))
            .map_err(|_| "Could not start tracking coordinator.")?;
        Ok(Self(Arc::new(Inner {
            sender,
            snapshot,
            epoch,
            thread: Mutex::new(Some(thread)),
        })))
    }
    pub fn snapshot(&self) -> RuntimeSnapshot {
        self.0.snapshot.lock().unwrap().clone()
    }
    pub fn action(&self, action: RuntimeAction) -> Result<(), String> {
        let generation = self.0.epoch.fetch_add(1, Ordering::SeqCst) + 1;
        let (tx, rx) = mpsc::sync_channel(1);
        self.0
            .sender
            .send(Request::Action(generation, action, tx))
            .map_err(|_| "Tracking has stopped.")?;
        rx.recv().map_err(|_| "Tracking has stopped.")?
    }
    pub fn close_notice(&self) {
        let _ = self.0.sender.send(Request::Close);
    }
    pub fn suspend(&self, suspended: bool) {
        let generation = self.0.epoch.fetch_add(1, Ordering::SeqCst) + 1;
        let _ = self.0.sender.send(Request::Suspend(generation, suspended));
    }
    pub fn shutdown(&self) {
        self.0.epoch.fetch_add(1, Ordering::SeqCst);
        let _ = self.0.sender.send(Request::Quit);
        if let Some(t) = self.0.thread.lock().unwrap().take() {
            let _ = t.join();
        }
    }
}
fn worker(
    mut engine: Engine,
    receiver: mpsc::Receiver<Request>,
    shared: Arc<Mutex<RuntimeSnapshot>>,
    epoch: Arc<AtomicU64>,
    publish: Publish,
    factory: Factory,
) {
    let started = Instant::now();
    let mut adapter: Option<Box<dyn Adapter>> = None;
    let mut suspended = false;
    let mut discovery = false;
    let mut refresh = Instant::now();
    let mut revision = 0;
    let mut processed = 0;
    let emit = |engine: &mut Engine, revision: &mut u64, generation: u64| {
        if epoch.load(Ordering::SeqCst) != generation {
            return;
        }
        *revision += 1;
        engine.snapshot.revision = Counter::new(*revision);
        engine.snapshot.generation = Counter::new(generation);
        let value = engine.snapshot.clone();
        *shared.lock().unwrap() = value.clone();
        publish(value);
    };
    loop {
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(Request::Quit) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Ok(Request::Suspend(generation, value)) => {
                if generation < processed {
                    continue;
                }
                processed = generation;
                suspended = value;
                adapter = None;
                engine.unavailable(if value {
                    "Tracking suspended"
                } else {
                    "Refreshing playback"
                });
                refresh = Instant::now();
                emit(&mut engine, &mut revision, processed);
            }
            Ok(Request::Close) => {
                engine.snapshot.close_notice = true;
                emit(&mut engine, &mut revision, processed);
            }
            Ok(Request::Action(generation, action, reply)) => {
                if generation < processed {
                    let _ = reply.send(Err("A newer action superseded this request.".into()));
                    continue;
                }
                processed = generation;
                discovery = matches!(action, RuntimeAction::Discover);
                let result = engine.apply(action);
                if !engine.settings.tracking_enabled {
                    adapter = None;
                }
                refresh = Instant::now();
                emit(&mut engine, &mut revision, processed);
                let _ = reply.send(result);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if adapter.as_ref().is_some_and(|a| a.changed()) {
            refresh = Instant::now();
        }
        let cancel = Cancellation {
            epoch: epoch.clone(),
            expected: processed,
        };
        // A queued command owns a newer epoch; never borrow that epoch for old work.
        if cancel.cancelled()
            || suspended
            || (!engine.settings.tracking_enabled && !discovery)
            || Instant::now() < refresh
        {
            continue;
        }
        let observation = (|| {
            if adapter.is_none() {
                adapter = Some(factory(&cancel)?);
            }
            adapter.as_mut().unwrap().refresh(
                if engine.settings.tracking_enabled {
                    &engine.settings.enabled_players
                } else {
                    &[]
                },
                started.elapsed().as_millis() as u64,
                &cancel,
            )
        })();
        if cancel.cancelled() {
            continue;
        }
        match observation {
            Ok(value) => {
                if let Err(error) = engine.observe_checked(
                    value,
                    started.elapsed().as_millis() as u64,
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                    &|| cancel.cancelled(),
                ) {
                    engine.snapshot.status_message = error;
                }
            }
            Err(error) => {
                adapter = None;
                engine.unavailable(error);
            }
        }
        if !engine.settings.tracking_enabled {
            adapter = None;
        }
        discovery = false;
        refresh = Instant::now() + Duration::from_secs(1);
        emit(&mut engine, &mut revision, processed);
    }
    drop(adapter);
    engine.unavailable("Tracking stopped");
    engine.snapshot.tracking_enabled = false;
    emit(&mut engine, &mut revision, epoch.load(Ordering::SeqCst));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queued_stop_cannot_lend_its_epoch_to_an_earlier_settings_request() {
        let dir = tempfile::tempdir().unwrap();
        let db = Storage::open(dir.path().join("db")).unwrap();
        let calls = Arc::new(AtomicU64::new(0));
        let count = calls.clone();
        let (entered_tx, entered_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let release = Mutex::new(release_rx);
        let runtime = Runtime::spawn(
            db,
            Arc::new(|_| {}),
            Box::new(move |_| {
                count.fetch_add(1, Ordering::SeqCst);
                let _ = entered_tx.try_send(());
                let _ = release.lock().unwrap().recv_timeout(Duration::from_secs(2));
                Err("test unavailable")
            }),
        )
        .unwrap();
        runtime
            .action(RuntimeAction::Tracking { enabled: true })
            .unwrap();
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let (first, first_reply) = mpsc::sync_channel(1);
        let (stop, stop_reply) = mpsc::sync_channel(1);
        let generation = runtime.0.epoch.fetch_add(1, Ordering::SeqCst) + 1;
        runtime
            .0
            .sender
            .send(Request::Action(
                generation,
                RuntimeAction::AutoOverlay { enabled: false },
                first,
            ))
            .unwrap();
        let generation = runtime.0.epoch.fetch_add(1, Ordering::SeqCst) + 1;
        runtime
            .0
            .sender
            .send(Request::Action(
                generation,
                RuntimeAction::Tracking { enabled: false },
                stop,
            ))
            .unwrap();
        release_tx.send(()).unwrap();
        first_reply
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap();
        stop_reply
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(!runtime.snapshot().tracking_enabled);
        runtime.shutdown();
    }
}
