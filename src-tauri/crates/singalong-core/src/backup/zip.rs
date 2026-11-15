//! Strict single-disk ZIP/ZIP64 reader. Only stored and raw DEFLATE entries.
//! Central records must cover consecutive local records, with no aliases or gaps.
use super::{BackupBudget, BackupError, Result, control::Context, ensure};
use std::collections::BTreeMap;

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8], at: usize) -> Self {
        Self { bytes, at }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .at
            .checked_add(n)
            .ok_or(BackupError::Invalid("ZIP offset overflow"))?;
        let bytes = self
            .bytes
            .get(self.at..end)
            .ok_or(BackupError::Invalid("truncated ZIP"))?;
        self.at = end;
        Ok(bytes)
    }
    fn u16(&mut self) -> Result<u16> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }
    fn u32(&mut self) -> Result<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn u64(&mut self) -> Result<u64> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }
    fn sig(&mut self, expected: u32) -> Result<()> {
        ensure(self.u32()? == expected, "invalid ZIP signature")
    }
}
fn size(v: u64) -> Result<usize> {
    usize::try_from(v).map_err(|_| BackupError::Invalid("ZIP offset exceeds address space"))
}
fn extra(bytes: &[u8]) -> Result<Option<&[u8]>> {
    let mut c = Cursor::new(bytes, 0);
    let mut zip64 = None;
    while c.at < bytes.len() {
        let kind = c.u16()?;
        let n = c.u16()? as usize;
        let body = c.take(n)?;
        if kind == 1 {
            ensure(zip64.is_none(), "duplicate ZIP64 extra")?;
            zip64 = Some(body);
        }
    }
    Ok(zip64)
}
fn name(bytes: &[u8]) -> Result<String> {
    let s = std::str::from_utf8(bytes)
        .map_err(|_| BackupError::Invalid("invalid ZIP path encoding"))?;
    ensure(
        !s.is_empty()
            && s.len() <= 200
            && s.bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"/_.-".contains(&c))
            && s.split('/').all(|p| !p.is_empty() && p != "." && p != ".."),
        "unsafe ZIP path",
    )?;
    Ok(s.into())
}
struct Entry {
    name: String,
    needed: u16,
    timestamp: u32,
    flags: u16,
    method: u16,
    crc: u32,
    compressed: u64,
    expanded: u64,
    offset: u64,
    zip64: bool,
}

pub(super) fn decode(
    bytes: &[u8],
    budget: BackupBudget,
    ctx: &Context<'_>,
) -> Result<BTreeMap<String, Vec<u8>>> {
    ensure(bytes.len() >= 22, "truncated ZIP end")?;
    let start = bytes.len().saturating_sub(22 + 65535);
    let mut found = None;
    for at in (start..=bytes.len() - 22).rev() {
        ctx.check()?;
        if bytes.get(at..at + 4) == Some(&[0x50, 0x4b, 0x05, 0x06][..])
            && at + 22 + u16::from_le_bytes([bytes[at + 20], bytes[at + 21]]) as usize
                == bytes.len()
        {
            found = Some(at);
            break;
        }
    }
    let end = found.ok_or(BackupError::Invalid("missing ZIP end"))?;
    let mut e = Cursor::new(bytes, end + 4);
    ensure(e.u16()? == 0 && e.u16()? == 0, "multi-disk ZIP unsupported")?;
    let disk_count = e.u16()?;
    let count16 = e.u16()?;
    ensure(disk_count == count16, "inconsistent ZIP counts")?;
    let size32 = e.u32()?;
    let offset32 = e.u32()?;
    let mut count = count16 as u64;
    let mut directory_size = size32 as u64;
    let mut offset = offset32 as u64;
    let mut directory_end = end;
    let locator = end
        .checked_sub(20)
        .filter(|&p| bytes.get(p..p + 4) == Some(&[0x50, 0x4b, 0x06, 0x07][..]));
    if let Some(loc) = locator {
        let mut l = Cursor::new(bytes, loc + 4);
        ensure(l.u32()? == 0, "multi-disk ZIP64 unsupported")?;
        let z_at = size(l.u64()?)?;
        ensure(l.u32()? == 1, "multi-disk ZIP64 unsupported")?;
        let mut z = Cursor::new(bytes, z_at);
        z.sig(0x06064b50)?;
        let record_size = size(z.u64()?)?;
        ensure(
            record_size >= 44 && z.at.checked_add(record_size) == Some(loc),
            "invalid ZIP64 end size",
        )?;
        z.take(4)?;
        ensure(
            z.u32()? == 0 && z.u32()? == 0,
            "multi-disk ZIP64 unsupported",
        )?;
        let disk_entries = z.u64()?;
        count = z.u64()?;
        ensure(disk_entries == count, "inconsistent ZIP64 count")?;
        directory_size = z.u64()?;
        offset = z.u64()?;
        ensure(
            (count16 == u16::MAX || count16 as u64 == count)
                && (size32 == u32::MAX || size32 as u64 == directory_size)
                && (offset32 == u32::MAX || offset32 as u64 == offset),
            "ZIP64 end disagreement",
        )?;
        directory_end = z_at;
    } else {
        ensure(
            count16 != u16::MAX && size32 != u32::MAX && offset32 != u32::MAX,
            "missing ZIP64 locator",
        )?;
    }
    ensure(
        offset.checked_add(directory_size) == Some(directory_end as u64)
            && count <= directory_size / 46,
        "invalid ZIP directory bounds",
    )?;
    let mut c = Cursor::new(bytes, size(offset)?);
    let mut entries = BTreeMap::new();
    let mut names = std::collections::BTreeSet::new();
    let mut declared = 0u64;
    for _ in 0..count {
        ctx.charge(1, 0)?;
        c.sig(0x02014b50)?;
        c.take(2)?;
        let needed = c.u16()?;
        let flags = c.u16()?;
        let method = c.u16()?;
        let timestamp = c.u32()?;
        ensure(needed <= 45, "unsupported ZIP extraction version")?;
        ensure(
            flags & !0x080e == 0 && (method == 0 || method == 8) && (method == 8 || flags & 6 == 0),
            "unsupported ZIP compression or flags",
        )?;
        let crc = c.u32()?;
        let compressed32 = c.u32()?;
        let expanded32 = c.u32()?;
        let name_len = c.u16()? as usize;
        let extra_len = c.u16()? as usize;
        let comment_len = c.u16()? as usize;
        let disk = c.u16()?;
        c.take(2)?;
        let attrs = c.u32()?;
        let local32 = c.u32()?;
        let path = name(c.take(name_len)?)?;
        ctx.charge(0, path.len() as u64)?;
        ensure(names.insert(path.clone()), "duplicate ZIP entry")?;
        ensure(
            attrs & 0x10 == 0 && ((attrs >> 16) & 0xf000 == 0 || (attrs >> 16) & 0xf000 == 0x8000),
            "nonregular ZIP entry",
        )?;
        let extra_bytes = c.take(extra_len)?;
        let zip_extra = extra(extra_bytes)?;
        let mut x = Cursor::new(zip_extra.unwrap_or(&[]), 0);
        let expanded = if expanded32 == u32::MAX {
            x.u64()?
        } else {
            expanded32 as u64
        };
        let compressed = if compressed32 == u32::MAX {
            x.u64()?
        } else {
            compressed32 as u64
        };
        let local = if local32 == u32::MAX {
            x.u64()?
        } else {
            local32 as u64
        };
        let disk = if disk == u16::MAX {
            x.u32()?
        } else {
            disk as u32
        };
        ensure(disk == 0, "multi-disk ZIP entry")?;
        c.take(comment_len)?;
        declared = declared
            .checked_add(expanded)
            .ok_or(BackupError::Invalid("ZIP size overflow"))?;
        budget.check(declared)?;
        ensure(
            entries
                .insert(
                    local,
                    Entry {
                        name: path,
                        needed,
                        timestamp,
                        flags,
                        method,
                        crc,
                        compressed,
                        expanded,
                        offset: local,
                        zip64: expanded32 == u32::MAX || compressed32 == u32::MAX,
                    },
                )
                .is_none(),
            "duplicate local ZIP offset",
        )?;
    }
    ensure(c.at == directory_end, "ZIP directory length mismatch")?;
    let mut next = 0usize;
    let mut total = 0u64;
    let mut result = BTreeMap::new();
    let mut entries = entries.into_values().peekable();
    while let Some(entry) = entries.next() {
        ctx.check()?;
        let boundary = size(entries.peek().map_or(offset, |next| next.offset))?;
        ensure(size(entry.offset)? == next, "local ZIP gap or alias")?;
        let mut l = Cursor::new(bytes, next);
        l.sig(0x04034b50)?;
        ensure(l.u16()? == entry.needed, "local ZIP version disagreement")?;
        ensure(
            l.u16()? == entry.flags && l.u16()? == entry.method,
            "local ZIP header disagreement",
        )?;
        ensure(
            l.u32()? == entry.timestamp,
            "local ZIP timestamp disagreement",
        )?;
        let crc = l.u32()?;
        let compressed32 = l.u32()?;
        let expanded32 = l.u32()?;
        let n = l.u16()? as usize;
        let x = l.u16()? as usize;
        ensure(
            name(l.take(n)?)? == entry.name,
            "local ZIP path disagreement",
        )?;
        let zip_extra = extra(l.take(x)?)?;
        let mut x = Cursor::new(zip_extra.unwrap_or(&[]), 0);
        let expanded = if expanded32 == u32::MAX {
            x.u64()?
        } else {
            expanded32 as u64
        };
        let compressed = if compressed32 == u32::MAX {
            x.u64()?
        } else {
            compressed32 as u64
        };
        let descriptor = entry.flags & 8 != 0;
        if descriptor {
            ensure(
                (crc == 0 || crc == entry.crc)
                    && (expanded == 0 || expanded == entry.expanded)
                    && (compressed == 0 || compressed == entry.compressed),
                "local ZIP descriptor header disagreement",
            )?;
        } else {
            ensure(
                crc == entry.crc && expanded == entry.expanded && compressed == entry.compressed,
                "local ZIP size or CRC disagreement",
            )?;
        }
        let compressed_bytes = l.take(size(entry.compressed)?)?;
        ensure(l.at <= size(offset)?, "ZIP data overlaps directory")?;
        let mut output = Vec::new();
        if entry.method == 0 {
            ensure(
                entry.compressed == entry.expanded,
                "stored ZIP size mismatch",
            )?;
            super::read_bounded(
                &mut &compressed_bytes[..],
                &mut output,
                budget,
                &mut total,
                ctx,
            )?;
        } else {
            let mut decoder = flate2::Decompress::new(false);
            // Count actual inflated bytes before retaining them, even with forged headers.
            let mut buffer = [0u8; 8192];
            loop {
                ctx.check()?;
                let before_in = decoder.total_in();
                let before_out = decoder.total_out();
                let input_at = size(before_in)?;
                let input_end = input_at.saturating_add(8192).min(compressed_bytes.len());
                let status = decoder
                    .decompress(
                        &compressed_bytes[input_at..input_end],
                        &mut buffer,
                        flate2::FlushDecompress::None,
                    )
                    .map_err(|_| BackupError::Invalid("invalid DEFLATE data"))?;
                let n = size(decoder.total_out() - before_out)?;
                total = total
                    .checked_add(n as u64)
                    .ok_or(BackupError::Invalid("ZIP size overflow"))?;
                budget.check(total)?;
                ensure(
                    (output.len() as u64)
                        .checked_add(n as u64)
                        .is_some_and(|v| v <= entry.expanded),
                    "ZIP expanded size mismatch",
                )?;
                output
                    .try_reserve(n)
                    .map_err(|_| BackupError::Invalid("backup allocation failed"))?;
                output.extend_from_slice(&buffer[..n]);
                if status == flate2::Status::StreamEnd {
                    break;
                }
                ensure(
                    n != 0 || decoder.total_in() != before_in,
                    "incomplete DEFLATE stream",
                )?;
            }
            ensure(
                decoder.total_in() == entry.compressed,
                "trailing or incomplete DEFLATE data",
            )?;
        }
        let mut crc = crc32fast::Hasher::new();
        for chunk in output.chunks(8192) {
            ctx.check()?;
            crc.update(chunk);
        }
        ensure(
            output.len() as u64 == entry.expanded && crc.finalize() == entry.crc,
            "ZIP size or CRC mismatch",
        )?;
        if descriptor {
            let wide = entry.zip64 || expanded32 == u32::MAX || compressed32 == u32::MAX;
            // CRC32 can itself equal the optional signature. Try both layouts;
            // accept only one matching central metadata AND the next local/CD boundary.
            let mut matched = None;
            for signed in [false, true] {
                ctx.check()?;
                if let Ok(end) = descriptor_end(bytes, l.at, boundary, &entry, wide, signed) {
                    ensure(matched.is_none(), "ambiguous ZIP data descriptor")?;
                    matched = Some(end);
                }
            }
            l.at = matched.ok_or(BackupError::Invalid("ZIP data descriptor mismatch"))?;
        }
        ensure(l.at == boundary, "local ZIP record boundary mismatch")?;
        next = l.at;
        result.insert(entry.name, output);
    }
    ensure(next == size(offset)?, "unindexed local ZIP entries")?;
    Ok(result)
}

fn descriptor_end(
    bytes: &[u8],
    at: usize,
    boundary: usize,
    entry: &Entry,
    wide: bool,
    signed: bool,
) -> Result<usize> {
    let width = if wide { 8 } else { 4 };
    let length = 4 + 2 * width + if signed { 4 } else { 0 };
    ensure(
        at.checked_add(length) == Some(boundary),
        "descriptor length mismatch",
    )?;
    let mut d = Cursor::new(bytes, at);
    if signed {
        d.sig(0x08074b50)?;
    }
    let crc = d.u32()?;
    let compressed = if wide { d.u64()? } else { d.u32()? as u64 };
    let expanded = if wide { d.u64()? } else { d.u32()? as u64 };
    ensure(
        crc == entry.crc && compressed == entry.compressed && expanded == entry.expanded,
        "descriptor metadata mismatch",
    )?;
    Ok(d.at)
}
