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
pub fn parse_protocol(input: &str) -> Result<ProtocolSpec, ParseError> {
    let doc = json::parse(input)?;
    parse_protocol_value(&doc)
}

fn parse_protocol_value(doc: &Json) -> Result<ProtocolSpec, ParseError> {
    let name = require_str(doc, "name")?;
    let roles = str_array(doc, "roles")?;
    let initial = require_str(doc, "initial")?;
    let states = str_array(doc, "states")?;
    let accepting = str_array(doc, "accepting")?;
    let description = opt_str(doc, "description");

    let trans_arr = doc
        .get("transitions")
        .and_then(Json::as_array)
        .ok_or_else(|| ParseError("field `transitions` must be an array".into()))?;

    let mut transitions = Vec::with_capacity(trans_arr.len());
    for (i, t) in trans_arr.iter().enumerate() {
        let ctx = |k: &str| ParseError(format!("transition #{i}: missing field `{k}`"));
        let from = t
            .get("from")
            .and_then(Json::as_str)
            .ok_or_else(|| ctx("from"))?;
        let to = t
            .get("to")
            .and_then(Json::as_str)
            .ok_or_else(|| ctx("to"))?;
        let role = t
            .get("role")
            .and_then(Json::as_str)
            .ok_or_else(|| ctx("role"))?;
        let msg = t
            .get("msg")
            .and_then(Json::as_str)
            .ok_or_else(|| ctx("msg"))?;
        let requires_fresh_nonce = t
            .get("requires_fresh_nonce")
            .and_then(Json::as_bool)
            .unwrap_or(false);
        let requires_seq = t
            .get("requires_seq")
            .and_then(Json::as_bool)
            .unwrap_or(false);
        let note = opt_str(t, "note");
        transitions.push(Transition {
            from: from.to_string(),
            to: to.to_string(),
            role: role.to_string(),
            msg: msg.to_string(),
            requires_fresh_nonce,
            requires_seq,
