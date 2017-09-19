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
