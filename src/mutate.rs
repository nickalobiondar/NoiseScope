//! Deterministic transcript mutations.
//!
//! A *mutation* is a single, reversible edit applied to a transcript's event
//! list. `noisescope` uses four families, each chosen because it mirrors a
//! realistic handshake failure mode:
//!
//!  * [`Mutation::Drop`] — a message is lost in flight.
//!  * [`Mutation::Duplicate`] — a message is replayed / delivered twice.
//!  * [`Mutation::Reorder`] — two messages arrive out of order.
//!  * [`Mutation::CorruptMeta`] — a metadata field (nonce/seq/meta) is altered.
//!
//! All mutations are *deterministic*: given the same seed and transcript length
//! the generated mutation plan is identical, which makes fuzzing runs and their
//! minimized counter-examples fully reproducible.
//!
//! The PRNG is a small SplitMix64. It is **not** cryptographically secure and is
//! never used for anything security-sensitive — only to pick which
//! deterministic edit to apply next.

use crate::model::Transcript;

/// SplitMix64 — a tiny, well-distributed, fully deterministic PRNG.
///
/// Reference: Steele, Lea & Flood, "Fast splittable pseudorandom number
/// generators" (2014). Used here purely to schedule mutations reproducibly.
#[derive(Debug, Clone)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        SplitMix64 { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform-ish integer in `[0, n)`. Returns 0 when `n == 0`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % (n as u64)) as usize
        }
    }
}

/// A single deterministic edit to a transcript.
#[derive(Debug, Clone, PartialEq)]
pub enum Mutation {
    /// Remove the event at `index`.
    Drop { index: usize },
    /// Insert a copy of the event at `index` immediately after it.
    Duplicate { index: usize },
    /// Swap the events at `a` and `b`.
    Reorder { a: usize, b: usize },
    /// Corrupt a metadata field of the event at `index`.
    CorruptMeta { index: usize, field: MetaField },
}

/// Which metadata field a [`Mutation::CorruptMeta`] targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaField {
    /// Zero-out or reuse the nonce.
    Nonce,
    /// Perturb the sequence number.
    Seq,
    /// Flip the role to a different declared role.
    Role,
}

impl Mutation {
    /// A stable human/JSON label for reports.
    pub fn kind(&self) -> &'static str {
        match self {
            Mutation::Drop { .. } => "drop",
            Mutation::Duplicate { .. } => "duplicate",
            Mutation::Reorder { .. } => "reorder",
            Mutation::CorruptMeta { .. } => "corrupt_meta",
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Mutation::Drop { index } => format!("drop event #{index}"),
            Mutation::Duplicate { index } => format!("duplicate event #{index}"),
            Mutation::Reorder { a, b } => format!("reorder events #{a} <-> #{b}"),
            Mutation::CorruptMeta { index, field } => {
                let f = match field {
                    MetaField::Nonce => "nonce",
                    MetaField::Seq => "seq",
                    MetaField::Role => "role",
                };
                format!("corrupt {f} of event #{index}")
            }
        }
    }
}

/// Apply a single mutation to a transcript, returning the mutated copy.
///
/// Mutations that reference an out-of-range index are treated as no-ops so that
/// a mutation plan generated for one transcript length stays well-defined after
/// earlier drops shrink the list. This keeps minimization total and panic-free.
pub fn apply_one(transcript: &Transcript, m: &Mutation, roles: &[String]) -> Transcript {
    let mut out = transcript.clone();
    let n = out.events.len();
    match *m {
        Mutation::Drop { index } => {
            if index < out.events.len() {
                out.events.remove(index);
            }
        }
        Mutation::Duplicate { index } => {
            if index < out.events.len() {
                let mut copy = out.events[index].clone();
                copy.id = format!("{}~dup", copy.id);
