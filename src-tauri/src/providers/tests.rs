use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
fn recording() -> Recording {
    Recording {
        id: "fixture".into(),
        title: "Original fixture".into(),
        artists: vec!["Fixture artist".into()],
        album: Some("Fixture album".into()),
        duration_ms: Some(singalong_core::model::Counter::new(120000)),
        file_name: None,
        version_label: None,
    }
}
fn cancel() -> Cancellation {
    Cancellation {
        epoch: Arc::new(AtomicU64::new(0)),
        expected: 0,
    }
}
const SONG: &str = r#"[{"id":1,"trackName":"Original fixture","artistName":"Fixture artist","albumName":"Fixture album","duration":120,"syncedLyrics":"[00:01]Original synthetic line"}]"#;
struct Server {
    url: String,
    hits: Arc<AtomicU64>,
    stop: Arc<AtomicU64>,
    thread: Option<thread::JoinHandle<()>>,
}
impl Server {
    fn new(replies: Vec<(u16, String, String, u64)>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let hits = Arc::new(AtomicU64::new(0));
        let stop = Arc::new(AtomicU64::new(0));
        let (count, done) = (hits.clone(), stop.clone());
        let thread = thread::spawn(move || {
            let mut responses = replies.into_iter();
            while done.load(Ordering::SeqCst) == 0 {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_nonblocking(false).unwrap();
                        stream
                            .set_read_timeout(Some(Duration::from_secs(1)))
                            .unwrap();
                        let mut buffer = [0; 8192];
                        let mut used = 0;
                        while used < buffer.len()
                            && !buffer[..used].windows(4).any(|w| w == b"\r\n\r\n")
                        {
                            match stream.read(&mut buffer[used..]) {
                                Ok(0) | Err(_) => break,
                                Ok(n) => used += n,
                            }
                        }
                        if !buffer[..used].windows(4).any(|w| w == b"\r\n\r\n") {
                            continue;
                        }
                        count.fetch_add(1, Ordering::SeqCst);
                        let Some((status, body, headers, delay)) = responses.next() else {
                            break;
                        };
                        thread::sleep(Duration::from_millis(delay));
                        let response = format!(
                            "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}",
                            body.len()
                        );
                        let _ = stream.write_all(response.as_bytes());
                    }
                    Err(_) => thread::sleep(Duration::from_millis(2)),
                }
            }
        });
        Self {
            url,
            hits,
            stop,
            thread: Some(thread),
        }
    }
    fn client(&self) -> ProviderClient {
        ProviderClient::for_test(&self.url, Duration::from_secs(2), Duration::from_millis(15))
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(1, Ordering::SeqCst);
        let _ = self.thread.take().unwrap().join();
    }
}
fn reply(status: u16, body: &str) -> (u16, String, String, u64) {
    (status, body.into(), String::new(), 0)
}
#[tokio::test]
async fn primary_match_stops_fallback_and_retains_provenance() {
    let server = Server::new(vec![reply(200, SONG)]);
    let mut client = server.client();
    let LookupOutcome::Found(versions) = client.lookup(&recording(), &cancel()).await else {
        panic!("expected lyrics")
    };
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].source_name.as_deref(), Some("LRCLIB"));
    assert_eq!(server.hits.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn fallback_is_uncertain_and_errors_never_become_misses() {
    let server = Server::new(vec![
        reply(404, "{}"),
        reply(200, r#"{"lyrics":"Original fallback fixture"}"#),
    ]);
    let mut client = server.client();
    let LookupOutcome::Found(versions) = client.lookup(&recording(), &cancel()).await else {
        panic!("expected fallback")
    };
    assert!(matches!(versions[0].confidence, MatchConfidence::Uncertain));
    assert_eq!(versions[0].source_name.as_deref(), Some("lyrics.ovh"));
    for primary in [
        reply(500, "{}"),
        reply(200, "not JSON"),
        reply(200, &"x".repeat(2_000_001)),
        reply(302, "{}"),
    ] {
        let server = Server::new(vec![primary, reply(404, "{}")]);
        assert!(matches!(
            server.client().lookup(&recording(), &cancel()).await,
            LookupOutcome::Failed(_)
        ));
    }
    let server = Server::new(vec![reply(404, "{}"), reply(404, "{}")]);
    assert!(matches!(
        server.client().lookup(&recording(), &cancel()).await,
        LookupOutcome::NotFound
    ));
}
#[tokio::test]
async fn rate_limits_survive_cancellation_and_do_not_send_early_requests() {
    let server = Server::new(vec![
        (429, "{}".into(), "Retry-After: 60\r\n".into(), 0),
        (503, "{}".into(), "Retry-After: 60\r\n".into(), 0),
    ]);
    let mut client = server.client();
    for _ in 0..2 {
        let LookupOutcome::Failed(f) = client.lookup(&recording(), &cancel()).await else {
            panic!("expected cooldown")
        };
        assert!(f.retry_after_ms.unwrap() >= 59000);
    }
    assert_eq!(server.hits.load(Ordering::SeqCst), 2);
}
#[tokio::test]
async fn cancellation_and_timeout_drop_inflight_http() {
    let server = Server::new(vec![(200, SONG.into(), String::new(), 250)]);
    let mut client = server.client();
    let token = cancel();
    let epoch = token.epoch.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(40));
        epoch.store(1, Ordering::SeqCst);
    });
    let start = Instant::now();
    assert!(matches!(
        client.lookup(&recording(), &token).await,
        LookupOutcome::Cancelled
    ));
    assert!(start.elapsed() < Duration::from_millis(170));
    let server = Server::new(vec![
        (200, SONG.into(), String::new(), 250),
        reply(404, "{}"),
    ]);
    assert!(matches!(
        ProviderClient::for_test(
            &server.url,
            Duration::from_millis(180),
            Duration::from_millis(15)
        )
        .lookup(&recording(), &cancel())
        .await,
        LookupOutcome::Failed(_)
    ));
}
#[test]
fn retry_after_accepts_seconds_and_http_dates() {
    assert_eq!(retry_delay(Some("60"), 429).as_secs(), 60);
    let now = std::time::SystemTime::now() + Duration::from_secs(70);
    let value = httpdate::fmt_http_date(now);
    assert!((68..=70).contains(&retry_delay(Some(&value), 503).as_secs()));
    assert_eq!(retry_delay(Some("invalid"), 429).as_secs(), 60);
}

#[test]
fn worker_cancels_old_song_and_serializes_requests() {
    let server = Server::new(vec![
        (200, SONG.into(), String::new(), 100),
        reply(200, SONG),
    ]);
    let worker = Worker::with_client(server.client()).unwrap();
    worker.start(recording()).unwrap();
    let until = Instant::now() + Duration::from_secs(2);
    while server.hits.load(Ordering::SeqCst) == 0 {
        assert!(Instant::now() < until);
        thread::sleep(Duration::from_millis(5));
    }
    worker.cancel();
    let current = worker.start(recording()).unwrap();
    loop {
        if let Some((id, result)) = worker.poll() {
            assert_eq!(id, current);
            assert!(matches!(result, LookupOutcome::Found(_)));
            break;
        }
        assert!(Instant::now() < until);
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(server.hits.load(Ordering::SeqCst), 2);
}

#[test]
fn a_completed_cancelled_result_cannot_block_the_next_song() {
    let server = Server::new(vec![reply(200, SONG), reply(200, SONG)]);
    let worker = Worker::with_client(server.client()).unwrap();
    worker.start(recording()).unwrap();
    thread::sleep(Duration::from_millis(150));
    let id = worker.start(recording()).unwrap();
    thread::sleep(Duration::from_millis(150));
    assert_eq!(worker.poll().map(|(current, _)| current), Some(id));
}

#[tokio::test]
async fn primary_failure_can_use_fallback_and_cancelled_pacing_keeps_cooldown() {
    let server = Server::new(vec![
        reply(500, "{}"),
        reply(200, r#"{"lyrics":"Original fallback fixture"}"#),
    ]);
    assert!(matches!(
        server.client().lookup(&recording(), &cancel()).await,
        LookupOutcome::Found(_)
    ));
    let server = Server::new(vec![
        (429, "{}".into(), "Retry-After: 60\r\n".into(), 0),
        reply(200, r#"{"lyrics":"Original fallback fixture"}"#),
    ]);
    let mut client = ProviderClient::for_test(
        &server.url,
        Duration::from_secs(2),
        Duration::from_millis(200),
    );
    let token = cancel();
    let epoch = token.epoch.clone();
    let hits = server.hits.clone();
    let canceller = thread::spawn(move || {
        while hits.load(Ordering::SeqCst) == 0 {
            thread::sleep(Duration::from_millis(2));
        }
        thread::sleep(Duration::from_millis(30));
        epoch.store(1, Ordering::SeqCst);
    });
    let began = Instant::now();
    assert!(matches!(
        client.lookup(&recording(), &token).await,
        LookupOutcome::Cancelled
    ));
    canceller.join().unwrap();
    assert_eq!(server.hits.load(Ordering::SeqCst), 1);
    assert!(matches!(
        client.lookup(&recording(), &cancel()).await,
        LookupOutcome::Found(_)
    ));
    assert!(began.elapsed() >= Duration::from_millis(190));
    assert_eq!(server.hits.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn chunked_oversize_and_stalled_response_bodies_are_bounded() {
    for stall in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut bytes = [0; 8192];
            let _ = stream.read(&mut bytes);
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").unwrap();
            if stall {
                thread::sleep(Duration::from_millis(300));
            } else {
                let body = "x".repeat(100_000);
                let chunk = format!("{:x}\r\n{body}\r\n", body.len());
                for _ in 0..21 {
                    if stream.write_all(chunk.as_bytes()).is_err() {
                        break;
                    }
                }
                let _ = stream.write_all(b"0\r\n\r\n");
            }
        });
        let mut client =
            ProviderClient::for_test(&base, Duration::from_millis(180), Duration::ZERO);
        let error = client
            .json(0, Url::parse(&base).unwrap(), &cancel())
            .await
            .unwrap_err();
        assert_eq!(
            error.kind,
            if stall {
                FailureKind::Network
            } else {
                FailureKind::InvalidResponse
            }
        );
        server.join().unwrap();
    }
}
