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
