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
