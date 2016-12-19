//! Transcript replay engine and invariant checker.
//!
//! Given a [`ProtocolSpec`] and a [`Transcript`], the engine walks the events
//! in order, driving the declared state machine and enforcing four families of
//! structural invariants:
//!
//!  1. **role** — the event's role must match a transition available from the
//!     current state.
//!  2. **order** — there must exist a transition `(state, role, msg)`; if not,
//!     the event is *unexpected* for the current state.
//!  3. **nonce** — when a transition requires a fresh nonce, the event must
//!     carry a nonce that has not been observed before in this run.
//!  4. **sequence** — when a transition requires sequencing, the event's `seq`
//!     must equal the previous per-role sequence + 1 (monotonic, no gaps).
//!
//! These are *structural* checks. `noisescope` makes **no** claim about the
//! cryptographic soundness of any protocol; it only reports whether a transcript
//! conforms to the declared abstract state machine and its metadata invariants.

use crate::model::{Event, ProtocolSpec, Transcript};
use std::collections::{BTreeMap, BTreeSet};

/// The category of an invariant violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationKind {
    /// No transition exists for the current state + role + message.
    UnexpectedMessage,
    /// A transition exists for the message but not for this role in this state.
    RoleMismatch,
    /// A required fresh nonce was missing or replayed.
    NonceReplay,
    /// A required sequence number was missing or out of order.
    SequenceViolation,
    /// The run ended in a non-accepting state.
    NotAccepting,
}

impl ViolationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ViolationKind::UnexpectedMessage => "unexpected_message",
            ViolationKind::RoleMismatch => "role_mismatch",
            ViolationKind::NonceReplay => "nonce_replay",
            ViolationKind::SequenceViolation => "sequence_violation",
            ViolationKind::NotAccepting => "not_accepting",
        }
    }
}

/// A single invariant violation encountered during replay.
#[derive(Debug, Clone, PartialEq)]
pub struct Violation {
    pub kind: ViolationKind,
    /// Index of the offending event (or the event count for `NotAccepting`).
    pub event_index: usize,
    /// The event id, if applicable.
