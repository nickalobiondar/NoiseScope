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
            note,
        });
    }

    Ok(ProtocolSpec {
        name,
        roles,
        initial,
        accepting,
        states,
        transitions,
        description,
    })
}

/// Parse a transcript document.
pub fn parse_transcript(input: &str) -> Result<Transcript, ParseError> {
    let doc = json::parse(input)?;
    parse_transcript_value(&doc)
}

fn parse_transcript_value(doc: &Json) -> Result<Transcript, ParseError> {
    let protocol = require_str(doc, "protocol")?;
    let events_arr = doc
        .get("events")
        .and_then(Json::as_array)
        .ok_or_else(|| ParseError("field `events` must be an array".into()))?;

    let mut events = Vec::with_capacity(events_arr.len());
    for (i, e) in events_arr.iter().enumerate() {
        let role = e
            .get("role")
            .and_then(Json::as_str)
            .ok_or_else(|| ParseError(format!("event #{i}: missing `role`")))?;
        let msg = e
            .get("msg")
            .and_then(Json::as_str)
            .ok_or_else(|| ParseError(format!("event #{i}: missing `msg`")))?;
        let id = opt_str(e, "id").unwrap_or_else(|| format!("e{i}"));
        let nonce = e.get("nonce").and_then(Json::as_u64);
        let seq = e.get("seq").and_then(Json::as_u64);
        let mut meta = BTreeMap::new();
        if let Some(Json::Obj(entries)) = e.get("meta") {
            for (k, v) in entries {
                let val = match v {
                    Json::Str(s) => s.clone(),
                    Json::Num(n) => {
                        if n.fract() == 0.0 {
                            format!("{}", *n as i64)
                        } else {
                            format!("{n}")
                        }
                    }
                    Json::Bool(b) => b.to_string(),
                    other => other.to_compact(),
                };
                meta.insert(k.clone(), val);
            }
        }
        events.push(Event {
            id,
            role: role.to_string(),
            msg: msg.to_string(),
            nonce,
            seq,
            meta,
        });
    }

    Ok(Transcript { protocol, events })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_protocol_basic() {
        let src = r#"{
            "name": "demo",
            "roles": ["i", "r"],
            "initial": "start",
            "states": ["start", "done"],
            "accepting": ["done"],
            "transitions": [
                {"from": "start", "to": "done", "role": "i", "msg": "e",
                 "requires_fresh_nonce": true}
            ]
        }"#;
        let spec = parse_protocol(src).unwrap();
        assert_eq!(spec.name, "demo");
        assert_eq!(spec.transitions.len(), 1);
        assert!(spec.transitions[0].requires_fresh_nonce);
    }

    #[test]
    fn parse_transcript_basic() {
        let src = r#"{
            "protocol": "demo",
            "events": [
                {"role": "i", "msg": "e", "nonce": 7, "meta": {"cipher": "aead"}}
            ]
        }"#;
        let t = parse_transcript(src).unwrap();
        assert_eq!(t.events.len(), 1);
        assert_eq!(t.events[0].nonce, Some(7));
        assert_eq!(
            t.events[0].meta.get("cipher").map(String::as_str),
            Some("aead")
        );
        assert_eq!(t.events[0].id, "e0");
    }

    #[test]
    fn missing_field_errors() {
        assert!(parse_protocol(r#"{"name":"x"}"#).is_err());
    }

    #[test]
    fn meta_number_coerced() {
        let src = r#"{"protocol":"d","events":[{"role":"i","msg":"e","meta":{"len":16}}]}"#;
        let t = parse_transcript(src).unwrap();
        assert_eq!(t.events[0].meta.get("len").map(String::as_str), Some("16"));
    }
}

// draft note 43
