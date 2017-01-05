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
    pub event_id: Option<String>,
    pub state: String,
    pub detail: String,
}

/// One step recorded during a successful (or partial) walk of the machine.
#[derive(Debug, Clone, PartialEq)]
pub struct PathStep {
    pub event_index: usize,
    pub event_label: String,
    pub from: String,
    pub to: String,
}

/// The full outcome of replaying a transcript against a spec.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplayResult {
    pub protocol: String,
    /// State the machine halted in.
    pub final_state: String,
    pub reached_accepting: bool,
    pub violations: Vec<Violation>,
    pub path: Vec<PathStep>,
    /// Number of events actually consumed before the run halted (or all).
    pub consumed: usize,
}

impl ReplayResult {
    /// True when no invariants were violated and the machine accepted.
    pub fn is_conforming(&self) -> bool {
        self.violations.is_empty() && self.reached_accepting
    }
}

/// Replay a transcript against a spec, collecting all violations.
///
/// The engine is *fail-soft*: after an `UnexpectedMessage`/`RoleMismatch` it
/// does not advance the state (the offending event is skipped) so that later
/// events can still be evaluated. This yields richer divergence reports than a
/// fail-fast walk while remaining deterministic.
pub fn replay(spec: &ProtocolSpec, transcript: &Transcript) -> ReplayResult {
    let mut state = spec.initial.clone();
    let mut seen_nonces: BTreeSet<u64> = BTreeSet::new();
    let mut last_seq: BTreeMap<String, u64> = BTreeMap::new();
    let mut violations = Vec::new();
    let mut path = Vec::new();
    let mut consumed = 0usize;

    for (i, ev) in transcript.events.iter().enumerate() {
        consumed = i + 1;
        let candidates = spec.matching(&state, &ev.role, &ev.msg);

        if candidates.is_empty() {
            // Distinguish "wrong role" from "wrong message entirely".
            let msg_exists_for_other_role = spec
                .transitions
                .iter()
                .any(|t| t.from == state && t.msg == ev.msg && t.role != ev.role);
            let kind = if msg_exists_for_other_role {
                ViolationKind::RoleMismatch
            } else {
                ViolationKind::UnexpectedMessage
            };
            let expected = expected_here(spec, &state);
            violations.push(Violation {
                kind,
                event_index: i,
                event_id: Some(ev.id.clone()),
                state: state.clone(),
                detail: format!(
                    "got {}:{} in state `{}`; expected one of [{}]",
                    ev.role,
                    ev.msg,
                    state,
                    expected.join(", ")
                ),
            });
            continue; // fail-soft: do not advance
        }

        // Deterministically choose the first matching transition.
        let t = candidates[0];

        // Nonce invariant.
        if t.requires_fresh_nonce {
            match ev.nonce {
                None => violations.push(Violation {
                    kind: ViolationKind::NonceReplay,
                    event_index: i,
                    event_id: Some(ev.id.clone()),
                    state: state.clone(),
                    detail: format!(
                        "transition {}:{} requires a nonce but none present",
                        ev.role, ev.msg
                    ),
                }),
                Some(n) if seen_nonces.contains(&n) => violations.push(Violation {
                    kind: ViolationKind::NonceReplay,
                    event_index: i,
                    event_id: Some(ev.id.clone()),
                    state: state.clone(),
                    detail: format!("nonce {n} replayed (already observed)"),
                }),
                Some(n) => {
                    seen_nonces.insert(n);
                }
            }
        } else if let Some(n) = ev.nonce {
            // Track nonces even when not required, so a later required event can
            // detect a replay of a value used earlier.
            seen_nonces.insert(n);
        }

        // Sequence invariant.
        if t.requires_seq {
            let prev = last_seq.get(&ev.role).copied();
            match ev.seq {
                None => violations.push(Violation {
                    kind: ViolationKind::SequenceViolation,
                    event_index: i,
                    event_id: Some(ev.id.clone()),
                    state: state.clone(),
                    detail: format!(
                        "transition {}:{} requires seq but none present",
                        ev.role, ev.msg
                    ),
                }),
                Some(s) => {
                    let expected = prev.map(|p| p + 1).unwrap_or(0);
                    if s != expected {
                        violations.push(Violation {
                            kind: ViolationKind::SequenceViolation,
                            event_index: i,
                            event_id: Some(ev.id.clone()),
                            state: state.clone(),
                            detail: format!(
                                "role `{}` seq={} but expected {}",
                                ev.role, s, expected
                            ),
                        });
                    }
                    last_seq.insert(ev.role.clone(), s);
                }
            }
        } else if let Some(s) = ev.seq {
            last_seq.insert(ev.role.clone(), s);
        }

        // Advance the state machine regardless of nonce/seq problems: those are
        // metadata invariants layered on top of a structurally valid step.
        path.push(PathStep {
            event_index: i,
            event_label: ev.label(),
            from: state.clone(),
            to: t.to.clone(),
        });
        state = t.to.clone();
    }

    let reached_accepting = spec.is_accepting(&state);
    if !reached_accepting {
        violations.push(Violation {
            kind: ViolationKind::NotAccepting,
            event_index: transcript.events.len(),
            event_id: None,
            state: state.clone(),
            detail: format!(
                "final state `{}` is not accepting (accepting: [{}])",
                state,
                spec.accepting.join(", ")
            ),
        });
    }

    ReplayResult {
        protocol: spec.name.clone(),
        final_state: state,
        reached_accepting,
        violations,
        path,
        consumed,
    }
}

/// The set of `role:msg` labels accepted from a given state.
fn expected_here(spec: &ProtocolSpec, state: &str) -> Vec<String> {
    let mut v: Vec<String> = spec
        .transitions
        .iter()
        .filter(|t| t.from == state)
        .map(|t| format!("{}:{}", t.role, t.msg))
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Convenience: does replaying this transcript conform to the spec?
pub fn conforms(spec: &ProtocolSpec, transcript: &Transcript) -> bool {
    replay(spec, transcript).is_conforming()
}

/// Build a transcript from an event slice for the given spec.
pub fn transcript_from(protocol: &str, events: &[Event]) -> Transcript {
    Transcript {
        protocol: protocol.to_string(),
        events: events.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Event, Transition};

    fn spec() -> ProtocolSpec {
        ProtocolSpec {
            name: "p".into(),
            roles: vec!["i".into(), "r".into()],
            initial: "s0".into(),
            accepting: vec!["s2".into()],
            states: vec!["s0".into(), "s1".into(), "s2".into()],
            transitions: vec![
                Transition {
                    from: "s0".into(),
                    to: "s1".into(),
                    role: "i".into(),
                    msg: "e".into(),
                    requires_fresh_nonce: true,
                    requires_seq: false,
                    note: None,
                },
                Transition {
                    from: "s1".into(),
                    to: "s2".into(),
                    role: "r".into(),
                    msg: "ee".into(),
                    requires_fresh_nonce: false,
