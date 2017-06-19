//! `noisescope` command-line interface.
//!
//! Subcommands:
//!   lint    <spec.json>                      — check a spec for internal consistency
//!   check   <spec.json> <transcript.json>    — replay a transcript, report divergence
//!   fuzz    <spec.json> <transcript.json>    — mutate & minimize failing sequences
//!   paths   <spec.json> <transcript.json>    — emit the handshake path (for the viewer)
//!   version                                  — print version
//!   help                                     — this message
//!
//! Global flags:
//!   --format json|text   (default: text; `paths` always emits JSON)
//!   --seed <u64>         (fuzz only; default 0x5EED)
//!   --trials <n>         (fuzz only; default 256)
//!   --max-plan-len <n>   (fuzz only; default 4)
//!   --max-findings <n>   (fuzz only; default 8; 0 = unlimited within trials)
//!   --out <path>         write output to a file instead of stdout
//!
//! Exit codes: 0 = success/conforming, 1 = divergence/violations found,
//! 2 = usage or I/O error. This makes `noisescope` scriptable in CI.

use std::process::ExitCode;

use noisescope::divergence::{check_json, fuzz, replay_text, report_json, report_text, FuzzConfig};
use noisescope::engine::replay;
use noisescope::json::Json;
use noisescope::parser::{parse_protocol, parse_transcript};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Format {
    Json,
    Text,
}

struct Options {
    format: Format,
    seed: u64,
    trials: usize,
    max_plan_len: usize,
    max_findings: usize,
    out: Option<String>,
    positional: Vec<String>,
}

fn parse_options(args: &[String]) -> Result<Options, String> {
    let mut opts = Options {
        format: Format::Text,
        seed: 0x5EED,
        trials: 256,
        max_plan_len: 4,
        max_findings: 8,
        out: None,
        positional: Vec::new(),
    };
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "--format" => {
                i += 1;
