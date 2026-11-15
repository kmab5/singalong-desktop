use super::{MAX_BYTES, MAX_LINES};
use crate::model::{Counter, LyricLine};
use regex::Regex;
use std::sync::LazyLock;

static TAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[([0-9]{1,4}):([0-9]{2})(?:[.:]([0-9]{1,3}))?]").unwrap());
static OFFSET: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\[offset:([+-]?[0-9]+)]").unwrap());

fn row(text: String, start: Option<u64>, index: usize) -> LyricLine {
    LyricLine {
        id: index.to_string(),
        text,
        start_ms: start.map(Counter::new),
        end_ms: None,
        words: vec![],
    }
}

pub fn parse(synced: Option<&str>, plain: Option<&str>) -> Vec<LyricLine> {
    let synced = synced.filter(|s| s.len() <= MAX_BYTES).unwrap_or_default();
    let shift = OFFSET
        .captures(synced)
        .and_then(|c| c[1].parse::<i64>().ok())
        .unwrap_or(0);
    let mut timed = vec![];
    let mut expanded_bytes = 0;
    for text in synced.lines() {
        let Some(last) = TAG.find_iter(text).last() else {
            continue;
        };
        let content = text[last.end()..].trim();
        if content.len() > 100_000 {
            return vec![];
        }
        for tag in TAG.captures_iter(text) {
            let seconds = tag[2].parse::<i64>().unwrap();
            if seconds >= 60 {
                continue;
            }
            expanded_bytes += content.len();
            if timed.len() == MAX_LINES || expanded_bytes > MAX_BYTES {
                return vec![];
            }
            let fraction = tag.get(3).map_or(0, |v| {
                v.as_str().parse::<i64>().unwrap() * 10_i64.pow(3 - v.len() as u32)
            });
            let start = (tag[1].parse::<i64>().unwrap() * 60_000 + seconds * 1000 + fraction)
                .saturating_add(shift)
                .max(0) as u64;
            timed.push(row(content.to_string(), Some(start), timed.len()));
        }
    }
    if timed.iter().any(|r| !r.text.trim().is_empty()) {
        timed.sort_by_key(|r| r.start_ms.as_ref().unwrap().value());
        for (i, r) in timed.iter_mut().enumerate() {
            r.id = i.to_string();
        }
        return timed;
    }
    let plain = plain.filter(|s| s.len() <= MAX_BYTES).unwrap_or_default();
    if plain.trim().is_empty() {
        return vec![];
    }
    let lines: Vec<_> = plain.split('\n').take(MAX_LINES + 1).collect();
    if lines.len() > MAX_LINES || lines.iter().any(|s| s.len() > 100_000) {
        return vec![];
    }
    lines
        .into_iter()
        .enumerate()
        .map(|(i, s)| row(s.strip_suffix('\r').unwrap_or(s).into(), None, i))
        .collect()
}
