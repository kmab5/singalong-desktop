use crate::model::LyricLine;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Moment {
    pub index: Option<usize>,
    pub countdown_seconds: Option<u64>,
}

/// Positive delay makes lyrics later. Only explicit blank markers create gaps.
pub fn at(lines: &[LyricLine], position: Option<u64>, delay: i64) -> Moment {
    let Some(position) = position else {
        return Moment::default();
    };
    let time = i128::from(position) - i128::from(delay);
    let index = lines.iter().rposition(|l| {
        l.start_ms
            .as_ref()
            .is_some_and(|s| i128::from(s.value()) <= time)
    });
    let next = if index.is_none_or(|i| lines[i].text.trim().is_empty()) {
        lines
            .iter()
            .skip(index.map_or(0, |i| i + 1))
            .find(|l| l.start_ms.is_some() && !l.text.trim().is_empty())
    } else {
        None
    };
    let countdown_seconds = next
        .map(|l| ((i128::from(l.start_ms.as_ref().unwrap().value()) - time).max(0) + 999) / 1000)
        .map(|v| v.min(i128::from(u64::MAX)) as u64);
    Moment {
        index,
        countdown_seconds,
    }
}
