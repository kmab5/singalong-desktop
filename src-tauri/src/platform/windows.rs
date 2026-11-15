use super::*;
use crate::position_ms;
use ::windows::{
    Foundation::TypedEventHandler,
    Media::Control::{
        GlobalSystemMediaTransportControlsSession as Session,
        GlobalSystemMediaTransportControlsSessionManager as Manager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
    },
    Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
};
use singalong_core::clock::PlaybackClock;
use std::{
    sync::mpsc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
macro_rules! wait {
    ($operation:expr, $cancel:expr) => {{
        let operation = $operation.map_err(|_| "Windows media access failed.")?;
        let deadline = Instant::now() + Duration::from_secs(3);
        while operation
            .Status()
            .map_err(|_| "Windows media access failed.")?
            == windows_future::AsyncStatus::Started
        {
            if $cancel.cancelled() || Instant::now() >= deadline {
                let _ = operation.Cancel();
                return Err("Media refresh cancelled or timed out.");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        if $cancel.cancelled() {
            return Err("Media refresh cancelled.");
        }
        operation
            .GetResults()
            .map_err(|_| "Windows media access failed.")
    }};
}
struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            RoUninitialize();
        }
    }
}
struct Subscription {
    session: Session,
    id: SessionId,
    playback: Option<i64>,
    timeline: Option<i64>,
    metadata: Option<i64>,
    clock: PlaybackClock,
    last: Option<PlaybackSnapshot>,
    revision: Arc<AtomicU64>,
}
impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(t) = self.playback {
            let _ = self.session.RemovePlaybackInfoChanged(t);
        }
        if let Some(t) = self.timeline {
            let _ = self.session.RemoveTimelinePropertiesChanged(t);
        }
        if let Some(t) = self.metadata {
            let _ = self.session.RemoveMediaPropertiesChanged(t);
        }
    }
}
impl Subscription {
    fn new(session: Session, tx: &mpsc::SyncSender<()>) -> Result<Self, &'static str> {
        let mut s = Self {
            session,
            id: SessionId(uuid::Uuid::new_v4().to_string()),
            playback: None,
            timeline: None,
            metadata: None,
            clock: PlaybackClock::default(),
            last: None,
            revision: Arc::new(AtomicU64::new(0)),
        };
        let sender = tx.clone();
        let revision = s.revision.clone();
        s.playback = Some(
            s.session
                .PlaybackInfoChanged(&TypedEventHandler::new(move |_, _| {
                    revision.fetch_add(1, Ordering::SeqCst);
                    let _ = sender.try_send(());
                    Ok(())
                }))
                .map_err(|_| "Could not observe playback.")?,
        );
        let sender = tx.clone();
        let revision = s.revision.clone();
        s.timeline = Some(
            s.session
                .TimelinePropertiesChanged(&TypedEventHandler::new(move |_, _| {
                    revision.fetch_add(1, Ordering::SeqCst);
                    let _ = sender.try_send(());
                    Ok(())
                }))
                .map_err(|_| "Could not observe timing.")?,
        );
        let sender = tx.clone();
        let revision = s.revision.clone();
        s.metadata = Some(
            s.session
                .MediaPropertiesChanged(&TypedEventHandler::new(move |_, _| {
                    revision.fetch_add(1, Ordering::SeqCst);
                    let _ = sender.try_send(());
                    Ok(())
                }))
                .map_err(|_| "Could not observe metadata.")?,
        );
        Ok(s)
    }
}
// Fields are dropped in declaration order: subscriptions/manager before apartment.
pub struct WindowsAdapter {
    subscriptions: Vec<Subscription>,
    manager: Manager,
    token: i64,
    tx: mpsc::SyncSender<()>,
    rx: mpsc::Receiver<()>,
    _apartment: Apartment,
}
impl Drop for WindowsAdapter {
    fn drop(&mut self) {
        let _ = self.manager.RemoveSessionsChanged(self.token);
    }
}
impl WindowsAdapter {
    pub fn new(cancel: &Cancellation) -> Result<Self, &'static str> {
        // Dedicated coordinator thread owns apartment and all event registrations.
        unsafe {
            RoInitialize(RO_INIT_MULTITHREADED)
                .map_err(|_| "Could not initialize Windows media observation.")?;
        }
        let apartment = Apartment;
        let manager = wait!(Manager::RequestAsync(), cancel)?;
        let (tx, rx) = mpsc::sync_channel(1);
        let sender = tx.clone();
        let token = manager
            .SessionsChanged(&TypedEventHandler::new(move |_, _| {
                let _ = sender.try_send(());
                Ok(())
            }))
            .map_err(|_| "Could not observe Windows sessions.")?;
        Ok(Self {
            subscriptions: vec![],
            manager,
            token,
            tx,
            rx,
            _apartment: apartment,
        })
    }
}
impl Adapter for WindowsAdapter {
    fn changed(&self) -> bool {
        self.rx.try_recv().is_ok()
    }
    fn refresh(
        &mut self,
        enabled: &[PlayerId],
        now: u64,
        cancel: &Cancellation,
    ) -> Result<Observation, &'static str> {
        let sessions: Vec<_> = self
            .manager
            .GetSessions()
            .map_err(|_| "Windows media access is unavailable. Try restarting tracking.")?
            .into_iter()
            .collect();
        self.subscriptions.retain(|s| sessions.contains(&s.session));
        for session in sessions.into_iter().take(64) {
            if cancel.cancelled() {
                return Err("Media refresh cancelled.");
            }
            if !self.subscriptions.iter().any(|s| s.session == session)
                && let Ok(subscription) = Subscription::new(session, &self.tx)
            {
                self.subscriptions.push(subscription);
            }
        }
        let mut result = Observation {
            players: vec![],
            sessions: vec![],
        };
        for sub in &mut self.subscriptions {
            if cancel.cancelled() {
                return Err("Media refresh cancelled.");
            }
            let Ok(raw_id) = sub.session.SourceAppUserModelId() else {
                continue;
            };
            let native_id = raw_id.to_string();
            let name = player_name(&native_id);
            let player_id = PlayerId {
                platform: Platform::Windows,
                native_id,
            };
            let allowed = enabled.contains(&player_id);
            if !result.players.iter().any(|p| p.id == player_id) {
                result.players.push(PlayerInfo {
                    id: player_id.clone(),
                    name: name.clone(),
                    enabled: allowed,
                    available: true,
                });
            }
            if !allowed {
                continue;
            }
            let fallback_id = player_id.clone();
            let fallback_name = name.clone();
            let revision = sub.revision.load(Ordering::SeqCst);
            let read = (|| -> Result<PlaybackSnapshot, &'static str> {
                let info = sub
                    .session
                    .GetPlaybackInfo()
                    .map_err(|_| "Playback refresh failed; timing is unavailable.")?;
                let status = match info.PlaybackStatus().ok() {
                    Some(Status::Playing) => PlaybackStatus::Playing,
                    Some(Status::Paused) => PlaybackStatus::Paused,
                    Some(Status::Stopped | Status::Closed) => PlaybackStatus::Stopped,
                    _ => PlaybackStatus::Unknown,
                };
                let rate = info
                    .PlaybackRate()
                    .ok()
                    .and_then(|r| r.Value().ok())
                    .filter(|r| r.is_finite() && *r >= 0.0 && *r <= 16.0);
                let metadata = wait!(sub.session.TryGetMediaPropertiesAsync(), cancel)?;
                let timeline = sub.session.GetTimelineProperties().ok();
                let start = timeline
                    .as_ref()
                    .and_then(|t| t.StartTime().ok())
                    .and_then(|v| position_ms(v.Duration));
                let end = timeline
                    .as_ref()
                    .and_then(|t| t.EndTime().ok())
                    .and_then(|v| position_ms(v.Duration));
                let duration = if start == Some(0) {
                    end.filter(|v| *v > 0)
                } else {
                    None
                };
                let raw = timeline
                    .as_ref()
                    .and_then(|t| {
                        Some((
                            position_ms(t.Position().ok()?.Duration)?,
                            t.LastUpdatedTime().ok()?.UniversalTime,
                        ))
                    })
                    .filter(|(p, _)| duration.is_none_or(|d| *p <= d));
                let utc_ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i128
                    + 11644473600000i128;
                let age = raw.map_or(u64::MAX, |(_, ticks)| {
                    let age = utc_ms - i128::from(ticks) / 10000;
                    if !(0..=15000).contains(&age) {
                        u64::MAX
                    } else {
                        age as u64
                    }
                });
                let position = sub
                    .clock
                    .refresh(raw, status.clone(), rate, now, age)
                    .map(|p| duration.map_or(p, |d| p.min(d)));
                let title = metadata
                    .Title()
                    .ok()
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let artist = metadata
                    .Artist()
                    .ok()
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let album = metadata
                    .AlbumTitle()
                    .ok()
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty());
                let recording = (!title.trim().is_empty()).then(|| Recording {
                    id: String::new(),
                    title,
                    artists: if artist.is_empty() {
                        vec![]
                    } else {
                        vec![artist]
                    },
                    album,
                    duration_ms: duration.map(Counter::new),
                    version_label: None,
                    file_name: None,
                });
                let id_lower = player_id.native_id.to_lowercase();
                let browser = ["chrome", "msedge", "firefox", "brave", "opera", "vivaldi"]
                    .iter()
                    .any(|v| id_lower.contains(v));
                if sub.revision.load(Ordering::SeqCst) != revision {
                    return Err("Playback changed during refresh.");
                }
                Ok(PlaybackSnapshot {
                    session_id: sub.id.clone(),
                    player_id,
                    player_name: name,
                    recording,
                    status,
                    position_ms: position.map(Counter::new),
                    observed_at_ms: Counter::new(now),
                    rate,
                    stream_pin_available: !browser,
                })
            })();
            match read {
                Ok(snapshot) => {
                    sub.last = Some(snapshot.clone());
                    result.sessions.push(snapshot);
                }
                Err(_) => {
                    let mut snapshot = sub.last.clone().unwrap_or(PlaybackSnapshot {
                        session_id: sub.id.clone(),
                        player_id: fallback_id,
                        player_name: fallback_name,
                        recording: None,
                        status: PlaybackStatus::Unknown,
                        position_ms: None,
                        observed_at_ms: Counter::new(now),
                        rate: None,
                        stream_pin_available: false,
                    });
                    snapshot.status = PlaybackStatus::Unknown;
                    snapshot.position_ms = None;
                    snapshot.rate = None;
                    snapshot.observed_at_ms = Counter::new(now);
                    result.sessions.push(snapshot);
                }
            }
        }
        Ok(result)
    }
}
pub fn player_name(id: &str) -> String {
    let lower = id.to_lowercase();
    for (key, name) in [
        ("zunemusic", "Media Player"),
        ("spotify", "Spotify"),
        ("chrome", "Chrome"),
        ("msedge", "Edge"),
        ("firefox", "Firefox"),
        ("vlc", "VLC"),
        ("kmplayer", "KMPlayer"),
    ] {
        if lower.contains(key) {
            return name.into();
        }
    }
    id.rsplit(['/', '\\']).next().unwrap_or(id).to_string()
}
