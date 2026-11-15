//! Lyricsfile 1.0 draft. Unsupported documents fall back to legacy LRC/plain.
use super::{MAX_BYTES, MAX_LINES};
use crate::model::{Counter, LyricLine, LyricWord};
use serde_json::{Map, Value};
use yaml_rust2::{
    Yaml,
    parser::{Event, Parser},
    scanner::TScalarStyle,
};

fn next(parser: &mut Parser<std::str::Chars<'_>>) -> Option<Event> {
    parser.next_token().ok().map(|(e, _)| e)
}

// Read events directly: reject aliases/anchors/tags before allocating a tree,
// limit depth and node count, and reject duplicate keys instead of overwriting.
fn node(
    parser: &mut Parser<std::str::Chars<'_>>,
    event: Event,
    depth: usize,
    budget: &mut usize,
) -> Option<Value> {
    if depth > 20 || *budget == 0 {
        return None;
    }
    *budget -= 1;
    match event {
        Event::Scalar(s, style, 0, None) => {
            if s.len() > 100_000 {
                return None;
            }
            if style != TScalarStyle::Plain {
                return Some(Value::String(s));
            }
            Some(match Yaml::from_str(&s) {
                Yaml::Integer(i) => Value::from(i),
                Yaml::Boolean(b) => Value::from(b),
                Yaml::Null => Value::Null,
                Yaml::String(s) => Value::String(s),
                // Reals and oversized integers must never become timestamps.
                _ => Value::Bool(false),
            })
        }
        Event::SequenceStart(0, None) => {
            let mut values = vec![];
            loop {
                let event = next(parser)?;
                if matches!(event, Event::SequenceEnd) {
                    break;
                }
                values.push(node(parser, event, depth + 1, budget)?);
            }
            Some(Value::Array(values))
        }
        Event::MappingStart(0, None) => {
            let mut values = Map::new();
            loop {
                let event = next(parser)?;
                if matches!(event, Event::MappingEnd) {
                    break;
                }
                let key = node(parser, event, depth + 1, budget)?.as_str()?.to_owned();
                if values.contains_key(&key) {
                    return None;
                }
                let event = next(parser)?;
                values.insert(key, node(parser, event, depth + 1, budget)?);
            }
            Some(Value::Object(values))
        }
        _ => None,
    }
}
fn millis(value: &Value) -> Option<u64> {
    value.as_i64().and_then(|n| u64::try_from(n).ok())
}
fn end(value: Option<&Value>, start: u64) -> Option<Option<Counter>> {
    match value.filter(|v| !v.is_null()) {
        None => Some(None),
        Some(v) => {
            let end = millis(v)?;
            (end >= start).then(|| Some(Counter::new(end)))
        }
    }
}
fn words(value: Option<&Value>, text: &str) -> Option<Vec<LyricWord>> {
    let rows = value?.as_array()?;
    if rows.len() > 10_000 {
        return None;
    }
    let mut words = vec![];
    let mut offset = 0;
    for raw in rows {
        let word = raw.as_object()?;
        let part = word.get("text")?.as_str()?;
        if part.is_empty() || !text.get(offset..)?.starts_with(part) {
            return None;
        }
        offset += part.len();
        let start = millis(word.get("start_ms")?)?;
        words.push(LyricWord {
            text: part.into(),
            start_ms: Counter::new(start),
            end_ms: end(word.get("end_ms"), start)?,
        });
    }
    (offset == text.len()).then_some(words)
}

pub fn parse(document: &str) -> Option<Vec<LyricLine>> {
    if document.trim().is_empty() || document.len() > MAX_BYTES {
        return None;
    }
    let mut parser = Parser::new_from_str(document);
    if !matches!(next(&mut parser)?, Event::StreamStart)
        || !matches!(next(&mut parser)?, Event::DocumentStart)
    {
        return None;
    }
    let event = next(&mut parser)?;
    let root = node(&mut parser, event, 0, &mut 250_000)?;
    if !matches!(next(&mut parser)?, Event::DocumentEnd)
        || !matches!(next(&mut parser)?, Event::StreamEnd)
    {
        return None;
    }
    let root = root.as_object()?;
    if root.get("version")?.as_str()? != "1.0" {
        return None;
    }
    let metadata = root.get("metadata")?.as_object()?;
    metadata.get("title")?.as_str()?;
    metadata.get("artist")?.as_str()?;
    if let Some(offset) = metadata.get("offset_ms").filter(|v| !v.is_null())
        && millis(offset)? != 0
    {
        return None;
    }
    let rows = root.get("lines")?.as_array()?;
    if rows.is_empty() || rows.len() > MAX_LINES {
        return None;
    }
    let mut lines = vec![];
    for (index, raw) in rows.iter().enumerate() {
        let row = raw.as_object()?;
        let text = row.get("text")?.as_str()?;
        let start = millis(row.get("start_ms")?)?;
        lines.push(LyricLine {
            id: index.to_string(),
            text: text.into(),
            start_ms: Some(Counter::new(start)),
            end_ms: end(row.get("end_ms"), start)?,
            words: words(row.get("words"), text).unwrap_or_default(),
        });
    }
    lines.sort_by_key(|l| l.start_ms.as_ref().unwrap().value());
    Some(lines)
}
