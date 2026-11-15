use crate::model::LyricLine;
/// Missing ends are inferred for presentation only; overlapping supplied ends survive.
pub fn emphasis(
    lines: &[LyricLine],
    index: usize,
    position: Option<u64>,
    duration: Option<u64>,
    delay: i64,
) -> Vec<f32> {
    let Some(line) = lines.get(index) else {
        return vec![];
    };
    let Some(position) = position else {
        return vec![0.0; line.words.len()];
    };
    let time = i128::from(position) - i128::from(delay);
    let line_end = line
        .end_ms
        .as_ref()
        .map(|c| c.value())
        .or_else(|| {
            lines
                .iter()
                .skip(index + 1)
                .filter_map(|l| l.start_ms.as_ref().map(|c| c.value()))
                .find(|start| *start > line.start_ms.as_ref().map_or(0, |c| c.value()))
        })
        .or(duration);
    let starts: std::collections::BTreeSet<u64> = line
        .words
        .iter()
        .map(|word| word.start_ms.value())
        .collect();
    line.words
        .iter()
        .map(|word| {
            let start = word.start_ms.value();
            if time < i128::from(start) {
                return 0.0;
            }
            let end = word
                .end_ms
                .as_ref()
                .map(|c| c.value())
                .or_else(|| {
                    starts
                        .range((std::ops::Bound::Excluded(start), std::ops::Bound::Unbounded))
                        .next()
                        .copied()
                })
                .or(line_end.filter(|end| *end >= start));
            let progress = match end {
                Some(end) if end > start => {
                    (time - i128::from(start)) as f64 / (end - start) as f64
                }
                _ => 1.0,
            };
            (progress.clamp(0.0, 1.0) * 100.0).round() as f32 / 100.0
        })
        .collect()
}
