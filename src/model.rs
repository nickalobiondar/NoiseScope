//! Core data model for `noisescope`.
//!
//! A **protocol specification** is a declarative state machine: a set of named
//! states, a designated initial state, a set of accepting (terminal) states,
//! and a list of transitions. Each transition is triggered by a *message type*
//! sent by a particular *role* and moves the machine from one state to another.
//!
//! A **transcript** is an ordered list of *events* (observed messages) that we
//! replay against the specification. Each event carries a role, a message type,
//! an optional nonce and an optional sequence number, plus free-form metadata.
//!
//! Nothing in this model is cryptographic. Nonces and sequence numbers are
//! plain integers used to check *structural* invariants (freshness / ordering),
//! not to verify any cryptographic property. See `docs/PROTOCOL.md`.

use std::collections::BTreeMap;

/// A participant role in a handshake. Kept as a small string so specs can use
/// whatever names they like (`initiator`, `responder`, `client`, `server`, ...).
pub type Role = String;

/// A message type label, e.g. `"e"`, `"s"`, `"ClientHello"`, `"Commit"`.
pub type MsgType = String;

/// A named protocol state.
pub type StateName = String;

/// A single transition rule in the protocol state machine.
#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    /// State the machine must be in for this transition to apply.
    pub from: StateName,
    /// State entered after the transition fires.
    pub to: StateName,
    /// Role expected to have sent the message.
    pub role: Role,
    /// Message type that triggers the transition.
    pub msg: MsgType,
    /// If true, the message must carry a nonce that has not been seen before.
    pub requires_fresh_nonce: bool,
    /// If true, the message must carry a sequence number equal to the previous
    /// sequence for this role plus one (per-role monotonic counter).
    pub requires_seq: bool,
    /// Optional human note surfaced in reports.
    pub note: Option<String>,
}

/// The declarative protocol specification (a state machine).
#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolSpec {
    pub name: String,
    pub roles: Vec<Role>,
    pub initial: StateName,
    pub accepting: Vec<StateName>,
    pub states: Vec<StateName>,
    pub transitions: Vec<Transition>,
    /// Free-form description surfaced in reports/docs.
    pub description: Option<String>,
}

impl ProtocolSpec {
    /// Find transitions that apply for a given `(from, role, msg)` key.
    pub fn matching(&self, from: &str, role: &str, msg: &str) -> Vec<&Transition> {
        self.transitions
            .iter()
            .filter(|t| t.from == from && t.role == role && t.msg == msg)
            .collect()
    }

    pub fn is_accepting(&self, state: &str) -> bool {
        self.accepting.iter().any(|s| s == state)
    }

    pub fn has_state(&self, state: &str) -> bool {
        self.states.iter().any(|s| s == state)
    }

    /// Structural sanity checks on the spec itself (not on any transcript).
    /// Returns a list of human-readable problems; empty means the spec is
    /// internally consistent.
    pub fn lint(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if !self.has_state(&self.initial) {
            problems.push(format!("initial state `{}` is not declared", self.initial));
        }
        for acc in &self.accepting {
            if !self.has_state(acc) {
                problems.push(format!("accepting state `{}` is not declared", acc));
            }
        }
        for (i, t) in self.transitions.iter().enumerate() {
            if !self.has_state(&t.from) {
                problems.push(format!(
                    "transition #{i} references unknown `from` state `{}`",
                    t.from
                ));
            }
            if !self.has_state(&t.to) {
                problems.push(format!(
                    "transition #{i} references unknown `to` state `{}`",
                    t.to
                ));
            }
            if !self.roles.iter().any(|r| r == &t.role) {
                problems.push(format!(
                    "transition #{i} references undeclared role `{}`",
                    t.role
                ));
            }
        }
        problems
    }
}

/// A single observed handshake event.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    /// Stable identifier used in reports and minimization.
    pub id: String,
    pub role: Role,
    pub msg: MsgType,
    pub nonce: Option<u64>,
    pub seq: Option<u64>,
    /// Arbitrary string key/value metadata (e.g. `cipher=chachapoly`).
    pub meta: BTreeMap<String, String>,
}

impl Event {
    pub fn new(id: impl Into<String>, role: impl Into<String>, msg: impl Into<String>) -> Self {
        Event {
            id: id.into(),
            role: role.into(),
            msg: msg.into(),
            nonce: None,
            seq: None,
            meta: BTreeMap::new(),
        }
    }

    /// A concise label used in path rendering, e.g. `initiator:e#3`.
    pub fn label(&self) -> String {
        let mut s = format!("{}:{}", self.role, self.msg);
        if let Some(n) = self.nonce {
            s.push_str(&format!("#{n}"));
        }
        s
    }
}

/// A transcript: an ordered sequence of events, plus the spec name it targets.
#[derive(Debug, Clone, PartialEq)]
pub struct Transcript {
    pub protocol: String,
    pub events: Vec<Event>,
}

impl Transcript {
    pub fn new(protocol: impl Into<String>) -> Self {
        Transcript {
            protocol: protocol.into(),
            events: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_spec() -> ProtocolSpec {
        ProtocolSpec {
            name: "tiny".into(),
            roles: vec!["a".into(), "b".into()],
            initial: "s0".into(),
            accepting: vec!["s2".into()],
            states: vec!["s0".into(), "s1".into(), "s2".into()],
            transitions: vec![
                Transition {
                    from: "s0".into(),
                    to: "s1".into(),
                    role: "a".into(),
                    msg: "hello".into(),
                    requires_fresh_nonce: true,
                    requires_seq: false,
                    note: None,
                },
                Transition {
                    from: "s1".into(),
                    to: "s2".into(),
                    role: "b".into(),
                    msg: "ack".into(),
                    requires_fresh_nonce: false,
                    requires_seq: false,
                    note: None,
                },
            ],
            description: None,
        }
    }

    #[test]
    fn lint_clean_spec() {
        assert!(tiny_spec().lint().is_empty());
    }

    #[test]
    fn lint_detects_bad_initial() {
        let mut s = tiny_spec();
        s.initial = "nope".into();
        assert!(!s.lint().is_empty());
    }

    #[test]
    fn matching_transitions() {
        let s = tiny_spec();
        assert_eq!(s.matching("s0", "a", "hello").len(), 1);
        assert_eq!(s.matching("s0", "b", "hello").len(), 0);
    }

    #[test]
    fn event_label() {
        let mut e = Event::new("e0", "initiator", "e");
        e.nonce = Some(3);
        assert_eq!(e.label(), "initiator:e#3");
    }
}
