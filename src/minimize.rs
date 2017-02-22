//! Minimization of failing mutation plans (delta-debugging style).
//!
//! When the fuzzer finds a mutation plan that makes a previously-conforming
//! transcript diverge, the raw plan is usually longer than necessary. This
//! module shrinks it to a *1-minimal* subsequence: a plan from which no single
//! mutation can be removed without the transcript ceasing to fail.
//!
//! The algorithm is the classic greedy pass used by delta-debugging's final
//! stage: repeatedly try removing each element; keep the removal if the reduced
//! plan still fails the predicate. It runs to a fixed point. This is O(n^2)
//! predicate evaluations in the worst case but plans are short, and each
//! evaluation is a deterministic replay.

use crate::model::Transcript;
use crate::mutate::{apply_all, Mutation};

/// A predicate that returns `true` when a transcript is considered *failing*
/// (i.e. it diverges from the spec in the way we are trying to preserve).
pub type FailPredicate<'a> = dyn Fn(&Transcript) -> bool + 'a;

/// Result of a minimization pass.
#[derive(Debug, Clone)]
pub struct Minimized {
    pub plan: Vec<Mutation>,
    /// Number of predicate evaluations performed (useful for reports/tests).
    pub evaluations: usize,
    pub original_len: usize,
}

/// Minimize `plan` so that applying it to `base` still satisfies `fails`.
///
/// Requires that the *full* plan already fails; if it does not, the original
/// plan is returned unchanged (with `evaluations == 0`).
pub fn minimize(
    base: &Transcript,
    roles: &[String],
