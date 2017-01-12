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
