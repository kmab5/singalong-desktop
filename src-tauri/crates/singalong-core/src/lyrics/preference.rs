use crate::model::{LyricVersion, MatchConfidence, TimingKind};

/// Only rank eligible candidates; stored explicit selections are resolved first.
pub fn sort(versions: &mut [LyricVersion], duration: Option<u64>) {
    versions.sort_by_key(|v| {
        (
            !matches!(v.timing, TimingKind::Supplied),
            duration
                .filter(|d| *d > 0)
                .zip(v.provider_duration_ms.as_ref())
                .map_or(u64::MAX, |(d, p)| d.abs_diff(p.value())),
            !matches!(v.confidence, MatchConfidence::Confident),
            std::cmp::Reverse(v.lines.iter().filter(|l| !l.words.is_empty()).count()),
            v.id.clone(),
        )
    });
}
