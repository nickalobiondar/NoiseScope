//! Divergence reports and fuzzing orchestration.
//!
//! A *divergence* is the difference in outcome between replaying the original
//! transcript and replaying a mutated one. The fuzzer:
//!
//!  1. confirms the baseline transcript conforms (a sensible starting point),
//!  2. generates deterministic mutation plans of increasing length,
//!  3. replays each mutated transcript,
//!  4. records the first plans that induce divergence,
//!  5. minimizes each failing plan to a 1-minimal counter-example.
//!
//! Reports serialize to both JSON (stable, machine-readable) and a compact text
//! form for terminals. Everything is deterministic given `(seed, spec,
//! transcript, budget)`.

use crate::engine::{replay, ReplayResult, Violation};
use crate::json::{Json, ObjBuilder};
use crate::minimize::{minimize, Minimized};
use crate::model::{ProtocolSpec, Transcript};
use crate::mutate::{apply_all, generate_plan, Mutation, SplitMix64};

/// A single divergence-inducing (and minimized) finding.
#[derive(Debug, Clone)]
pub struct Finding {
    pub seed: u64,
    pub plan: Vec<Mutation>,
    pub minimized: Minimized,
    pub result: ReplayResult,
}

/// The full fuzzing report.
#[derive(Debug, Clone)]
pub struct FuzzReport {
    pub protocol: String,
    pub baseline_conforming: bool,
    pub baseline: ReplayResult,
    pub trials: usize,
    pub findings: Vec<Finding>,
}

/// Parameters controlling a fuzzing run.
#[derive(Debug, Clone)]
pub struct FuzzConfig {
    pub seed: u64,
    pub trials: usize,
    pub max_plan_len: usize,
    /// Stop after this many distinct findings (0 = unlimited within trials).
    pub max_findings: usize,
}

impl Default for FuzzConfig {
    fn default() -> Self {
        FuzzConfig {
            seed: 0x5EED,
            trials: 256,
            max_plan_len: 4,
            max_findings: 8,
        }
    }
}

/// Run the fuzzer. The failure predicate is "does not conform" — that is, the
/// mutated transcript either violates an invariant or fails to reach an
/// accepting state.
pub fn fuzz(spec: &ProtocolSpec, transcript: &Transcript, cfg: &FuzzConfig) -> FuzzReport {
    let baseline = replay(spec, transcript);
    let mut findings: Vec<Finding> = Vec::new();

    let roles = spec.roles.clone();
    let fails = |t: &Transcript| !replay(spec, t).is_conforming();

    let mut rng = SplitMix64::new(cfg.seed);
    let mut seen_minimal: Vec<Vec<Mutation>> = Vec::new();

    for trial in 0..cfg.trials {
        if cfg.max_findings != 0 && findings.len() >= cfg.max_findings {
            break;
        }
        // Vary plan length deterministically between 1..=max_plan_len.
        let plan_len = 1 + (trial % cfg.max_plan_len.max(1));
        let trial_seed = cfg.seed ^ ((trial as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let mut trial_rng = SplitMix64::new(trial_seed);
        let plan = generate_plan(&mut trial_rng, transcript.events.len(), plan_len);
        // Consume the top-level rng too so runs with different trial counts stay
        // related but distinct.
        let _ = rng.next_u64();

        if plan.is_empty() {
            continue;
        }
        let mutated = apply_all(transcript, &plan, &roles);
        let result = replay(spec, &mutated);
        if result.is_conforming() {
            continue; // no divergence
        }

        let minimized = minimize(transcript, &roles, &plan, &fails);
        // Deduplicate by minimized plan so the report stays crisp.
        if seen_minimal.iter().any(|p| p == &minimized.plan) {
            continue;
        }
        seen_minimal.push(minimized.plan.clone());

        let min_mutated = apply_all(transcript, &minimized.plan, &roles);
        let min_result = replay(spec, &min_mutated);

        findings.push(Finding {
            seed: trial_seed,
            plan,
            minimized,
            result: min_result,
        });
    }

    FuzzReport {
        protocol: spec.name.clone(),
        baseline_conforming: baseline.is_conforming(),
        baseline: baseline.clone(),
        trials: cfg.trials,
        findings,
    }
}

// ---------------------------------------------------------------------------
// JSON serialization
// ---------------------------------------------------------------------------

fn violation_json(v: &Violation) -> Json {
    ObjBuilder::new()
        .set("kind", Json::Str(v.kind.as_str().to_string()))
        .set("event_index", Json::Num(v.event_index as f64))
        .set(
            "event_id",
            v.event_id.clone().map(Json::Str).unwrap_or(Json::Null),
        )
        .set("state", Json::Str(v.state.clone()))
        .set("detail", Json::Str(v.detail.clone()))
        .build()
}

fn mutation_json(m: &Mutation) -> Json {
    let b = ObjBuilder::new().set("kind", Json::Str(m.kind().to_string()));
    let b = match m {
        Mutation::Drop { index } => b.set("index", Json::Num(*index as f64)),
        Mutation::Duplicate { index } => b.set("index", Json::Num(*index as f64)),
        Mutation::Reorder { a, b: bb } => b
            .set("a", Json::Num(*a as f64))
            .set("b", Json::Num(*bb as f64)),
        Mutation::CorruptMeta { index, field } => {
            let f = match field {
                crate::mutate::MetaField::Nonce => "nonce",
                crate::mutate::MetaField::Seq => "seq",
                crate::mutate::MetaField::Role => "role",
            };
            b.set("index", Json::Num(*index as f64))
                .set("field", Json::Str(f.to_string()))
        }
    };
    b.set("describe", Json::Str(m.describe())).build()
}

fn plan_json(plan: &[Mutation]) -> Json {
    Json::Arr(plan.iter().map(mutation_json).collect())
}

fn replay_json(r: &ReplayResult) -> Json {
    let path: Vec<Json> = r
        .path
        .iter()
        .map(|s| {
            ObjBuilder::new()
                .set("event_index", Json::Num(s.event_index as f64))
                .set("event", Json::Str(s.event_label.clone()))
                .set("from", Json::Str(s.from.clone()))
                .set("to", Json::Str(s.to.clone()))
                .build()
        })
        .collect();
    ObjBuilder::new()
        .set("final_state", Json::Str(r.final_state.clone()))
        .set("reached_accepting", Json::Bool(r.reached_accepting))
        .set("conforming", Json::Bool(r.is_conforming()))
        .set("consumed", Json::Num(r.consumed as f64))
        .set(
            "violations",
            Json::Arr(r.violations.iter().map(violation_json).collect()),
        )
