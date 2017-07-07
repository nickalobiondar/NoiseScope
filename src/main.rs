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
                let v = args.get(i).ok_or("--format requires a value")?;
                opts.format = match v.as_str() {
                    "json" => Format::Json,
                    "text" => Format::Text,
                    other => return Err(format!("unknown format `{other}`")),
                };
            }
            "--seed" => {
                i += 1;
                let v = args.get(i).ok_or("--seed requires a value")?;
                opts.seed = parse_u64(v)?;
            }
            "--trials" => {
                i += 1;
                let v = args.get(i).ok_or("--trials requires a value")?;
                opts.trials = parse_usize(v)?;
            }
            "--max-plan-len" => {
                i += 1;
                let v = args.get(i).ok_or("--max-plan-len requires a value")?;
                opts.max_plan_len = parse_usize(v)?.max(1);
            }
            "--max-findings" => {
                i += 1;
                let v = args.get(i).ok_or("--max-findings requires a value")?;
                opts.max_findings = parse_usize(v)?;
            }
            "--out" => {
                i += 1;
                let v = args.get(i).ok_or("--out requires a value")?;
                opts.out = Some(v.clone());
            }
            other if other.starts_with("--") => {
                return Err(format!("unknown flag `{other}`"));
            }
            _ => opts.positional.push(a.clone()),
        }
        i += 1;
    }
    Ok(opts)
}

fn parse_u64(s: &str) -> Result<u64, String> {
    let s = s.trim();
    let parsed = if let Some(hex) = s.strip_prefix("0x") {
        u64::from_str_radix(hex, 16)
    } else {
        s.parse::<u64>()
    };
    parsed.map_err(|_| format!("invalid unsigned integer `{s}`"))
}

fn parse_usize(s: &str) -> Result<usize, String> {
    s.trim()
        .parse::<usize>()
        .map_err(|_| format!("invalid count `{s}`"))
}

fn read_file(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("cannot read `{path}`: {e}"))
}

fn emit(opts: &Options, s: &str) -> Result<(), String> {
    match &opts.out {
        Some(path) => std::fs::write(path, s).map_err(|e| format!("cannot write `{path}`: {e}")),
        None => {
            print!("{s}");
            if !s.ends_with('\n') {
                println!();
            }
            Ok(())
        }
    }
}

const HELP: &str = "\
noisescope — cryptographic handshake transcript state-machine analyzer/fuzzer
(structural analysis only; NOT a cryptographic proof tool)

USAGE:
  noisescope <command> [args] [flags]

COMMANDS:
  lint    <spec.json>                     check a spec for internal consistency
  check   <spec.json> <transcript.json>   replay a transcript, report divergence
  fuzz    <spec.json> <transcript.json>   mutate & minimize failing sequences
  paths   <spec.json> <transcript.json>   emit handshake path JSON (for the viewer)
  version                                 print version
  help                                    show this message

FLAGS:
  --format json|text    output format (default: text; `paths` is always JSON)
  --seed <u64>          fuzz PRNG seed (default: 0x5EED)
  --trials <n>          fuzz trial budget (default: 256)
  --max-plan-len <n>    max mutations per plan (default: 4)
  --max-findings <n>    stop after n findings (default: 8; 0 = unlimited)
  --out <path>          write output to a file instead of stdout

EXIT CODES:
  0  success / conforming
  1  divergence or violations found
  2  usage or I/O error
";

fn run() -> Result<i32, String> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if raw.is_empty() {
        print!("{HELP}");
        return Ok(2);
    }
    let command = raw[0].clone();
    let opts = parse_options(&raw[1..])?;

    match command.as_str() {
        "help" | "-h" | "--help" => {
            print!("{HELP}");
            Ok(0)
        }
        "version" | "-V" | "--version" => {
            println!("noisescope {}", noisescope::VERSION);
            Ok(0)
        }
        "lint" => {
            let path = opts.positional.first().ok_or("lint requires <spec.json>")?;
            let spec = parse_protocol(&read_file(path)?).map_err(|e| e.to_string())?;
            let problems = spec.lint();
            if opts.format == Format::Json {
                let arr = Json::Arr(problems.iter().cloned().map(Json::Str).collect());
                let obj = noisescope::json::ObjBuilder::new()
                    .set("spec", Json::Str(spec.name.clone()))
                    .set("ok", Json::Bool(problems.is_empty()))
                    .set("problems", arr)
                    .build();
                emit(&opts, &obj.to_pretty())?;
            } else if problems.is_empty() {
                emit(
                    &opts,
                    &format!("spec `{}` is internally consistent\n", spec.name),
                )?;
            } else {
                let mut s = format!("spec `{}` has {} problem(s):\n", spec.name, problems.len());
                for p in &problems {
                    s.push_str(&format!("  - {p}\n"));
                }
                emit(&opts, &s)?;
            }
            Ok(if problems.is_empty() { 0 } else { 1 })
        }
        "check" => {
            let (spec, transcript) = load_pair(&opts)?;
            let result = replay(&spec, &transcript);
            if opts.format == Format::Json {
                emit(&opts, &check_json(&result).to_pretty())?;
            } else {
                emit(&opts, &replay_text(&result))?;
            }
            Ok(if result.is_conforming() { 0 } else { 1 })
        }
        "fuzz" => {
            let (spec, transcript) = load_pair(&opts)?;
            let cfg = FuzzConfig {
                seed: opts.seed,
                trials: opts.trials,
                max_plan_len: opts.max_plan_len,
                max_findings: opts.max_findings,
            };
            let report = fuzz(&spec, &transcript, &cfg);
            if opts.format == Format::Json {
                emit(&opts, &report_json(&report).to_pretty())?;
            } else {
                emit(&opts, &report_text(&report))?;
            }
            Ok(if report.findings.is_empty() { 0 } else { 1 })
        }
        "paths" => {
            let (spec, transcript) = load_pair(&opts)?;
            let result = replay(&spec, &transcript);
            // `paths` emits a viewer-friendly JSON document (always JSON).
            let steps: Vec<Json> = result
                .path
                .iter()
                .map(|s| {
                    noisescope::json::ObjBuilder::new()
                        .set("event_index", Json::Num(s.event_index as f64))
                        .set("event", Json::Str(s.event_label.clone()))
                        .set("from", Json::Str(s.from.clone()))
