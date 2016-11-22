//! Parsers that build the [`model`] types from JSON fixtures.
//!
//! Two document kinds are supported:
//!  * a **protocol spec** (`kind: "protocol"`)
//!  * a **transcript** (`kind: "transcript"`)
//!
//! The `kind` field is optional; documents are recognized structurally when it
//! is absent (a `transitions` array means protocol, an `events` array means
//! transcript). See `docs/PROTOCOL.md` for the full schema.

use crate::json::{self, Json};
use crate::model::{Event, ProtocolSpec, Transcript, Transition};
use std::collections::BTreeMap;

/// Error produced while turning JSON into model types.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "parse error: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

impl From<json::JsonError> for ParseError {
    fn from(e: json::JsonError) -> Self {
        ParseError(e.to_string())
    }
}

fn require_str(obj: &Json, key: &str) -> Result<String, ParseError> {
    obj.get(key)
        .and_then(Json::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| ParseError(format!("missing required string field `{key}`")))
}

fn opt_str(obj: &Json, key: &str) -> Option<String> {
    obj.get(key).and_then(Json::as_str).map(|s| s.to_string())
}

fn str_array(obj: &Json, key: &str) -> Result<Vec<String>, ParseError> {
    let arr = obj
        .get(key)
        .and_then(Json::as_array)
        .ok_or_else(|| ParseError(format!("field `{key}` must be an array")))?;
    let mut out = Vec::with_capacity(arr.len());
    for (i, item) in arr.iter().enumerate() {
        match item.as_str() {
            Some(s) => out.push(s.to_string()),
            None => {
                return Err(ParseError(format!(
                    "element {i} of `{key}` must be a string"
                )))
            }
        }
    }
    Ok(out)
}

/// Parse a protocol specification document.
