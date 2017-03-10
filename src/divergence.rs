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
