# Source notices

The lyric parsing, search-query, matching, preference and timeline rules in
`src-tauri/crates/singalong-core/src/lyrics/` and their synthetic regression tests
are adapted from [Singalong Android](https://github.com/kmab5/singalong), commit
`453068332bb58e3d642d294ef861ba6ccd337952` (1.3.0): `LyricTiming.kt`,
`LyricsfileParser.kt`, `WordSync.kt`, `SearchQueries.kt`, `LyricsMatcher.kt`,
`LyricPreference.kt`, and their tests under `com/kmab/singalong/lyrics`.

The Android source is distributed under GNU GPL version 3; its license is
preserved verbatim in [licenses/Singalong-Android-GPL-3.0.txt](licenses/Singalong-Android-GPL-3.0.txt).
These adaptations retain that license. Desktop changes include Rust types,
bounded parsing, strict event-based YAML validation, overflow-safe arithmetic,
and protection against filename hints removing tagged edition qualifiers.

Test lyric text is synthetic. Provider responses are not bundled source assets.
