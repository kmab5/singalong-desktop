use super::{BackupError, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    cell::Cell,
    io::{Read, Write},
};

/// Independent parser-work limits, shared across all JSON entries and embedded
/// preference snapshots in one operation. Byte-budget Unlimited does not disable
/// these limits. Raise them explicitly for unusually large trusted backups.
#[derive(Debug, Clone, Copy)]
pub struct ParseLimits {
    pub max_nodes: u64,
    pub max_string_bytes: u64,
}
impl Default for ParseLimits {
    fn default() -> Self {
        Self {
            max_nodes: 400_000,
            max_string_bytes: 50 * 1024 * 1024,
        }
    }
}
/// Callback must be cheap and nonblocking; true requests cancellation. Cancellation
/// is latched for the operation. An AtomicBool load is a suitable implementation.
pub struct BackupControl<'a> {
    pub is_cancelled: &'a dyn Fn() -> bool,
    pub parse_limits: ParseLimits,
}
impl Default for BackupControl<'_> {
    fn default() -> Self {
        Self {
            is_cancelled: &never_cancel,
            parse_limits: ParseLimits::default(),
        }
    }
}
fn never_cancel() -> bool {
    false
}
pub(super) struct Context<'a> {
    pub control: &'a BackupControl<'a>,
    cancelled: Cell<bool>,
    limited: Cell<bool>,
    nodes: Cell<u64>,
    strings: Cell<u64>,
}
impl<'a> Context<'a> {
    pub fn new(control: &'a BackupControl<'a>) -> Self {
        Self {
            control,
            cancelled: Cell::new(false),
            limited: Cell::new(false),
            nodes: Cell::new(0),
            strings: Cell::new(0),
        }
    }
    pub fn check(&self) -> Result<()> {
        if self.cancelled.get() || (self.control.is_cancelled)() {
            self.cancelled.set(true);
            return Err(BackupError::Cancelled);
        }
        if self.limited.get() {
            return Err(BackupError::ParseLimitExceeded);
        }
        Ok(())
    }
    pub fn charge(&self, nodes: u64, strings: u64) -> Result<()> {
        self.check()?;
        let n = self.nodes.get().checked_add(nodes);
        let s = self.strings.get().checked_add(strings);
        if n.is_none_or(|v| v > self.control.parse_limits.max_nodes)
            || s.is_none_or(|v| v > self.control.parse_limits.max_string_bytes)
        {
            self.limited.set(true);
            return Err(BackupError::ParseLimitExceeded);
        }
        self.nodes.set(n.unwrap_or(u64::MAX));
        self.strings.set(s.unwrap_or(u64::MAX));
        Ok(())
    }
    pub fn io_check(&self) -> std::io::Result<()> {
        self.check().map_err(std::io::Error::other)
    }
    pub fn string_token(&self, bytes: u64) -> Result<()> {
        if self
            .strings
            .get()
            .checked_add(bytes)
            .is_none_or(|v| v > self.control.parse_limits.max_string_bytes)
        {
            self.limited.set(true);
            return Err(BackupError::ParseLimitExceeded);
        }
        Ok(())
    }
    pub fn finish<T>(&self, result: serde_json::Result<T>) -> Result<T> {
        self.check()?;
        Ok(result?)
    }
}
/// Serde's IoRead reads one byte at a time. Check every 4096 bytes, including
/// inside one huge string/whitespace run, rather than just between JSON values.
pub(super) struct CheckedReader<'a, 'c, R> {
    inner: R,
    ctx: &'a Context<'c>,
    remaining: usize,
}
impl<'a, 'c, R> CheckedReader<'a, 'c, R> {
    pub fn new(inner: R, ctx: &'a Context<'c>) -> Self {
        Self {
            inner,
            ctx,
            remaining: 0,
        }
    }
}
impl<R: Read> Read for CheckedReader<'_, '_, R> {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        if self.remaining == 0 {
            self.ctx.io_check()?;
            self.remaining = 4096;
        }
        let limit = bytes.len().min(self.remaining);
        let n = self.inner.read(&mut bytes[..limit])?;
        self.remaining -= n;
        Ok(n)
    }
}
pub(super) fn digest(bytes: &[u8], ctx: &Context<'_>) -> Result<String> {
    let mut hash = Sha256::new();
    for chunk in bytes.chunks(8192) {
        ctx.check()?;
        hash.update(chunk);
    }
    ctx.check()?;
    Ok(format!("{:x}", hash.finalize()))
}
pub(super) fn checksum<T: Serialize>(value: &T, ctx: &Context<'_>) -> Result<String> {
    struct HashWriter<'a, 'c> {
        hash: Sha256,
        ctx: &'a Context<'c>,
    }
    impl Write for HashWriter<'_, '_> {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            for chunk in b.chunks(8192) {
                self.ctx.io_check()?;
                self.hash.update(chunk);
            }
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.ctx.io_check()
        }
    }
    let mut writer = HashWriter {
        hash: Sha256::new(),
        ctx,
    };
    ctx.finish(serde_json::to_writer(&mut writer, value))?;
    Ok(format!("{:x}", writer.hash.finalize()))
}
