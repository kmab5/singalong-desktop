//! Fixed-endpoint provider client. A single worker owns this mutable client so
//! pacing and server cooldowns survive cancelled songs and explicit retries.
use crate::platform::Cancellation;
use reqwest::{Client, Url};
use singalong_core::{
    lyrics::{
        MAX_BYTES,
        matching::{self, ProviderSong},
        queries,
    },
    model::{LyricVersion, MatchConfidence, Recording},
};
use std::time::{Duration, Instant, SystemTime};
mod worker;
pub use worker::Worker;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailureKind {
    Network,
    InvalidResponse,
    Unavailable,
    RateLimited,
    Cancelled,
}
#[derive(Debug, Clone)]
pub struct Failure {
    pub kind: FailureKind,
    pub retry_after_ms: Option<u64>,
}
impl Failure {
    fn new(kind: FailureKind) -> Self {
        Self {
            kind,
            retry_after_ms: None,
        }
    }
}
#[derive(Debug, Clone)]
pub enum LookupOutcome {
    Found(Vec<LyricVersion>),
    Instrumental,
    NotFound,
    InsufficientMetadata,
    Failed(Failure),
    Cancelled,
}
pub struct ProviderClient {
    http: Client,
    bases: [Url; 2],
    blocked: [Option<Instant>; 2],
    next_request: Instant,
    pace: Duration,
}
impl ProviderClient {
    pub fn new() -> Result<Self, reqwest::Error> {
        Self::build(
            [
                Url::parse("https://lrclib.net/api/search").unwrap(),
                Url::parse("https://api.lyrics.ovh/v1/").unwrap(),
            ],
            Duration::from_secs(10),
            Duration::from_millis(750),
        )
    }
    fn build(bases: [Url; 2], timeout: Duration, pace: Duration) -> Result<Self, reqwest::Error> {
        let http = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .timeout(timeout)
            .connect_timeout(timeout)
            .user_agent("SingalongDesktop/0.0.0 (https://github.com/kmab5/singalong-desktop)")
            .build()?;
        Ok(Self {
            http,
            bases,
            blocked: [None, None],
            next_request: Instant::now(),
            pace,
        })
    }
    #[cfg(test)]
    fn for_test(base: &str, timeout: Duration, pace: Duration) -> Self {
        Self::build(
            [
                Url::parse(&format!("{base}/api/search")).unwrap(),
                Url::parse(&format!("{base}/v1/")).unwrap(),
            ],
            timeout,
            pace,
        )
        .unwrap()
    }
    async fn json(
        &mut self,
        source: usize,
        url: Url,
        token: &Cancellation,
    ) -> Result<Option<serde_json::Value>, Failure> {
        let work = async {
            if let Some(until) = self.blocked[source].filter(|t| *t > Instant::now()) {
                return Err(Failure {
                    kind: FailureKind::RateLimited,
                    retry_after_ms: Some(
                        until.saturating_duration_since(Instant::now()).as_millis() as u64,
                    ),
                });
            }
            tokio::time::sleep(self.next_request.saturating_duration_since(Instant::now())).await;
            self.next_request = Instant::now() + self.pace;
            let mut response = self
                .http
                .get(url)
                .send()
                .await
                .map_err(|_| Failure::new(FailureKind::Network))?;
            let status = response.status().as_u16();
            if status == 429 || status == 503 {
                let wait = retry_delay(
                    response
                        .headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok()),
                    status,
                );
                self.blocked[source] = Some(Instant::now() + wait);
                return Err(Failure {
                    kind: if status == 429 {
                        FailureKind::RateLimited
                    } else {
                        FailureKind::Unavailable
                    },
                    retry_after_ms: Some(wait.as_millis() as u64),
                });
            }
            if status == 404 {
                return Ok(None);
            }
            if !(200..300).contains(&status) {
                return Err(Failure::new(FailureKind::Unavailable));
            }
            if !response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .is_some_and(|s| {
                    s.split(';')
                        .next()
                        .is_some_and(|s| s.trim().eq_ignore_ascii_case("application/json"))
                })
                || response
                    .content_length()
                    .is_some_and(|n| n > MAX_BYTES as u64)
            {
                return Err(Failure::new(FailureKind::InvalidResponse));
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| Failure::new(FailureKind::Network))?
            {
                if bytes.len() + chunk.len() > MAX_BYTES {
                    return Err(Failure::new(FailureKind::InvalidResponse));
                }
                bytes.extend_from_slice(&chunk);
            }
            serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|_| Failure::new(FailureKind::InvalidResponse))
        };
        tokio::select! {biased;
            _=cancelled(token)=>Err(Failure::new(FailureKind::Cancelled)),
            value=work=>value,
        }
    }
    async fn search(
        &mut self,
        source: usize,
        recording: &Recording,
        token: &Cancellation,
    ) -> Result<Vec<ProviderSong>, Failure> {
        let title = matching::search_title(recording);
        let artist = recording.artists.join(", ");
        let mut url = self.bases[source].clone();
        if source == 0 {
            url.query_pairs_mut()
                .append_pair("track_name", &title)
                .append_pair("artist_name", &artist);
        } else {
            url.path_segments_mut()
                .unwrap()
                .pop_if_empty()
                .push(&artist)
                .push(&title);
        }
        let Some(value) = self.json(source, url, token).await? else {
            return Ok(vec![]);
        };
        if source == 0 {
            if value.as_array().is_none_or(|v| v.len() > 100) {
                return Err(Failure::new(FailureKind::InvalidResponse));
            }
            let mut songs: Vec<ProviderSong> = serde_json::from_value(value)
                .map_err(|_| Failure::new(FailureKind::InvalidResponse))?;
            for song in &mut songs {
                if song.id < 0 {
                    return Err(Failure::new(FailureKind::InvalidResponse));
                }
                song.source = "LRCLIB".into();
                song.source_url = Some(format!("https://lrclib.net/api/get/{}", song.id));
            }
            Ok(songs)
        } else {
            let text = value
                .get("lyrics")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Failure::new(FailureKind::InvalidResponse))?;
            if text.trim().is_empty() {
                return Ok(vec![]);
            }
            let mut origin = Url::parse("https://api.lyrics.ovh/v1/").unwrap();
            origin
                .path_segments_mut()
                .unwrap()
                .pop_if_empty()
                .push(&artist)
                .push(&title);
            Ok(vec![ProviderSong {
                id: 0,
                track_name: title,
                artist_name: artist,
                album_name: String::new(),
                duration: 0.0,
                instrumental: false,
                plain_lyrics: Some(text.into()),
                synced_lyrics: None,
                lyricsfile: None,
                source: "lyrics.ovh".into(),
                source_url: Some(origin.to_string()),
            }])
        }
    }
    pub async fn lookup(&mut self, recording: &Recording, token: &Cancellation) -> LookupOutcome {
        let plans = queries::plan(recording);
        if plans.is_empty() {
            return LookupOutcome::InsufficientMetadata;
        }
        for query in plans {
            let mut failure: Option<Failure> = None;
            for source in 0..2 {
                if token.cancelled() {
                    return LookupOutcome::Cancelled;
                }
                match self.search(source, &query.recording, token).await {
                    Ok(songs) => {
                        let instrumental = !query.inferred
                            && songs.iter().any(|s| {
                                s.instrumental
                                    && matches!(
                                        matching::confidence(&query.recording, s),
                                        Some(MatchConfidence::Confident)
                                    )
                            });
                        let versions = matching::versions(&query.recording, songs, query.inferred);
                        if token.cancelled() {
                            return LookupOutcome::Cancelled;
                        }
                        if !versions.is_empty() {
                            return LookupOutcome::Found(versions);
                        }
                        if instrumental {
                            return LookupOutcome::Instrumental;
                        }
                    }
                    Err(error) => {
                        if error.kind == FailureKind::Cancelled {
                            return LookupOutcome::Cancelled;
                        }
                        if let Some(previous) = &mut failure {
                            previous.retry_after_ms =
                                previous.retry_after_ms.max(error.retry_after_ms);
                        } else {
                            failure = Some(error);
                        }
                    }
                }
            }
            if let Some(error) = failure {
                return LookupOutcome::Failed(error);
            }
        }
        LookupOutcome::NotFound
    }
}
async fn cancelled(token: &Cancellation) {
    while !token.cancelled() {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
fn retry_delay(header: Option<&str>, status: u16) -> Duration {
    let seconds = header
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(Duration::from_secs)
        .or_else(|| {
            header
                .and_then(|s| httpdate::parse_http_date(s).ok())
                .map(|date| date.duration_since(SystemTime::now()).unwrap_or_default())
        })
        .unwrap_or_else(|| Duration::from_secs(if status == 429 { 60 } else { 2 }));
    // Bound untrusted arithmetic while retaining long server-requested cooldowns.
    seconds.min(Duration::from_secs(365 * 24 * 60 * 60))
}

#[cfg(test)]
mod tests;
