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
