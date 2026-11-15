//! Bounded provider-data parsing and conservative matching. See SOURCE-NOTICES.md.
pub mod lrc;
pub mod lyricsfile;
pub mod matching;
pub mod preference;
pub mod queries;
pub mod timeline;

pub const MAX_BYTES: usize = 2_000_000;
pub const MAX_LINES: usize = 20_000;
