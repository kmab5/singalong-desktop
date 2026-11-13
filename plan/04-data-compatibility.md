# Data, providers and Android compatibility

Stable scope follows [the confirmed contract](09-confirmed-release-scope.md): Android import, full desktop export/merge restore and inert data retention. Selective export UI, reverse Android export, translation services, editing, folder scanning and cards are later work. Their data model descriptions below preserve future compatibility without requiring those features now.

## Persistence contract

Create an independent desktop database with ordered, transactional migrations. Preserve the Android concepts, not Room's generated schema metadata. Tables should cover recordings, occurrences, lyric versions, provenance-only references, explicit selections, adjustments, complete translations, typed settings and inert migrated preferences. Translation segment cache/quota can be added with the later translation phase. Use foreign keys and transactions. Index recording comparison keys, lyric recording IDs and history ordering.

Store original strings separately from normalized comparison values. IDs are stable opaque strings. Do not use title/artist alone as a primary key. Keep user selection distinct from candidate recommendation, and keep lyric references distinct from full text.

Persist timestamps in milliseconds with checked integer conversion. Kotlin Long values can exceed JavaScript's safe integer range; serialize unbounded counts/revisions as decimal strings or reject unsupported values explicitly at the bridge. Backup parsing and writing stay in Rust with 64-bit integers.

Use SQLite from a Rust repository/worker, not arbitrary frontend SQL. A small synchronous SQLite library on a dedicated worker is sufficient; select and pin its version at P1. Add no cloud database. Desktop settings can share the SQLite transaction used for restore, avoiding Android's cross-store partial-commit issue.

## Android backup interchange

Support importing Android 1.3.0 schema-1 ZIP archives and legacy schema-1 JSON. Internal desktop schema numbers must not be confused with archive schema numbers or Android Room schema 4.

ZIP schema 1:

| Entry | Meaning |
| --- | --- |
| index.json | format = singalong; schema = 1; options; entries with path, bytes, sha256 |
| details.json | Minimal recording identities; mediaFilesIncluded=false; optional creation/app version |
| songs.json | Recording metadata and favorite values, only if selected |
| history.json | Recording ID, plays, lastPlayed, only if selected |
| settings.json | String map containing typed preference encodings and legacy preference fields |
| lyrics.json | Recording ID, version-file paths, references, selected version, adjustments, translations |
| lyrics/0.json etc. | Complete LyricVersion objects; omitted for links-only mode |

The index is written after streamed payloads and does not hash itself. Validate exact included entries and referenced IDs before mutation. Reject unknown schema, duplicate names, unsafe paths, inconsistent local/central entries, CRC/hash/size mismatch, invalid encoding and unexpected files. Never extract arbitrary archive paths to disk.

Maintain Android record limits, including 500 versions/references per song, 20,000 lines, 2 MiB lyric text, valid ordered line timing and valid word concatenation. Archive default is 50 MiB uncompressed with adjustable or unlimited byte cap; unlimited does not waive record validation. Count bytes while streaming, not just declared sizes. Support ZIP64, handle disk-full/cancel, and clean temporary files on every exit. No hidden per-entry archive cap may contradict the UI.

For strict round-trip compatibility, copy exact Android field names/defaults/enum encodings and typed preference formats into documented codec fixtures; do not guess from the public guide. Android's JSON parser rejects unknown keys, so adding desktop fields to schema 1 would break Android restore.

The first stable release exports a full, versioned desktop backup. Android-compatible export is deferred. Keep these formats distinct:

1. **Android-compatible backup (later):** exact schema-1 content; portable preferences only, using known Android names/types. Do not invent Android packages from desktop player IDs.
2. **Full desktop backup (stable):** a clearly versioned desktop format carrying native settings. Define it during P6 and reject unsupported versions explicitly; never disguise it as Android schema 1.

Stable requires Android → desktop import and desktop → desktop full backup/restore. Advertise desktop → Android only after the Android decoder accepts generated fixtures and a device restore is verified. Test hashes against raw encoded bytes; field ordering differences may change content hashes even for equivalent objects. Preserve imported hashes/IDs; derive new hashes using a documented desktop canonicalization and do not rewrite existing IDs just to match it.

## Restore algorithm

1. Copy selected input to private temporary storage under the configured byte budget.
2. Decode, validate all records and selected settings, and compute a preview with counts, collisions and settings portability warnings.
3. Tie preview to an opaque token and immutable file digest. Applying it must not reopen a different path's contents.
4. Serialize mutation through the database owner. Resolve stable IDs first; incompatible collisions receive deterministic remapped IDs.
5. Use fuzzy matching only with sufficient full metadata and one unclaimed existing candidate. Partial identities do not justify fuzzy merging; preserve distinct source IDs.
6. Map versions, references, selections, adjustments and translations through the recording/version ID maps.
7. Existing selections, edits and adjustments win. History uses non-inflating merge semantics (maximum play count/latest timestamp), not sum on every import. Omitted sections leave existing data alone.
8. Apply allowed settings only if selected. Preserve active tracking, current translation consent, startup registration and OS permission decisions. Android package IDs, notification prompt flags and dp geometry do not enable desktop behavior.
9. Commit atomically in desktop SQLite and emit one revision. Remove temporary files and show actual counts, including skipped/conflicting records.

Retain validated saved translations and round-trip them through full desktop backups without enabling translation UI/services. Store validated unsupported Android preferences separately as inert migration data; never apply them as active player IDs, consent, permissions, startup or network settings. The import summary distinguishes restored content, preserved-but-inactive data and unapplied settings. Preserve current desktop network policy and disclose unsupported Wi-Fi-only preferences.

Partial translation cache is not a complete translation backup. Source references must not make `hasLyrics` true. Restoring the same archive twice must leave the second result unchanged.

## Lookup and network policy

Use any connection by default and expose Offline mode, which cancels/stops provider requests while permitting cache reads. Fetch immediately for an open reader; background collection saves after ten seconds of actual playing, even with hidden windows. Display uncertain/inferred results as Possible match. Support alternate-result selection and query correction through Wrong lyrics, plus manual title/artist search without playback. Do not overwrite source metadata with search hints.

Port the source behavior before adding providers. Recommended desktop improvements are shared per-provider pacing across live lookup, background collection and folder scanning, and explicit cancellation generations for destructive cleanup.

LRCLIB uses search with title/artist, identifying User-Agent, a bounded response and cancellation. Keep the Android 350 ms pacing and 10-second timeout as initial app policy, subject to current service requirements. Respect Retry-After on rate limits and unavailability. Preserve fallback error-versus-miss semantics and user selections.

Retain maximum three conservative query variants, original display metadata, edition qualifiers and uncertain labels for inferred matches. A confident match needs album equality and duration within two seconds; reject duration disagreement over the larger of ten seconds or ten percent. Source-specific plain fallback does not gain independent recording evidence.

Parse LRC and the supported Lyricsfile subset with bounded work and no object construction/aliases from YAML. Unsupported future formats fall back safely. Never label visual word estimates as supplied word sync.

Consult [LRCLIB docs](https://lrclib.net/docs) and [lyrics.ovh's API repository](https://github.com/NTag/lyrics.ovh#api) at implementation time. The draft Lyricsfile behavior is evidenced in Android source; do not assume every live LRCLIB response contains it.

## Translation replacement

ML Kit language identification is Android-specific in this project. Define a local `LanguageDetector` port and evaluate a maintained Rust-compatible detector against short, multilingual and mixed-language synthetic lyric samples. Selection is a P7 gate: record license, model size, startup cost and error rates. Unknown/low-confidence detection must return unsupported, not guess a source language.

Preserve consent, English target, original line IDs, content-hash binding, repeated-line cache, maximum two concurrent requests, 500 UTF-8-byte segments and local 5,000-code-point budget. Oversized source lines remain unsupported for parity; future splitting needs a separate mapping/test design. Cache before spending quota, reserve budget atomically and keep partial UI progress distinct from complete-version storage.

MyMemory documents the 500-byte query limit and language-pair parameter. Recheck current service limits and terms before release. Sources: [API](https://mymemory.translated.net/doc/spec.php), [limits](https://mymemory.translated.net/doc/usagelimits.php), [terms](https://mymemory.translated.net/terms-and-conditions). The 5,000 budget here is the observed Android app policy, not a newly verified service guarantee.

External Translate is a user-initiated clipboard/browser action, separate from automatic translation consent. Explain clipboard replacement, preserve full text, bound URLs, report copy/open failures separately and do not ingest browser results.

## Editor, files and cards

Use native file/folder dialogs and opaque access tokens. Validate UTF-8/BOM and size before previewing imports. Save a new immutable lyric version; text changes remove affected word timing, and timing changes remove invalidated ends/words. Draft state is independent of the selected playback song.

Folder scanning walks only the chosen root, handles permission loss and symlink cycles/escapes, reads local metadata with a pinned supported-format library, and reports unsupported formats per file. Never upload audio. A one-second inter-file lookup pause plus shared provider pacing preserves conservative behavior. Commit only nonempty matches with a transactional duplicate recheck.

Cards use a local Canvas/SVG-based renderer with bundled fonts and a frozen content/artwork snapshot. Share preview/export composition rules and render at explicit output dimensions, not devicePixelRatio-dependent sizes. Wait for fonts, shape RTL/Unicode correctly, detect overflow and bound image decode memory. Keep artwork palette distinct from optional background. Save PNG through native dialogs; offer clipboard image only if supported and tested. Native share sheets are optional, not assumed cross-platform.

## Deletion and recovery

Clearing downloads invalidates relevant jobs before deleting provider versions; retain imports, personal edits and favorite protection. Check generation and version existence before later saves. Clearing history resets counts/timestamps without deleting content; define how active qualification is reset so it cannot unexpectedly recount the same occurrence.

Surface storage errors, do not silently recreate a corrupt database. Provide a safe backup/recovery path. A single-instance lock avoids two independent collection cores writing the same library. Diagnostics remain bounded and local; exclude user content and raw provider request URLs.
