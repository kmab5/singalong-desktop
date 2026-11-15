//! Preflight without a Value tree: discard scalars/arrays and retain only active
//! object keys for duplicate detection. Charge each value/key before parsing and
//! each decoded string before retaining it. Only after bounded preflight succeeds
//! does serde allocate the typed model. Nesting retains serde's recursion limit.
use std::{collections::BTreeSet, fmt, io::{Read, Write}};
use serde::de::{DeserializeSeed, Visitor, SeqAccess, MapAccess, Error};
use super::{control::{Context, CheckedReader}, BackupBudget, BackupError, Result};

struct Node<'a,'c>(&'a Context<'c>);
impl<'de> DeserializeSeed<'de> for Node<'_,'_> {
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> std::result::Result<(),D::Error> {
        self.0.charge(1, 0).map_err(D::Error::custom)?;
        d.deserialize_any(self)
    }
}
impl<'de> Visitor<'de> for Node<'_,'_> {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result { f.write_str("bounded JSON without duplicate keys") }
    fn visit_bool<E: Error>(self, _: bool) -> std::result::Result<(),E> { Ok(()) }
    fn visit_i64<E: Error>(self, _: i64) -> std::result::Result<(),E> { Ok(()) }
    fn visit_u64<E: Error>(self, _: u64) -> std::result::Result<(),E> { Ok(()) }
    fn visit_f64<E: Error>(self, _: f64) -> std::result::Result<(),E> { Ok(()) }
    fn visit_unit<E: Error>(self) -> std::result::Result<(),E> { Ok(()) }
    fn visit_str<E: Error>(self, v: &str) -> std::result::Result<(),E> { self.0.charge(0, v.len() as u64).map_err(E::custom) }
    fn visit_string<E: Error>(self, v: String) -> std::result::Result<(),E> { self.visit_str(&v) }
    fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> std::result::Result<(),A::Error> {
        while a.next_element_seed(Node(self.0))?.is_some() { self.0.check().map_err(A::Error::custom)?; }
        Ok(())
    }
    fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> std::result::Result<(),A::Error> {
        let mut keys = BTreeSet::new();
        while let Some(key) = a.next_key_seed(Key(self.0))? {
            if !keys.insert(key) { return Err(A::Error::custom("duplicate JSON key")); }
            a.next_value_seed(Node(self.0))?;
        }
        Ok(())
    }
}
struct Key<'a,'c>(&'a Context<'c>);
impl<'de> DeserializeSeed<'de> for Key<'_,'_> {
    type Value = String;
    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> std::result::Result<String,D::Error> {
        self.0.charge(1, 0).map_err(D::Error::custom)?;
        d.deserialize_str(self)
    }
}
impl<'de> Visitor<'de> for Key<'_,'_> {
    type Value = String;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result { f.write_str("bounded JSON key") }
    fn visit_str<E: Error>(self, v: &str) -> std::result::Result<String,E> {
        self.0.charge(0, v.len() as u64).map_err(E::custom)?; Ok(v.to_owned())
    }
    fn visit_string<E: Error>(self, v: String) -> std::result::Result<String,E> {
        self.0.charge(0, v.len() as u64).map_err(E::custom)?; Ok(v)
    }
}
pub(super) fn preflight(bytes: &[u8], ctx: &Context<'_>) -> Result<()> {
    let reader = StringGuard { reader: CheckedReader::new(bytes, ctx), ctx,
        in_string: false, escaped: false, digits: 0, unit: 0, high: false, length: 0 };
    let mut d = serde_json::Deserializer::from_reader(reader);
    ctx.finish(Node(ctx).deserialize(&mut d))?;
    ctx.finish(d.end())
}

/// Bound serde's temporary decoded-string scratch buffer before the visitor is
/// called. Count decoded UTF-8 bytes, including Unicode escapes/surrogate pairs.
/// Invalid escape sequences are still rejected by serde, not repaired here.
struct StringGuard<'a,'c,R> {
    reader: CheckedReader<'a,'c,R>, ctx: &'a Context<'c>, in_string: bool,
    escaped: bool, digits: u8, unit: u16, high: bool, length: u64,
}
impl<R: Read> Read for StringGuard<'_,'_,R> {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        if bytes.is_empty() { return Ok(0); }
        let n = self.reader.read(&mut bytes[..1])?;
        if n == 0 { return Ok(0); }
        let b = bytes[0];
        let was_in_string = self.in_string;
        if !self.in_string {
            if b == b'"' { self.in_string = true; self.length = 0; self.high = false; }
        } else if self.digits != 0 {
            let digit = match b { b'0'..=b'9' => b-b'0', b'a'..=b'f' => b-b'a'+10, b'A'..=b'F' => b-b'A'+10, _ => 0 };
            self.unit = (self.unit << 4) | digit as u16;
            self.digits -= 1;
            if self.digits == 0 {
                if self.high && (0xdc00..=0xdfff).contains(&self.unit) {
                    self.length += 4; self.high = false;
                } else {
                    if self.high { self.length += 3; self.high = false; }
                    if (0xd800..=0xdbff).contains(&self.unit) { self.high = true; }
                    else { self.length += if self.unit < 0x80 { 1 } else if self.unit < 0x800 { 2 } else { 3 }; }
                }
            }
        } else if self.escaped {
            self.escaped = false;
            if b == b'u' { self.digits = 4; self.unit = 0; }
            else { self.length += 1; }
        } else if b == b'\\' { self.escaped = true; }
        else if b == b'"' { self.in_string = false; }
        else { self.length += 1; }
        if was_in_string || self.in_string {
            self.ctx.string_token(self.length).map_err(std::io::Error::other)?;
        }
        Ok(n)
    }
}
pub(super) fn typed<T: serde::de::DeserializeOwned>(bytes: &[u8], ctx: &Context<'_>) -> Result<T> {
    ctx.finish(serde_json::from_reader(CheckedReader::new(bytes, ctx)))
}
pub(super) fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8], ctx: &Context<'_>) -> Result<T> {
    preflight(bytes, ctx)?; typed(bytes, ctx)
}

/// Count the exact serialized envelope without retaining it. This is a sink for
/// trusted serde_json output, not an alternate parser for untrusted input. Every
/// container, scalar and object key is one node, matching Node/Key above. Strings
/// charge decoded UTF-8 bytes, not their escaped wire length. The same Context
/// also carries the embedded-preference work charged by validate_inner.
pub(super) fn preflight_serialized<T: serde::Serialize>(value: &T, budget: BackupBudget, ctx: &Context<'_>) -> Result<()> {
    struct Counter<'a,'c> {
        ctx: &'a Context<'c>, budget: BackupBudget, bytes: u64, byte_limit: bool,
        in_string: bool, escaped: bool, digits: u8, unit: u16, high: bool,
        scalar: bool, strings: u64,
    }
    impl Counter<'_,'_> {
        fn flush_strings(&mut self) -> std::io::Result<()> {
            self.ctx.charge(0, self.strings).map_err(std::io::Error::other)?;
            self.strings = 0;
            Ok(())
        }
        fn byte(&mut self, b: u8) -> std::io::Result<()> {
            if self.in_string {
                if self.digits != 0 {
                    let digit = match b { b'0'..=b'9' => b-b'0', b'a'..=b'f' => b-b'a'+10, b'A'..=b'F' => b-b'A'+10, _ => 0 };
                    self.unit = (self.unit << 4) | digit as u16;
                    self.digits -= 1;
                    if self.digits == 0 {
                        if self.high && (0xdc00..=0xdfff).contains(&self.unit) {
                            self.strings += 4; self.high = false;
                        } else if (0xd800..=0xdbff).contains(&self.unit) {
                            self.high = true;
                        } else {
                            self.strings += if self.unit < 0x80 { 1 } else if self.unit < 0x800 { 2 } else { 3 };
                        }
                    }
                } else if self.escaped {
                    self.escaped = false;
                    if b == b'u' { self.digits = 4; self.unit = 0; }
                    else { self.strings += 1; }
                } else if b == b'\\' { self.escaped = true; }
                else if b == b'"' { self.in_string = false; }
                else { self.strings += 1; }
            } else {
                match b {
                    b'"' => {
                        self.ctx.charge(1, 0).map_err(std::io::Error::other)?;
                        self.in_string = true; self.scalar = false;
                    }
                    b'{' | b'[' => {
                        self.ctx.charge(1, 0).map_err(std::io::Error::other)?;
                        self.scalar = false;
                    }
                    b'}' | b']' | b',' | b':' | b' ' | b'\r' | b'\n' | b'\t' => self.scalar = false,
                    _ if !self.scalar => {
                        self.ctx.charge(1, 0).map_err(std::io::Error::other)?;
                        self.scalar = true;
                    }
                    _ => {}
                }
            }
            Ok(())
        }
    }
    impl Write for Counter<'_,'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            for chunk in bytes.chunks(8192) {
                self.ctx.io_check()?;
                let next = self.bytes.checked_add(chunk.len() as u64);
                if next.is_none_or(|n| self.budget.check(n).is_err()) {
                    self.byte_limit = true;
                    return Err(std::io::Error::other("backup byte budget exceeded"));
                }
                self.bytes = next.unwrap_or(u64::MAX);
                for &b in chunk { self.byte(b)?; }
                self.flush_strings()?;
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> { self.flush_strings() }
    }
    let mut counter = Counter { ctx, budget, bytes: 0, byte_limit: false,
        in_string: false, escaped: false, digits: 0, unit: 0, high: false, scalar: false, strings: 0 };
    let result = serde_json::to_writer(&mut counter, value);
    ctx.check()?;
    if counter.byte_limit { return Err(BackupError::BudgetExceeded); }
    ctx.finish(result)
}
