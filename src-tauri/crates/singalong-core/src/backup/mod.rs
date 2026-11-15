//! Pure backup codecs: no filesystem, database, network, or settings side effects.
//! Desktop v1 is UTF-8 JSON with format="singalong-desktop", schema=1,
//! data (BackupData), and sha256 over compact serde JSON serialization of data.
//! BTreeMaps sort keys; struct declaration order defines fields. Existing lyric
//! contentHash values are opaque identities and are never recomputed.
mod android_v1;
mod control;
mod json;
pub use control::{BackupControl, ParseLimits};
use control::{Context, checksum, digest};
use json::decode as json;
mod zip;
pub mod model;
mod validation;
pub use model::*;
pub use validation::{validate, validate_preferences, validate_with_control, validate_preferences_with_control};

use std::{fmt, io::{Read, Seek, SeekFrom, Write}};
use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, BackupError>;
#[derive(Debug)]
pub enum BackupError { Io(std::io::Error), Json(serde_json::Error), Invalid(&'static str), BudgetExceeded, ParseLimitExceeded, Cancelled }
impl fmt::Display for BackupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(_) => f.write_str("Backup input/output failed"),
            Self::Json(_) => f.write_str("Invalid backup JSON"),
            Self::Invalid(message) => f.write_str(message),
            Self::Cancelled => f.write_str("Backup operation cancelled"),
            Self::ParseLimitExceeded => f.write_str("Backup exceeds selected parser-work limits"),
            Self::BudgetExceeded => f.write_str("Backup exceeds selected byte budget"),
        }
    }
}
impl std::error::Error for BackupError {}
impl From<std::io::Error> for BackupError { fn from(e: std::io::Error) -> Self { Self::Io(e) } }
impl From<serde_json::Error> for BackupError { fn from(e: serde_json::Error) -> Self { Self::Json(e) } }
pub(super) fn ensure(ok: bool, message: &'static str) -> Result<()> { if ok { Ok(()) } else { Err(BackupError::Invalid(message)) } }

/// None is unlimited archive bytes, not unlimited record sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupBudget { pub max_bytes: Option<u64> }
impl Default for BackupBudget { fn default() -> Self { Self { max_bytes: Some(50 * 1024 * 1024) } } }
impl BackupBudget {
    pub const fn unlimited() -> Self { Self { max_bytes: None } }
    fn check(self, n: u64) -> Result<()> {
        ensure(self.max_bytes != Some(0), "byte budget must be positive or unlimited")?;
        if self.max_bytes.is_some_and(|max| n > max) { Err(BackupError::BudgetExceeded) } else { Ok(()) }
    }
    fn compressed(self) -> Self { Self { max_bytes: self.max_bytes.map(|m| m.saturating_mul(2).saturating_add(4 * 1024 * 1024)) } }
}
fn read_bounded<R: Read>(input: &mut R, output: &mut Vec<u8>, budget: BackupBudget, count: &mut u64, ctx: &Context<'_>) -> Result<()> {
    let mut buffer = [0u8; 8192];
    loop {
        ctx.check()?;
        let n = input.read(&mut buffer)?;
        if n == 0 { return Ok(()); }
        *count = count.checked_add(n as u64).ok_or(BackupError::Invalid("backup byte count overflow"))?;
        budget.check(*count)?;
        output.try_reserve(n).map_err(|_| BackupError::Invalid("backup allocation failed"))?;
        output.extend_from_slice(&buffer[..n]);
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Desktop { format: String, schema: u32, data: BackupData, sha256: String }

/// Reads from the current stream position to EOF; leaves stream ownership with caller.
/// Input is buffered under the selected cap. Unlimited remains memory-dependent.
pub fn decode_reader<R: Read + Seek>(input: R, budget: BackupBudget) -> Result<DecodedBackup> {
    decode_reader_with_control(input, budget, &BackupControl::default())
}
pub fn decode_reader_with_control<R: Read + Seek>(mut input: R, budget: BackupBudget, control: &BackupControl<'_>) -> Result<DecodedBackup> {
    let ctx = Context::new(control);
    ctx.check()?;
    budget.check(0)?;
    let position = input.stream_position()?;
    let mut prefix = [0u8; 2];
    let mut got = 0;
    while got < 2 { ctx.check()?; let n = input.read(&mut prefix[got..])?; if n == 0 { break; } got += n; }
    input.seek(SeekFrom::Start(position))?;
    let is_zip = prefix == *b"PK";
    let mut bytes = Vec::new(); let mut count = 0;
    read_bounded(&mut input, &mut bytes, if is_zip { budget.compressed() } else { budget }, &mut count, &ctx)?;
    decode_inner(&bytes, budget, &ctx)
}
pub fn decode_bytes(bytes: &[u8], budget: BackupBudget) -> Result<DecodedBackup> {
    decode_bytes_with_control(bytes, budget, &BackupControl::default())
}
pub fn decode_bytes_with_control(bytes: &[u8], budget: BackupBudget, control: &BackupControl<'_>) -> Result<DecodedBackup> {
    decode_inner(bytes, budget, &Context::new(control))
}
fn decode_inner(bytes: &[u8], budget: BackupBudget, ctx: &Context<'_>) -> Result<DecodedBackup> {
    ctx.check()?;
    budget.check(0)?;
    let (source, data) = if bytes.starts_with(b"PK") {
        budget.compressed().check(bytes.len() as u64)?;
        (BackupSource::AndroidZipV1, android_v1::decode(zip::decode(bytes, budget, ctx)?, ctx)?)
    } else {
        budget.check(bytes.len() as u64)?;
        let bytes = bytes.strip_prefix(&[0xef,0xbb,0xbf]).unwrap_or(bytes);
        json::preflight(bytes, ctx)?;
        #[derive(Deserialize)]
        struct Header { #[serde(default)] format: Option<String> }
        let header: Header = json::typed(bytes, ctx)?;
        if header.format.is_some() {
            let desktop: Desktop = json::typed(bytes, ctx)?;
            ensure(desktop.format == "singalong-desktop" && desktop.schema == 1, "unsupported desktop backup version")?;
            ensure(desktop.data.options == BackupOptions::default(), "desktop backup must contain all sections")?;
            ensure(desktop.sha256 == checksum(&desktop.data, ctx)?, "desktop backup hash mismatch")?;
            (BackupSource::DesktopV1, desktop.data)
        } else {
            let document: android_v1::Document = json::typed(bytes, ctx)?;
            (BackupSource::AndroidJsonV1, document.into_data(ctx)?)
        }
    };
    validation::validate_inner(&data, ctx)?;
    let summary = validation::summary(&data, ctx)?;
    Ok(DecodedBackup { source, data, summary })
}

/// Full desktop export only. Caller supplies a complete storage snapshot; a partial
/// Android import cannot be silently widened into a full backup.
pub fn encode_bytes(data: &BackupData, budget: BackupBudget) -> Result<Vec<u8>> {
    encode_bytes_with_control(data, budget, &BackupControl::default())
}
pub fn encode_bytes_with_control(data: &BackupData, budget: BackupBudget, control: &BackupControl<'_>) -> Result<Vec<u8>> {
    let mut output = Vec::new(); encode_writer_with_control(&mut output, data, budget, control)?; Ok(output)
}
/// Writes without closing or flushing caller output. On error output may be partial;
/// native wrapper must use a private temporary file and atomic replacement.
pub fn encode_writer<W: Write>(output: W, data: &BackupData, budget: BackupBudget) -> Result<()> {
    encode_writer_with_control(output, data, budget, &BackupControl::default())
}
pub fn encode_writer_with_control<W: Write>(mut output: W, data: &BackupData, budget: BackupBudget, control: &BackupControl<'_>) -> Result<()> {
    let ctx = Context::new(control);
    ctx.check()?;
    budget.check(0)?;
    validation::validate_inner(data, &ctx)?;
    ensure(data.options == BackupOptions::default(), "desktop export requires full snapshot")?;
    #[derive(Serialize)]
    struct Envelope<'a> { format: &'static str, schema: u32, data: &'a BackupData, sha256: String }
    let envelope = Envelope { format: "singalong-desktop", schema: 1, data, sha256: checksum(data, &ctx)? };
    // validate_inner already charged the embedded savedPreferences parse, just
    // as desktop decode does. Add the complete envelope to that SAME context.
    // No destination bytes are written until both cumulative limits pass.
    json::preflight_serialized(&envelope, budget, &ctx)?;
    struct BudgetWriter<'a,'c,W> { ctx: &'a Context<'c>, inner: &'a mut W, budget: BackupBudget, count: u64, exceeded: bool }
    impl<W: Write> Write for BudgetWriter<'_,'_,W> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.ctx.io_check()?;
            let bytes = &bytes[..bytes.len().min(8192)];
            let next = self.count.checked_add(bytes.len() as u64);
            if next.is_none_or(|n| self.budget.check(n).is_err()) {
                self.exceeded = true; return Err(std::io::Error::other("backup byte budget exceeded"));
            }
            let written = self.inner.write(bytes)?;
            self.count += written as u64; Ok(written)
        }
        fn flush(&mut self) -> std::io::Result<()> { self.inner.flush() }
    }
    let mut writer = BudgetWriter { ctx: &ctx, inner: &mut output, budget, count: 0, exceeded: false };
    let result = serde_json::to_writer(&mut writer, &envelope);
    ctx.check()?;
    if writer.exceeded { return Err(BackupError::BudgetExceeded); }
    result?; Ok(())
}
