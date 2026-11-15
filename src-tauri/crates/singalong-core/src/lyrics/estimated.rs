use crate::model::LyricLine;
/// In-memory reading aid only; original timestamps remain untouched.
pub fn starts(lines: &[LyricLine], duration: u64) -> Vec<Option<u64>> {
    if duration == 0 {
        return vec![None; lines.len()];
    }
    let weights: Vec<u128> = lines
        .iter()
        .map(|line| {
            line.text
                .chars()
                .filter(|c| !c.is_whitespace())
                .map(|c| c.len_utf16() as u128)
                .sum()
        })
        .collect();
    let total = weights.iter().sum::<u128>().max(1);
    let mut consumed = 0;
    weights
        .into_iter()
        .map(|weight| {
            if weight == 0 {
                None
            } else {
                let start = (consumed * u128::from(duration) / total) as u64;
                consumed += weight;
                Some(start)
            }
        })
        .collect()
}
pub fn index(
    lines: &[LyricLine],
    position: Option<u64>,
    duration: u64,
    delay: i64,
) -> Option<usize> {
    let time = i128::from(position?) - i128::from(delay);
    starts(lines, duration)
        .iter()
        .rposition(|start| start.is_some_and(|start| i128::from(start) <= time))
}
