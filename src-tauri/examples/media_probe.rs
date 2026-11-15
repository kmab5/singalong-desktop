//! Read-only GSMTC probe. No media control, artwork, titles or artists are logged.
#[cfg(not(windows))]
fn main() {
    eprintln!("This feasibility probe requires Windows.");
    std::process::exit(1);
}

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    native::run()
}

#[cfg(windows)]
mod native {
    use serde_json::json;
    use singalong_desktop::position_ms;
    use std::{
        sync::mpsc,
        time::{Duration, Instant},
    };
    use windows::{
        Foundation::TypedEventHandler,
        Media::Control::{
            GlobalSystemMediaTransportControlsSession as Session,
            GlobalSystemMediaTransportControlsSessionManager as Manager,
        },
        Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
    };

    // The synchronous WinRT get() has no deadline. Poll before GetResults so a
    // stalled provider cannot strand the probe or its callback registrations.
    macro_rules! wait_operation {
        ($operation:expr) => {{
            let operation = $operation?;
            let deadline = Instant::now() + Duration::from_secs(3);
            while operation.Status()? == windows_future::AsyncStatus::Started {
                if Instant::now() >= deadline {
                    let _ = operation.Cancel();
                    return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                        0x800705B4u32 as i32,
                    )));
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            operation.GetResults()
        }};
    }

    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            // SAFETY: balances successful RoInitialize on this same thread.
            unsafe { RoUninitialize() };
        }
    }

    type Event = (&'static str, Option<u64>, Instant);

    struct Subscription {
        session: Session,
        id: u64,
        playback: Option<i64>,
        timeline: Option<i64>,
        metadata: Option<i64>,
    }

    fn cleanup(result: windows::core::Result<()>) {
        if let Err(error) = result {
            eprintln!(
                "{}",
                json!({"event":"cleanup_error", "hresult":error.code().0})
            );
        }
    }

    impl Drop for Subscription {
        fn drop(&mut self) {
            if let Some(token) = self.playback {
                cleanup(self.session.RemovePlaybackInfoChanged(token));
            }
            if let Some(token) = self.timeline {
                cleanup(self.session.RemoveTimelinePropertiesChanged(token));
            }
            if let Some(token) = self.metadata {
                cleanup(self.session.RemoveMediaPropertiesChanged(token));
            }
        }
    }

    impl Subscription {
        fn new(
            session: Session,
            id: u64,
            tx: &mpsc::SyncSender<Event>,
        ) -> windows::core::Result<Self> {
            // Build the guard first so partial registration failure also cleans up.
            let mut subscription = Self {
                session,
                id,
                playback: None,
                timeline: None,
                metadata: None,
            };
            let sender = tx.clone();
            subscription.playback = Some(subscription.session.PlaybackInfoChanged(
                &TypedEventHandler::new(move |_, _| {
                    let _ = sender.try_send(("playback", Some(id), Instant::now()));
                    Ok(())
                }),
            )?);
            let sender = tx.clone();
            subscription.timeline = Some(subscription.session.TimelinePropertiesChanged(
                &TypedEventHandler::new(move |_, _| {
                    let _ = sender.try_send(("timeline", Some(id), Instant::now()));
                    Ok(())
                }),
            )?);
            let sender = tx.clone();
            subscription.metadata = Some(subscription.session.MediaPropertiesChanged(
                &TypedEventHandler::new(move |_, _| {
                    let _ = sender.try_send(("metadata", Some(id), Instant::now()));
                    Ok(())
                }),
            )?);
            Ok(subscription)
        }

        fn snapshot(&self, started: Instant) {
            let app = self
                .session
                .SourceAppUserModelId()
                .ok()
                .map(|value| value.to_string());
            let playback = self.session.GetPlaybackInfo().ok();
            let timeline = self.session.GetTimelineProperties().ok();
            let metadata = self.metadata_presence();
            println!(
                "{}",
                json!({
                    "event": "snapshot", "session_id": self.id,
                    "player_id": app.as_ref().map(|value| value.rsplit(['/', '\\']).next().unwrap_or(value)),
                    "player_id_path_redacted": app.as_ref().is_some_and(|value| value.contains(['/', '\\'])),
                    "observed_monotonic_ms": started.elapsed().as_millis(),
                    "metadata_present": metadata.ok(),
                    "status": playback.as_ref().and_then(|value| value.PlaybackStatus().ok()).map(|value| format!("{value:?}")),
                    "rate": playback.as_ref().and_then(|value| value.PlaybackRate().ok()).and_then(|value| value.Value().ok()).filter(|value| value.is_finite()),
                    "position_ms": timeline.as_ref().and_then(|value| value.Position().ok()).and_then(|value| position_ms(value.Duration)),
                    "end_ms": timeline.as_ref().and_then(|value| value.EndTime().ok()).and_then(|value| position_ms(value.Duration)),
                    "timeline_updated_windows_ticks": timeline.as_ref().and_then(|value| value.LastUpdatedTime().ok()).map(|value| value.UniversalTime.to_string()),
                })
            );
        }

        fn metadata_presence(&self) -> windows::core::Result<serde_json::Value> {
            let metadata = wait_operation!(self.session.TryGetMediaPropertiesAsync())?;
            Ok(json!({
                "title": !metadata.Title()?.is_empty(),
                "artist": !metadata.Artist()?.is_empty(),
                "album": !metadata.AlbumTitle()?.is_empty()
            }))
        }
    }

    struct ManagerGuard {
        manager: Manager,
        token: i64,
    }
    impl Drop for ManagerGuard {
        fn drop(&mut self) {
            cleanup(self.manager.RemoveSessionsChanged(self.token));
        }
    }

    fn request_manager() -> windows::core::Result<Manager> {
        wait_operation!(Manager::RequestAsync())
    }

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let seconds: u64 = std::env::args()
            .nth(1)
            .map(|value| value.parse())
            .transpose()?
            .unwrap_or(15);
        if !(1..=300).contains(&seconds) {
            return Err("Duration must be 1..=300 seconds".into());
        }
        // SAFETY: dedicated executable main thread; guard balances initialization.
        unsafe {
            RoInitialize(RO_INIT_MULTITHREADED)?;
        }
        let _apartment = Apartment;
        let started = Instant::now();
        let manager = request_manager()?;
        let (tx, rx) = mpsc::sync_channel::<Event>(128);
        let sender = tx.clone();
        let token = manager.SessionsChanged(&TypedEventHandler::new(move |_, _| {
            let _ = sender.try_send(("sessions", None, Instant::now()));
            Ok(())
        }))?;
        let guard = ManagerGuard { manager, token };
        let mut subscriptions: Vec<Subscription> = Vec::new();
        let mut next_id = 1;
        println!(
            "{}",
            json!({"event":"started", "duration_seconds": seconds, "metadata":"presence_only"})
        );
        while started.elapsed() < Duration::from_secs(seconds) {
            let sessions: Vec<_> = guard.manager.GetSessions()?.into_iter().collect();
            subscriptions.retain(|item| sessions.contains(&item.session));
            for session in sessions {
                if !subscriptions.iter().any(|item| item.session == session) {
                    subscriptions.push(Subscription::new(session, next_id, &tx)?);
                    next_id += 1;
                }
            }
            println!(
                "{}",
                json!({"event":"session_count", "count":subscriptions.len()})
            );
            for subscription in &subscriptions {
                subscription.snapshot(started);
            }
            // Periodic reconciliation covers coalesced/dropped notifications.
            let remaining = Duration::from_secs(seconds).saturating_sub(started.elapsed());
            if let Ok((kind, id, at)) = rx.recv_timeout(remaining.min(Duration::from_secs(1))) {
                println!(
                    "{}",
                    json!({"event":kind,"session_id":id,"monotonic_ms":at.duration_since(started).as_millis()})
                );
            }
        }
        drop(subscriptions);
        drop(guard);
        drop(rx);
        println!(
            "{}",
            json!({"event":"stopped", "elapsed_ms":started.elapsed().as_millis()})
        );
        Ok(())
    }
}
