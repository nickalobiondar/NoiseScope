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
                out.events.insert(index + 1, copy);
            }
        }
        Mutation::Reorder { a, b } => {
            if a < n && b < n && a != b {
                out.events.swap(a, b);
            }
        }
        Mutation::CorruptMeta { index, field } => {
            if index < out.events.len() {
                let ev = &mut out.events[index];
                match field {
                    MetaField::Nonce => {
                        // Reuse a fixed value to force potential replay/absence.
                        ev.nonce = match ev.nonce {
                            Some(0) => None,
                            _ => Some(0),
                        };
                    }
                    MetaField::Seq => {
                        ev.seq = Some(ev.seq.map(|s| s.wrapping_add(7)).unwrap_or(99));
                    }
                    MetaField::Role => {
                        if let Some(other) = roles.iter().find(|r| *r != &ev.role) {
                            ev.role = other.clone();
                        }
                    }
                }
            }
        }
    }
    out
}

/// Apply a sequence of mutations in order.
pub fn apply_all(transcript: &Transcript, plan: &[Mutation], roles: &[String]) -> Transcript {
    let mut cur = transcript.clone();
    for m in plan {
        cur = apply_one(&cur, m, roles);
    }
    cur
}

/// Deterministically generate a mutation plan of `len` edits for a transcript.
///
/// The generator only ever references indices valid for the *original*
/// transcript length; [`apply_one`] tolerates drift caused by earlier drops.
pub fn generate_plan(rng: &mut SplitMix64, transcript_len: usize, len: usize) -> Vec<Mutation> {
    let mut plan = Vec::with_capacity(len);
    if transcript_len == 0 {
        return plan;
    }
    for _ in 0..len {
        let choice = rng.below(4);
        let m = match choice {
            0 => Mutation::Drop {
                index: rng.below(transcript_len),
            },
            1 => Mutation::Duplicate {
                index: rng.below(transcript_len),
            },
            2 => {
                let a = rng.below(transcript_len);
                let mut b = rng.below(transcript_len);
                if a == b {
                    b = (b + 1) % transcript_len;
                }
                Mutation::Reorder { a, b }
            }
            _ => {
                let field = match rng.below(3) {
                    0 => MetaField::Nonce,
                    1 => MetaField::Seq,
                    _ => MetaField::Role,
                };
                Mutation::CorruptMeta {
                    index: rng.below(transcript_len),
                    field,
                }
            }
        };
        plan.push(m);
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Event;

    fn t3() -> Transcript {
        let mut t = Transcript::new("p");
        for i in 0..3 {
            let mut e = Event::new(format!("e{i}"), "i", "m");
            e.nonce = Some(i as u64);
            e.seq = Some(i as u64);
            t.events.push(e);
        }
        t
    }

    #[test]
    fn splitmix_is_deterministic() {
        let mut a = SplitMix64::new(42);
        let mut b = SplitMix64::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn drop_removes_event() {
        let out = apply_one(&t3(), &Mutation::Drop { index: 1 }, &[]);
        assert_eq!(out.events.len(), 2);
        assert_eq!(out.events[1].id, "e2");
    }

    #[test]
    fn duplicate_inserts_copy() {
        let out = apply_one(&t3(), &Mutation::Duplicate { index: 0 }, &[]);
        assert_eq!(out.events.len(), 4);
        assert_eq!(out.events[1].id, "e0~dup");
    }

    #[test]
    fn reorder_swaps() {
        let out = apply_one(&t3(), &Mutation::Reorder { a: 0, b: 2 }, &[]);
        assert_eq!(out.events[0].id, "e2");
        assert_eq!(out.events[2].id, "e0");
    }

    #[test]
    fn corrupt_role_uses_other_role() {
        let roles = vec!["i".to_string(), "r".to_string()];
        let out = apply_one(
            &t3(),
            &Mutation::CorruptMeta {
                index: 0,
                field: MetaField::Role,
            },
            &roles,
        );
        assert_eq!(out.events[0].role, "r");
    }

    #[test]
    fn out_of_range_is_noop() {
        let out = apply_one(&t3(), &Mutation::Drop { index: 99 }, &[]);
        assert_eq!(out.events.len(), 3);
    }

    #[test]
    fn plan_is_reproducible() {
        let p1 = generate_plan(&mut SplitMix64::new(7), 3, 10);
        let p2 = generate_plan(&mut SplitMix64::new(7), 3, 10);
        assert_eq!(p1, p2);
    }
}

// draft note 85
