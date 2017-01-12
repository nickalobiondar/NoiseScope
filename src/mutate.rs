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
