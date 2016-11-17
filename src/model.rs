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
