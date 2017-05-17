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
        .set("path", Json::Arr(path))
        .build()
}

fn finding_json(f: &Finding) -> Json {
    ObjBuilder::new()
        .set("seed", Json::Num(f.seed as f64))
        .set("original_plan", plan_json(&f.plan))
        .set("minimized_plan", plan_json(&f.minimized.plan))
        .set("minimized_len", Json::Num(f.minimized.plan.len() as f64))
        .set("original_len", Json::Num(f.minimized.original_len as f64))
        .set(
            "minimizer_evaluations",
            Json::Num(f.minimized.evaluations as f64),
        )
        .set("divergence", replay_json(&f.result))
        .build()
}

/// Serialize a full fuzz report as a [`Json`] value.
pub fn report_json(report: &FuzzReport) -> Json {
    ObjBuilder::new()
        .set("tool", Json::Str("noisescope".to_string()))
        .set(
            "disclaimer",
            Json::Str("structural handshake analysis only; not a cryptographic proof".to_string()),
        )
        .set("protocol", Json::Str(report.protocol.clone()))
        .set(
            "baseline_conforming",
            Json::Bool(report.baseline_conforming),
        )
        .set("baseline", replay_json(&report.baseline))
        .set("trials", Json::Num(report.trials as f64))
        .set("findings_count", Json::Num(report.findings.len() as f64))
        .set(
            "findings",
            Json::Arr(report.findings.iter().map(finding_json).collect()),
        )
        .build()
}

/// Serialize a single replay result as JSON (used by the `check` command).
pub fn check_json(r: &ReplayResult) -> Json {
    ObjBuilder::new()
        .set("tool", Json::Str("noisescope".to_string()))
        .set(
            "disclaimer",
            Json::Str("structural handshake analysis only; not a cryptographic proof".to_string()),
        )
        .set("protocol", Json::Str(r.protocol.clone()))
        .set("result", replay_json(r))
        .build()
}

// ---------------------------------------------------------------------------
// Text rendering
// ---------------------------------------------------------------------------

/// Render a replay result as compact human text.
pub fn replay_text(r: &ReplayResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("protocol: {}\n", r.protocol));
    out.push_str(&format!(
        "final state: {} ({})\n",
        r.final_state,
        if r.reached_accepting {
            "accepting"
        } else {
            "NOT accepting"
        }
    ));
    out.push_str(&format!(
        "conforming: {}\n",
        if r.is_conforming() { "yes" } else { "no" }
    ));
    if r.path.is_empty() {
        out.push_str("path: (empty)\n");
    } else {
        out.push_str("path:\n");
        for s in &r.path {
            out.push_str(&format!("  {} --[{}]--> {}\n", s.from, s.event_label, s.to));
        }
    }
    if r.violations.is_empty() {
        out.push_str("violations: none\n");
    } else {
        out.push_str(&format!("violations ({}):\n", r.violations.len()));
        for v in &r.violations {
            out.push_str(&format!(
                "  [{}] @event {} state={}: {}\n",
                v.kind.as_str(),
                v.event_index,
                v.state,
                v.detail
            ));
        }
    }
    out
}

/// Render a full fuzz report as human text.
pub fn report_text(report: &FuzzReport) -> String {
    let mut out = String::new();
    out.push_str("noisescope — divergence report\n");
    out.push_str("(structural analysis only; not a cryptographic proof)\n");
    out.push_str(&format!("protocol: {}\n", report.protocol));
    out.push_str(&format!(
        "baseline conforming: {}\n",
        if report.baseline_conforming {
            "yes"
        } else {
            "no"
        }
    ));
    out.push_str(&format!("trials: {}\n", report.trials));
    out.push_str(&format!("findings: {}\n", report.findings.len()));
    for (i, f) in report.findings.iter().enumerate() {
        out.push_str(&format!("\n--- finding #{i} (seed {:#x}) ---\n", f.seed));
        out.push_str(&format!(
            "minimized plan ({} of {} mutations, {} evals):\n",
            f.minimized.plan.len(),
            f.minimized.original_len,
            f.minimized.evaluations
        ));
        for m in &f.minimized.plan {
            out.push_str(&format!("  - {}\n", m.describe()));
        }
        out.push_str("divergence:\n");
        for line in replay_text(&f.result).lines() {
            out.push_str(&format!("  {line}\n"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Event, ProtocolSpec, Transition};

    fn spec() -> ProtocolSpec {
        ProtocolSpec {
            name: "p".into(),
            roles: vec!["i".into(), "r".into()],
            initial: "s0".into(),
            accepting: vec!["s2".into()],
            states: vec!["s0".into(), "s1".into(), "s2".into()],
            transitions: vec![
                Transition {
                    from: "s0".into(),
                    to: "s1".into(),
                    role: "i".into(),
                    msg: "e".into(),
                    requires_fresh_nonce: true,
                    requires_seq: false,
                    note: None,
                },
                Transition {
                    from: "s1".into(),
                    to: "s2".into(),
                    role: "r".into(),
                    msg: "ee".into(),
                    requires_fresh_nonce: false,
                    requires_seq: true,
                    note: None,
                },
            ],
            description: None,
        }
    }

    fn good_transcript() -> Transcript {
        let mut t = Transcript::new("p");
        let mut a = Event::new("a", "i", "e");
        a.nonce = Some(1);
        let mut b = Event::new("b", "r", "ee");
        b.seq = Some(0);
        t.events.push(a);
        t.events.push(b);
        t
    }
