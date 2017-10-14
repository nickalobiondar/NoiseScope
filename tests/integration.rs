//! End-to-end integration tests exercising the bundled fixtures.
//!
//! These verify that the shipped Noise/TLS/MLS-inspired abstract fixtures parse,
//! lint clean, and produce the divergence outcomes documented in the README.

use std::fs;
use std::path::PathBuf;

use noisescope::divergence::{fuzz, report_json, FuzzConfig};
use noisescope::engine::{replay, ViolationKind};
use noisescope::mutate::apply_all;
use noisescope::parser::{parse_protocol, parse_transcript};

fn fixture(name: &str) -> String {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("fixtures");
    p.push(name);
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("cannot read fixture {name}: {e}"))
}

fn spec(name: &str) -> noisescope::model::ProtocolSpec {
    parse_protocol(&fixture(name)).expect("spec parses")
}

fn transcript(name: &str) -> noisescope::model::Transcript {
    parse_transcript(&fixture(name)).expect("transcript parses")
}

#[test]
fn all_specs_lint_clean() {
    for s in ["noise-xx", "tls13", "mls"] {
        let spec = spec(&format!("{s}.protocol.json"));
        let problems = spec.lint();
        assert!(problems.is_empty(), "{s} lint problems: {problems:?}");
    }
}

#[test]
fn ok_transcripts_conform() {
    for s in ["noise-xx", "tls13", "mls"] {
        let sp = spec(&format!("{s}.protocol.json"));
        let tr = transcript(&format!("{s}.ok.transcript.json"));
        let r = replay(&sp, &tr);
        assert!(
            r.is_conforming(),
            "{s} ok transcript did not conform: {:?}",
            r.violations
        );
    }
}

#[test]
fn noise_replay_flags_nonce() {
    let sp = spec("noise-xx.protocol.json");
    let tr = transcript("noise-xx.replay.transcript.json");
    let r = replay(&sp, &tr);
    assert!(r
        .violations
