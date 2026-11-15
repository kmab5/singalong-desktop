# P6 backup codec source checkpoint

Implemented Android schema-1 ZIP and legacy JSON import plus the distinct full desktop v1 JSON envelope. Source preserves IDs, original hashes, provenance, lyric lines/words, selections, adjustments, translations, references and typed/inert preferences. ZIP reads validate central/local agreement, CRC, entry hashes and signed/unsigned descriptors without extracting files.

Byte limits default to 50 MiB; parser-work limits independently default to 400,000 nodes and 50 MiB cumulative decoded strings. Unlimited archive bytes do not disable record or parser limits. Export preflight charges the complete envelope and embedded preferences against the same limits as restore. Cooperative cancellation checks parsing, decompression, validation, hashes and writing. Native integration must retain generation checks and atomic output cleanup.

Source review closed allocation amplification, cancellation coverage, ZIP signature ambiguity and export/restore budget asymmetry findings. Core IPC exporter generation completed after the changes; this is not a test suite result or a desktop build. All behavior tests and native checks remain paused at the user's request.

Next dependency: transactional storage merge/snapshot, immutable native preview, file dialogs and backup UI. P6 is not complete or verified at this checkpoint. No installer or release was built/published.
