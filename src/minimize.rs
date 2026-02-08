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
    plan: &[Mutation],
    fails: &FailPredicate<'_>,
) -> Minimized {
    let original_len = plan.len();
    let mut evaluations = 0usize;

    // Guard: only minimize genuine failures.
    let full = apply_all(base, plan, roles);
    evaluations += 1;
    if !fails(&full) {
        return Minimized {
            plan: plan.to_vec(),
            evaluations,
            original_len,
        };
    }

    let mut current: Vec<Mutation> = plan.to_vec();
    let mut changed = true;
    while changed {
        changed = false;
        let mut i = 0;
        while i < current.len() {
            let mut candidate = current.clone();
            candidate.remove(i);
            let mutated = apply_all(base, &candidate, roles);
            evaluations += 1;
            if fails(&mutated) {
                // Removal preserved the failure: keep the shorter plan.
                current = candidate;
                changed = true;
                // Do not advance `i`; the next element shifted into this slot.
            } else {
                i += 1;
            }
        }
    }

    Minimized {
        plan: current,
        evaluations,
        original_len,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Event, Transcript};
    use crate::mutate::Mutation;

    fn base() -> Transcript {
        let mut t = Transcript::new("p");
        for i in 0..5 {
            t.events.push(Event::new(format!("e{i}"), "i", "m"));
        }
        t
    }

    #[test]
    fn removes_irrelevant_mutations() {
        // The transcript "fails" iff it has fewer than 5 events.
        let fails = |t: &Transcript| t.events.len() < 5;
        // Plan: one meaningful drop + several no-op reorders that don't shrink.
        let plan = vec![
            Mutation::Reorder { a: 0, b: 1 },
            Mutation::Drop { index: 2 },
            Mutation::Reorder { a: 1, b: 3 },
        ];
        let m = minimize(&base(), &[], &plan, &fails);
        assert_eq!(m.plan.len(), 1);
        assert!(matches!(m.plan[0], Mutation::Drop { .. }));
        assert_eq!(m.original_len, 3);
    }

    #[test]
    fn non_failing_plan_unchanged() {
        let fails = |t: &Transcript| t.events.len() < 5;
        let plan = vec![Mutation::Reorder { a: 0, b: 1 }];
        let m = minimize(&base(), &[], &plan, &fails);
        assert_eq!(m.plan.len(), 1);
        assert_eq!(m.evaluations, 1);
    }

    #[test]
    fn keeps_all_when_all_needed() {
        // Fails only if at least two events dropped (len < 4).
        let fails = |t: &Transcript| t.events.len() < 4;
        let plan = vec![Mutation::Drop { index: 0 }, Mutation::Drop { index: 0 }];
        let m = minimize(&base(), &[], &plan, &fails);
        assert_eq!(m.plan.len(), 2);
    }
}
