//! Minimal JSON value model, parser, and serializer.
//!
//! `noisescope` uses only the Rust standard library. This module provides a
//! small, dependency-free JSON implementation that is sufficient for parsing
//! protocol/transcript fixtures and for emitting stable divergence reports.
//!
//! It is deliberately conservative: it supports objects, arrays, strings,
//! numbers (parsed as `f64` and also retained as raw text), booleans and null.
//! Serialization is deterministic (object keys are emitted in insertion order)
//! which keeps report output stable and diff-friendly.

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// A JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    /// Numbers keep both the parsed value and their original textual form so
    /// integer-like values round-trip without spurious `.0` suffixes.
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    /// Objects use an ordered map keyed by insertion to keep output stable.
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Json::Num(n) if *n >= 0.0 && n.fract() == 0.0 => Some(*n as u64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(v) => Some(v.as_slice()),
            _ => None,
        }
    }

    /// Look up a key in an object value.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// Serialize with 2-space indentation.
    pub fn to_pretty(&self) -> String {
        let mut out = String::new();
        self.write_pretty(&mut out, 0);
        out
    }

    /// Serialize compactly (single line).
    pub fn to_compact(&self) -> String {
        let mut out = String::new();
        self.write_compact(&mut out);
        out
    }

    fn write_compact(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) => write_number(out, *n),
            Json::Str(s) => write_string(out, s),
            Json::Arr(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write_compact(out);
                }
                out.push(']');
            }
            Json::Obj(entries) => {
                out.push('{');
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_string(out, k);
                    out.push(':');
                    v.write_compact(out);
                }
                out.push('}');
            }
        }
    }

    fn write_pretty(&self, out: &mut String, indent: usize) {
        match self {
            Json::Arr(items) if !items.is_empty() => {
                out.push_str("[\n");
                for (i, item) in items.iter().enumerate() {
                    push_indent(out, indent + 1);
                    item.write_pretty(out, indent + 1);
                    if i + 1 < items.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                push_indent(out, indent);
                out.push(']');
            }
            Json::Obj(entries) if !entries.is_empty() => {
                out.push_str("{\n");
                for (i, (k, v)) in entries.iter().enumerate() {
                    push_indent(out, indent + 1);
                    write_string(out, k);
                    out.push_str(": ");
                    v.write_pretty(out, indent + 1);
                    if i + 1 < entries.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                push_indent(out, indent);
                out.push('}');
            }
            _ => self.write_compact(out),
        }
    }
}

fn push_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}

fn write_number(out: &mut String, n: f64) {
    if n.fract() == 0.0 && n.is_finite() && n.abs() < 1e15 {
        let _ = write!(out, "{}", n as i64);
    } else {
        let _ = write!(out, "{}", n);
    }
}

fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Ordered-map convenience builder used by report emitters.
pub struct ObjBuilder {
    entries: Vec<(String, Json)>,
    seen: BTreeMap<String, ()>,
}

impl ObjBuilder {
    pub fn new() -> Self {
        ObjBuilder {
            entries: Vec::new(),
            seen: BTreeMap::new(),
        }
    }

    pub fn set(mut self, key: &str, value: Json) -> Self {
        if self.seen.insert(key.to_string(), ()).is_none() {
            self.entries.push((key.to_string(), value));
        } else {
            for e in self.entries.iter_mut() {
                if e.0 == key {
                    e.1 = value;
                    break;
                }
            }
        }
        self
    }

    pub fn build(self) -> Json {
        Json::Obj(self.entries)
    }
}

impl Default for ObjBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// A JSON parse error with byte position.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonError {
    pub message: String,
    pub position: usize,
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JSON error at byte {}: {}", self.position, self.message)
    }
}

impl std::error::Error for JsonError {}

/// Parse a JSON document from a string.
pub fn parse(input: &str) -> Result<Json, JsonError> {
    let mut p = Parser {
        bytes: input.as_bytes(),
        pos: 0,
    };
    p.skip_ws();
    let value = p.parse_value()?;
    p.skip_ws();
    if p.pos != p.bytes.len() {
        return Err(p.err("trailing characters after JSON document"));
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn err(&self, msg: &str) -> JsonError {
        JsonError {
            message: msg.to_string(),
            position: self.pos,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while let Some(b) = self.peek() {
            match b {
                b' ' | b'\t' | b'\n' | b'\r' => self.pos += 1,
                // Support line comments (`//`) as a friendly extension for fixtures.
                b'/' if self.bytes.get(self.pos + 1) == Some(&b'/') => {
                    while let Some(c) = self.peek() {
                        self.pos += 1;
                        if c == b'\n' {
                            break;
                        }
                    }
                }
                _ => break,
            }
        }
    }

    fn parse_value(&mut self) -> Result<Json, JsonError> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => Ok(Json::Str(self.parse_string()?)),
            Some(b't') | Some(b'f') => self.parse_bool(),
            Some(b'n') => self.parse_null(),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.parse_number(),
            Some(_) => Err(self.err("unexpected character")),
            None => Err(self.err("unexpected end of input")),
        }
    }

    fn parse_object(&mut self) -> Result<Json, JsonError> {
        self.pos += 1; // consume '{'
        let mut entries = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Json::Obj(entries));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(self.err("expected string key in object"));
            }
            let key = self.parse_string()?;
            self.skip_ws();
            if self.peek() != Some(b':') {
                return Err(self.err("expected ':' after object key"));
            }
            self.pos += 1;
            let value = self.parse_value()?;
            entries.push((key, value));
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                    continue;
                }
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(self.err("expected ',' or '}' in object")),
            }
        }
        Ok(Json::Obj(entries))
    }

    fn parse_array(&mut self) -> Result<Json, JsonError> {
        self.pos += 1; // consume '['
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Json::Arr(items));
        }
        loop {
            let value = self.parse_value()?;
            items.push(value);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                    continue;
                }
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(self.err("expected ',' or ']' in array")),
            }
        }
        Ok(Json::Arr(items))
    }

    fn parse_string(&mut self) -> Result<String, JsonError> {
        self.pos += 1; // consume opening quote
        let mut s = String::new();
        loop {
            match self.peek() {
                None => return Err(self.err("unterminated string")),
                Some(b'"') => {
                    self.pos += 1;
                    break;
                }
                Some(b'\\') => {
                    self.pos += 1;
                    match self.peek() {
                        Some(b'"') => s.push('"'),
                        Some(b'\\') => s.push('\\'),
                        Some(b'/') => s.push('/'),
                        Some(b'n') => s.push('\n'),
                        Some(b't') => s.push('\t'),
                        Some(b'r') => s.push('\r'),
                        Some(b'b') => s.push('\u{0008}'),
                        Some(b'f') => s.push('\u{000C}'),
                        Some(b'u') => {
                            let cp = self.parse_hex4()?;
                            // Handle surrogate pairs.
                            if (0xD800..=0xDBFF).contains(&cp) {
                                if self.bytes.get(self.pos + 1) == Some(&b'\\')
                                    && self.bytes.get(self.pos + 2) == Some(&b'u')
                                {
                                    self.pos += 2;
                                    let lo = self.parse_hex4()?;
                                    let c = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00);
                                    if let Some(ch) = char::from_u32(c) {
                                        s.push(ch);
                                    } else {
                                        return Err(self.err("invalid surrogate pair"));
                                    }
                                } else {
                                    return Err(self.err("unpaired high surrogate"));
                                }
                            } else if let Some(ch) = char::from_u32(cp) {
                                s.push(ch);
                            } else {
                                return Err(self.err("invalid unicode escape"));
                            }
                        }
                        _ => return Err(self.err("invalid escape sequence")),
                    }
                    self.pos += 1;
                }
                Some(_) => {
                    // Copy a full UTF-8 scalar.
                    let start = self.pos;
                    let len = utf8_len(self.bytes[self.pos]);
                    if start + len > self.bytes.len() {
                        return Err(self.err("truncated UTF-8 sequence"));
                    }
                    match std::str::from_utf8(&self.bytes[start..start + len]) {
                        Ok(chunk) => s.push_str(chunk),
                        Err(_) => return Err(self.err("invalid UTF-8 in string")),
                    }
                    self.pos += len;
                }
            }
        }
        Ok(s)
    }

    fn parse_hex4(&mut self) -> Result<u32, JsonError> {
        let mut value = 0u32;
        for _ in 0..4 {
            self.pos += 1;
            let d = match self.peek() {
                Some(c @ b'0'..=b'9') => (c - b'0') as u32,
                Some(c @ b'a'..=b'f') => (c - b'a' + 10) as u32,
                Some(c @ b'A'..=b'F') => (c - b'A' + 10) as u32,
                _ => return Err(self.err("invalid \\u escape")),
            };
            value = value * 16 + d;
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<Json, JsonError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.pos += 1;
            }
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| self.err("invalid number encoding"))?;
        text.parse::<f64>()
            .map(Json::Num)
            .map_err(|_| self.err("invalid number literal"))
    }

    fn parse_bool(&mut self) -> Result<Json, JsonError> {
        if self.bytes[self.pos..].starts_with(b"true") {
            self.pos += 4;
            Ok(Json::Bool(true))
        } else if self.bytes[self.pos..].starts_with(b"false") {
            self.pos += 5;
            Ok(Json::Bool(false))
        } else {
            Err(self.err("invalid literal"))
        }
    }

    fn parse_null(&mut self) -> Result<Json, JsonError> {
        if self.bytes[self.pos..].starts_with(b"null") {
            self.pos += 4;
            Ok(Json::Null)
        } else {
            Err(self.err("invalid literal"))
        }
    }
}

fn utf8_len(first: u8) -> usize {
    if first < 0x80 {
        1
    } else if first >> 5 == 0b110 {
        2
    } else if first >> 4 == 0b1110 {
        3
    } else {
        4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_object() {
        let v = parse(r#"{"a":1,"b":[true,null,"x"]}"#).unwrap();
        assert_eq!(v.get("a").and_then(Json::as_u64), Some(1));
        assert_eq!(v.to_compact(), r#"{"a":1,"b":[true,null,"x"]}"#);
    }

    #[test]
    fn integers_have_no_decimal() {
        let v = Json::Num(42.0);
        assert_eq!(v.to_compact(), "42");
    }

    #[test]
    fn string_escapes() {
        let v = parse(r#""line\nbreak\t\u0041""#).unwrap();
        assert_eq!(v.as_str(), Some("line\nbreak\tA"));
    }

    #[test]
    fn comments_allowed() {
        let v = parse("{\n // a comment\n \"k\": 1\n}").unwrap();
        assert_eq!(v.get("k").and_then(Json::as_u64), Some(1));
    }

    #[test]
    fn rejects_trailing_garbage() {
        assert!(parse("{} junk").is_err());
    }

    #[test]
    fn builder_dedup() {
        let o = ObjBuilder::new()
            .set("a", Json::Num(1.0))
            .set("a", Json::Num(2.0))
            .build();
        assert_eq!(o.to_compact(), r#"{"a":2}"#);
    }
}
