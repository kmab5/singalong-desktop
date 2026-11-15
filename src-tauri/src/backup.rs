//! A single cancellable backup job. Preview owns immutable decoded data and its digest.
use crate::{platform::Cancellation, runtime::Runtime, storage::Storage};
use sha2::{Digest, Sha256};
use singalong_core::{
    backup::{self, BackupBudget, BackupControl, DecodedBackup, ParseLimits},
    model::*,
};
use std::{
    io::Read,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::JoinHandle,
};
struct Preview {
    token: String,
    digest: String,
    data: Arc<DecodedBackup>,
}
struct State {
    progress: BackupProgress,
    preview: Option<Preview>,
}
struct Shared {
    state: Mutex<State>,
    epoch: Arc<AtomicU64>,
    busy: AtomicBool,
    closing: AtomicBool,
}
pub struct Manager {
    shared: Arc<Shared>,
    thread: Mutex<Option<JoinHandle<()>>>,
    db: Storage,
    runtime: Runtime,
}
enum Operation {
    Preview(BackupLimits),
    Export(BackupLimits),
    Apply { token: String, settings: bool },
}
impl Manager {
    pub fn new(db: Storage, runtime: Runtime) -> Self {
        Self {
            shared: Arc::new(Shared {
                state: Mutex::new(State {
                    progress: BackupProgress::default(),
                    preview: None,
                }),
                epoch: Arc::new(AtomicU64::new(0)),
                busy: AtomicBool::new(false),
                closing: AtomicBool::new(false),
            }),
            thread: Mutex::new(None),
            db,
            runtime,
        }
    }
    pub fn poll(&self) -> BackupProgress {
        self.shared.state.lock().unwrap().progress.clone()
    }
    pub fn cancel(&self) {
        self.shared.epoch.fetch_add(1, Ordering::SeqCst);
        let mut state = self.shared.state.lock().unwrap();
        state.preview = None;
        state.progress = BackupProgress {
            stage: if self.shared.busy.load(Ordering::SeqCst) {
                BackupStage::Cancelling
            } else {
                BackupStage::Idle
            },
            message: "Backup operation cancelled.".into(),
            ..Default::default()
        };
    }
    pub fn begin_shutdown(&self) {
        let _guard = self.thread.lock().unwrap();
        self.shared.closing.store(true, Ordering::SeqCst);
        self.cancel();
    }
    pub fn shutdown(&self) {
        self.begin_shutdown();
        if let Some(thread) = self.thread.lock().unwrap().take() {
            let _ = thread.join();
        }
    }
    pub fn preview(&self, limits: BackupLimits) -> Result<BackupProgress, String> {
        self.start(Operation::Preview(limits))
    }
    pub fn export(&self, limits: BackupLimits) -> Result<BackupProgress, String> {
        self.start(Operation::Export(limits))
    }
    pub fn apply(&self, token: String, settings: bool) -> Result<BackupProgress, String> {
        self.start(Operation::Apply { token, settings })
    }
    fn start(&self, operation: Operation) -> Result<BackupProgress, String> {
        let mut thread = self
            .thread
            .lock()
            .map_err(|_| "Backup worker unavailable.")?;
        if self.shared.closing.load(Ordering::SeqCst) {
            return Err("Singalong is quitting.".into());
        }
        if self.shared.busy.swap(true, Ordering::SeqCst) {
            return Err("Wait for the current backup operation to finish or cancel.".into());
        }
        if let Some(previous) = thread.take() {
            let _ = previous.join();
        }
        let epoch = self.shared.epoch.fetch_add(1, Ordering::SeqCst) + 1;
        let cancel = Cancellation {
            epoch: self.shared.epoch.clone(),
            expected: epoch,
        };
        self.shared.state.lock().unwrap().progress = BackupProgress {
            stage: BackupStage::Busy,
            message: "Preparing backup operation…".into(),
            ..Default::default()
        };
        let shared = self.shared.clone();
        let db = self.db.clone();
        let runtime = self.runtime.clone();
        let job = std::thread::Builder::new()
            .name("singalong-backup".into())
            .spawn(move || {
                let result = run(operation, &db, &runtime, &shared, &cancel);
                let mut state = shared.state.lock().unwrap();
                let committed = result
                    .as_ref()
                    .is_ok_and(|progress| matches!(progress.stage, BackupStage::Done));
                if cancel.cancelled() && !committed {
                    state.preview = None;
                    state.progress = BackupProgress {
                        message: "Backup operation cancelled.".into(),
                        ..Default::default()
                    };
                } else {
                    match result {
                        Ok(progress) => state.progress = progress,
                        Err(error) => {
                            state.preview = None;
                            state.progress = BackupProgress {
                                stage: BackupStage::Failed,
                                message: error,
                                ..Default::default()
                            };
                        }
                    }
                }
                shared.busy.store(false, Ordering::SeqCst);
            });
        match job {
            Ok(job) => *thread = Some(job),
            Err(_) => {
                self.shared.busy.store(false, Ordering::SeqCst);
                return Err("Could not start backup operation.".into());
            }
        }
        Ok(self.poll())
    }
}
impl Drop for Manager {
    fn drop(&mut self) {
        self.shutdown();
    }
}
fn limits(value: &BackupLimits) -> Result<(BackupBudget, ParseLimits), String> {
    let bytes = value.max_bytes.as_ref().map(Counter::value);
    let nodes = value.max_nodes.value();
    let strings = value.max_string_bytes.value();
    if bytes == Some(0) || nodes == 0 || strings == 0 {
        return Err(
            "Backup byte and parser limits must be positive, or explicitly unlimited bytes.".into(),
        );
    }
    Ok((
        BackupBudget { max_bytes: bytes },
        ParseLimits {
            max_nodes: nodes,
            max_string_bytes: strings,
        },
    ))
}
fn run(
    operation: Operation,
    db: &Storage,
    runtime: &Runtime,
    shared: &Shared,
    cancel: &Cancellation,
) -> Result<BackupProgress, String> {
    match operation {
        Operation::Preview(value) => {
            let (budget, parse_limits) = limits(&value)?;
            shared.state.lock().unwrap().preview = None;
            let Some(path) = crate::files::choose(false, false, Some(cancel.clone()))? else {
                return Ok(BackupProgress::default());
            };
            let mut file =
                std::fs::File::open(path).map_err(|_| "Could not read the selected backup.")?;
            let input_limit = budget
                .max_bytes
                .map(|n| n.saturating_mul(2).saturating_add(4 * 1024 * 1024));
            let mut bytes = Vec::new();
            let mut digest = Sha256::new();
            let mut buffer = [0u8; 16384];
            loop {
                if cancel.cancelled() {
                    return Err("Backup operation cancelled.".into());
                }
                let count = file
                    .read(&mut buffer)
                    .map_err(|_| "Could not read the selected backup.")?;
                if count == 0 {
                    break;
                }
                if input_limit
                    .is_some_and(|limit| (bytes.len() as u64).saturating_add(count as u64) > limit)
                {
                    return Err("Backup exceeds the selected input byte budget.".into());
                }
                bytes
                    .try_reserve(count)
                    .map_err(|_| "Not enough memory for the selected backup.")?;
                bytes.extend_from_slice(&buffer[..count]);
                digest.update(&buffer[..count]);
            }
            let cancelled = || cancel.cancelled();
            let control = BackupControl {
                is_cancelled: &cancelled,
                parse_limits,
            };
            let decoded = Arc::new(
                backup::decode_bytes_with_control(&bytes, budget, &control)
                    .map_err(|e| e.to_string())?,
            );
            drop(bytes);
            let digest = format!("{:x}", digest.finalize());
            let merge = db
                .backup_merge(Arc::new(decoded.data.clone()), false, true, cancel.clone())
                .map_err(|e| e.to_string())?;
            let token = uuid::Uuid::new_v4().to_string();
            let summary = &decoded.summary;
            let mut lines = vec![
                format!("Source: {:?}", decoded.source),
                format!(
                    "{} recordings; {} lyric versions; {} references; {} translations",
                    summary.recordings, summary.versions, summary.references, summary.translations
                ),
                format!(
                    "{} inert preferences; {} unapplied settings",
                    summary.inert_preferences, summary.unapplied_settings
                ),
            ];
            if !summary.omitted_sections.is_empty() {
                lines.push(format!(
                    "Omitted sections: {}",
                    summary.omitted_sections.join(", ")
                ));
            }
            if summary.preferred_only {
                lines.push(
                    "Only preferred lyric versions were included in this source backup.".into(),
                );
            }
            lines.extend(merge.summary());
            lines.push("Tracking, player permissions, startup, network policy and consent remain unchanged.".into());
            if cancel.cancelled() {
                return Err("Backup operation cancelled.".into());
            }
            shared.state.lock().unwrap().preview = Some(Preview {
                token: token.clone(),
                digest,
                data: decoded,
            });
            Ok(BackupProgress {
                stage: BackupStage::Preview,
                token: Some(token),
                message: "Preview ready. Review before merging into your library.".into(),
                summary: lines,
            })
        }
        Operation::Apply { token, settings } => {
            let data = {
                let state = shared.state.lock().unwrap();
                let preview = state
                    .preview
                    .as_ref()
                    .filter(|p| p.token == token && !p.digest.is_empty())
                    .ok_or("This backup preview expired. Select the file again.")?;
                Arc::new(preview.data.data.clone())
            };
            let result = runtime.restore(data, settings, cancel.clone())?;
            shared.state.lock().unwrap().preview = None;
            Ok(BackupProgress {
                stage: BackupStage::Done,
                token: None,
                message: "Backup merged into the local library.".into(),
                summary: result.summary(),
            })
        }
        Operation::Export(value) => {
            let (budget, parse_limits) = limits(&value)?;
            shared.state.lock().unwrap().preview = None;
            let Some(path) = crate::files::choose(true, false, Some(cancel.clone()))? else {
                return Ok(BackupProgress::default());
            };
            let data = db
                .backup_snapshot(cancel.clone())
                .map_err(|e| e.to_string())?;
            let cancelled = || cancel.cancelled();
            let control = BackupControl {
                is_cancelled: &cancelled,
                parse_limits,
            };
            crate::files::atomic_write(
                &path,
                |file| {
                    backup::encode_writer_with_control(file, &data, budget, &control)
                        .map_err(|e| e.to_string())
                },
                cancelled,
            )?;
            Ok(BackupProgress {
                stage: BackupStage::Done,
                message: "Full desktop backup saved.".into(),
                ..Default::default()
            })
        }
    }
}
