# noisescope

<p align="center">
  <img src="docs/assets/noise-constellation.svg" alt="noisescope handshake constellation" width="720"/>
</p>

<p align="center"><em>A control instrument for handshake state machines.<br/>
It charts the constellation of states a transcript sweeps through, then flies deterministic mutations across that constellation until a trace diverges.</em></p>

<p align="center">
  <strong>Structural handshake transcript analyzer &amp; fuzzer</strong> ·
  Rust (std-only) core · TypeScript viewer ·
  <strong>NOT a cryptographic proof tool</strong>
</p>

---

## The instrument, in one glance

Point `noisescope` at two small JSON documents:

- a **protocol spec** — a finite state machine of handshake states and the
  transitions between them, and
- a **transcript** — an ordered list of observed handshake events.

It then does three things, each of them deterministic and dependency-free:

1. **Replays** the transcript against the machine and lights up any structural
   fault — a message out of order, the wrong role speaking, a replayed nonce, a
   sequence counter that skipped.
2. **Fuzzes** a conforming transcript by flying deterministic *mutations*
   (drop / duplicate / reorder / corrupt-metadata) across it until conformance
   breaks.
3. **Minimizes** every failing mutation sequence down to the smallest set of
   edits that still causes divergence — the counter-example, distilled.

Everything is reproducible from `(seed, spec, transcript, budget)`. The core has
**zero third-party dependencies**: it is the Rust standard library and nothing
else, including a small hand-written JSON reader/writer, so the whole engine is
auditable in an afternoon.

> ### What this is *not*
>
> `noisescope` is **not** a cryptographic proof tool. It never evaluates
> secrecy, authentication, forward secrecy, or any computational or symbolic
> security property. Its "nonces" and "sequence numbers" are ordinary integers
> used to check *shape*, not *soundness*. The bundled Noise/TLS/MLS-inspired
> fixtures are **abstract teaching models with no wire compatibility**.
> Conformance in noisescope implies **nothing** about the real protocols.

---

## Why a "constellation"?

A handshake is a path through a small sky of states. Draw the states as stars
and the legal transitions as the lines between them, and a transcript becomes a
single lit path across that constellation. Most analysis tools give you a log;
noisescope gives you the *chart* — and then it perturbs the chart to see which
small nudges send the path off course.

The second instrument panel is the **divergence scope**:

<p align="center">
  <img src="docs/assets/handshake-divergence.svg" alt="noisescope divergence instrument" width="720"/>
</p>

The green trace is a baseline that conforms. The amber trace is the same
transcript after a *minimized* mutation. Where they part company, the red marker
pulses — that split point is exactly what the minimizer hands you.

Both SVGs above are local, self-contained, and animated (SMIL). No external
assets, no runtime, no network.

---

## Quick start

```console
$ cargo build --release
$ cargo test            # 34 unit + 6 integration tests

# 1. Is my spec internally consistent?
$ cargo run -- lint fixtures/noise-xx.protocol.json
spec `noise-XX-abstract` is internally consistent

# 2. Does a transcript conform?
$ cargo run -- check fixtures/mls.protocol.json fixtures/mls.ok.transcript.json
```

That last command prints a real replay trace:

```text
protocol: mls-abstract
final state: welcomed (accepting)
conforming: yes
path:
  epoch_open --[proposer:Proposal]--> proposed
  proposed --[committer:Commit#7777]--> committed
  committed --[joiner:WelcomeAck]--> welcomed
violations: none
```

---

## Reading a divergence

Feed it a transcript where the server flight arrives *before* the client hello:

```console
$ cargo run -- check fixtures/tls13.protocol.json fixtures/tls13.reorder.transcript.json
```

```text
protocol: tls13-abstract
final state: hello_done (NOT accepting)
conforming: no
path:
  start --[client:ClientHello#500]--> hello_done
violations (3):
  [unexpected_message] @event 0 state=start: got server:ServerFlight in state `start`; expected one of [client:ClientHello]
  [unexpected_message] @event 2 state=hello_done: got client:ClientFinished in state `hello_done`; expected one of [server:ServerFlight]
  [not_accepting] @event 3 state=hello_done: final state `hello_done` is not accepting (accepting: [established])
```

Notice the engine is **fail-soft**: the out-of-order `ServerFlight` is reported
but does not derail evaluation of the rest of the transcript, so you see *every*
fault in one pass, not just the first. The exit code is `1`, so this is directly
usable as a CI gate.

---

## Flying mutations: the fuzzer

Start from a transcript that *does* conform and let noisescope hunt for the
smallest edits that break it:

```console
$ cargo run -- fuzz fixtures/noise-xx.protocol.json fixtures/noise-xx.ok.transcript.json \
      --seed 0x5EED --trials 200 --max-findings 1
```

```text
noisescope — divergence report
(structural analysis only; not a cryptographic proof)
protocol: noise-XX-abstract
baseline conforming: yes
trials: 200
findings: 1

--- finding #0 (seed 0x5eed) ---
minimized plan (1 of 1 mutations, 2 evals):
  - drop event #2
divergence:
  protocol: noise-XX-abstract
  final state: await_se (NOT accepting)
  conforming: no
  path:
    await_e --[initiator:e#1001]--> await_ee
    await_ee --[responder:e_ee_s_es#2002]--> await_se
  violations (1):
    [not_accepting] @event 2 state=await_se: final state `await_se` is not accepting (accepting: [established])
```

The finding is exact: dropping the third message strands the machine in
`await_se`, one step short of `established`. The minimizer confirms this is
**1-minimal** — remove that single mutation and the transcript conforms again.

Because the plan generator is a seeded SplitMix64 PRNG, re-running with the same
flags reproduces byte-for-byte identical JSON. That determinism is asserted by
the test suite (`fuzzing_is_deterministic_and_minimal`).

### JSON for machines

```console
$ cargo run -- fuzz fixtures/noise-xx.protocol.json fixtures/noise-xx.ok.transcript.json \
      --seed 0x5EED --trials 200 --max-findings 2 --format json
```

```json
{
  "tool": "noisescope",
  "disclaimer": "structural handshake analysis only; not a cryptographic proof",
  "protocol": "noise-XX-abstract",
  "baseline_conforming": true,
  "baseline": {
    "final_state": "established",
    "reached_accepting": true,
    "conforming": true,
    "consumed": 3,
    "violations": [],
    "path": [
      { "event_index": 0, "event": "initiator:e#1001", "from": "await_e", "to": "await_ee" }
    ]
  },
  "trials": 200,
  "findings_count": 2,
  "findings": [ /* each with original_plan, minimized_plan, minimizer_evaluations, divergence */ ]
}
```

Every report carries the `disclaimer` field in-band, so downstream tooling can
never mistake a structural report for a security verdict.

---

## The viewer: charting a path

`noisescope paths ...` emits a viewer-friendly JSON document; the bundled
TypeScript viewer turns it into an ASCII chart or a standalone animated SVG.

```console
$ cargo run -- paths fixtures/noise-xx.protocol.json fixtures/noise-xx.ok.transcript.json \
      | node viewer/dist/cli.js
noise-XX-abstract  [final: established ✓]
(await_e)
  └─ initiator:e#1001 ─▶ (await_ee)
  └─ responder:e_ee_s_es#2002 ─▶ (await_se)
  └─ initiator:s_se ─▶ ((established))
```

Double parentheses mark an accepting state. For a shareable diagram:

```console
$ cargo run -- paths fixtures/tls13.protocol.json fixtures/tls13.ok.transcript.json \
      | node viewer/dist/cli.js --svg > handshake.svg
```

The viewer is pure TypeScript, uses only Node's built-ins (`node:fs`,
`node:test`), and validates its input with a real type guard so malformed JSON
fails loudly rather than rendering garbage.

Build and test the viewer:

```console
$ cd viewer && npm install && npm run build && npm test
```

---

## The four invariants

For each event, the engine finds the transitions matching
`(current_state, role, msg)` and enforces:

| Invariant             | Fault kind            | Triggered when …                                            |
|-----------------------|-----------------------|-------------------------------------------------------------|
| **Order**             | `unexpected_message`  | no transition exists for that message from the current state |
| **Role**              | `role_mismatch`       | the message is valid here but for a *different* role         |
| **Nonce freshness**   | `nonce_replay`        | a required-fresh nonce is missing or already seen            |
| **Sequence**          | `sequence_violation`  | a required `seq` is missing or ≠ previous-for-role + 1        |
| **Termination**       | `not_accepting`       | the run ends outside the `accepting` set                     |

`seq` is a **per-role** counter that starts at `0` for each role's first
sequenced message. Full semantics live in [`docs/PROTOCOL.md`](docs/PROTOCOL.md).

---

## The mutation kit

| Mutation       | Real-world analogue                | Effect                                   |
|----------------|------------------------------------|------------------------------------------|
| `drop`         | message lost in flight             | removes one event                        |
| `duplicate`    | replay / double delivery           | inserts a copy right after an event      |
| `reorder`      | out-of-order network delivery      | swaps two events                         |
| `corrupt_meta` | tampered / stale metadata          | flips a `nonce`, `seq`, or `role` field  |

Mutation plans are scheduled by a seeded SplitMix64 PRNG. Out-of-range indices
(possible after a `drop` shrinks the list) degrade to no-ops, which keeps
mutation application total and the minimizer panic-free.

### Minimization

Failing plans are reduced with the classic greedy delta-debugging pass: try
removing each mutation, keep the removal whenever the transcript still diverges,
and iterate to a fixed point. The result is a **1-minimal** counter-example, and
the report tells you how many predicate evaluations it took to get there.

---

## Command reference

```text
noisescope <command> [args] [flags]

  lint    <spec.json>                     check a spec for internal consistency
  check   <spec.json> <transcript.json>   replay a transcript, report divergence
  fuzz    <spec.json> <transcript.json>   mutate & minimize failing sequences
  paths   <spec.json> <transcript.json>   emit handshake path JSON (for the viewer)
  version                                 print version
  help                                    show help

Flags:
  --format json|text    output format (default: text; `paths` is always JSON)
  --seed <u64>          fuzz PRNG seed (default: 0x5EED; accepts 0x-hex)
  --trials <n>          fuzz trial budget (default: 256)
  --max-plan-len <n>    max mutations per plan (default: 4)
  --max-findings <n>    stop after n findings (default: 8; 0 = unlimited)
  --out <path>          write output to a file instead of stdout

Exit codes:  0 conforming/clean · 1 divergence/violations · 2 usage/I/O error
```

---

## Bundled fixtures

Three abstract, **wire-incompatible** teaching models live in `fixtures/`:

- **`noise-xx`** — loosely inspired by the Noise `XX` pattern
  (`-> e`, `<- e ee s es`, `-> s se`), collapsed into abstract message tokens.
- **`tls13`** — loosely inspired by the TLS 1.3 flight structure, with the whole
  server flight collapsed into one abstract `ServerFlight` message.
- **`mls`** — loosely inspired by the MLS `Proposal → Commit → Welcome` group
  handshake, with epoch progression modeled as per-role sequence numbers.

Each ships with a conforming transcript plus at least one diverging transcript
(a nonce replay, an out-of-order flight). They exist to make the *tool*
legible — not to represent the real protocols.

---

## Project layout

```text
noisescope/
├── src/
│   ├── main.rs         CLI (lint / check / fuzz / paths)
│   ├── lib.rs          crate root + module map
│   ├── json.rs         std-only JSON parse/serialize
│   ├── model.rs        ProtocolSpec, Transcript, Event
│   ├── parser.rs       JSON → model
│   ├── engine.rs       replay + the four invariants
│   ├── mutate.rs       SplitMix64 PRNG + mutations
│   ├── minimize.rs     delta-debugging minimizer
│   └── divergence.rs   fuzz orchestration + JSON/text reports
├── tests/integration.rs
├── fixtures/           noise / tls / mls specs + transcripts
├── viewer/             TypeScript path viewer (ASCII + SVG)
├── docs/
│   ├── PROTOCOL.md     the full schema & semantics
│   └── assets/         two animated local SVGs
├── Makefile  ·  Cargo.toml  ·  LICENSE  ·  CHANGELOG.md
└── .github/workflows/ci.yml
```

---

## Building everything

```console
$ make check     # fmt-check + clippy (if available) + rust tests + viewer tests
$ make build     # release binary + compiled viewer
$ make demo      # runs the check/fuzz/paths demos shown above
```

The `Makefile` degrades gracefully: optional tools (`rustfmt`, `clippy`) are
skipped with a note if they are not installed, so `make check` works on a bare
toolchain.

---

## Design notes: why it looks the way it does

**Standard library only, on purpose.** The core crate lists *no* dependencies.
That includes JSON: `src/json.rs` is a small, hand-audited reader/writer with
ordered object keys so report output is stable and diff-friendly. The payoff is
supply-chain simplicity — you can read the entire trust surface — and
reproducible builds that will still compile years from now. The cost is that we
re-implement a few conveniences; the tests in `json.rs` cover escapes, surrogate
pairs, integer formatting, and trailing-garbage rejection to keep that honest.

**Determinism as a feature, not an accident.** A fuzzing tool is only useful if
its findings can be reproduced and shared. noisescope threads a single
SplitMix64 seed through the whole run: the top-level `--seed`, an XOR-mixed
per-trial seed, and a per-plan generator are all derived from it by pure
functions. The integration test `fuzzing_is_deterministic_and_minimal` asserts
that two runs with identical configuration serialize to byte-identical JSON.
This is why findings quote their seed (`finding #0 (seed 0x5eed)`): paste the
seed back and you get the same counter-example.

**Fail-soft replay.** Many checkers stop at the first fault. noisescope keeps
walking after ordering faults (skipping the offending event without advancing
state) and layers nonce/sequence checks on top of otherwise-valid steps. One
`check` run therefore surfaces the *whole* set of structural problems, which is
what you want when triaging a broken transcript rather than fixing one fault at
a time only to discover the next.

**Total mutation application.** Minimization repeatedly removes mutations and
re-applies the remainder. Because a `drop` shrinks the event list, later indices
in a plan can fall out of range mid-reduction. Rather than special-casing this,
`apply_one` treats out-of-range edits as no-ops. Mutation application is thus a
*total* function over any plan and any transcript, so the minimizer can never
panic and always terminates at a fixed point.

**Small, sharp data model.** States, roles, and message types are just strings,
so a spec can speak whatever vocabulary a protocol uses. The engine's power
comes from composition — order + role + freshness + sequencing — not from a
sprawling type hierarchy. The whole model fits in `model.rs` in a single sitting.

## Using noisescope in CI

The exit codes make it a drop-in gate. A conforming transcript exits `0`; any
divergence exits `1`; a usage or I/O problem exits `2`. For example, to fail a
pipeline when a captured handshake stops matching its spec:

```console
$ noisescope check spec.json captured.transcript.json || echo "handshake drift!"
```

Or to keep a protocol's negative tests honest — asserting that a transcript you
*expect* to diverge still does — invert the check in your test script. The
bundled GitHub Actions workflow builds the Rust core, runs all tests, compiles
and tests the TypeScript viewer, and exercises the fixtures end-to-end.

## Honest limitations

- **No cryptography, full stop.** Nonces and sequence numbers are integers.
  noisescope cannot and does not reason about keys, AEAD, DH, signatures, or any
  security property. Use a real analyzer (Tamarin, ProVerif, hax, symbolic or
  computational tooling) for security claims.
- **Abstract, non-interoperable fixtures.** The Noise/TLS/MLS models are
  simplifications with **no wire compatibility**. Do not infer anything about
  the real protocols from them.
- **First-match transition selection.** When several transitions match
  `(state, role, msg)`, the first declared one is taken; genuinely
  nondeterministic machines are not explored exhaustively.
- **Bounded, sampled fuzzing.** The fuzzer explores a deterministic sample of
  short mutation plans. No findings means "none in this budget," never "proven
  robust."
- **Structural faults only.** A transcript can be perfectly conforming here and
  still be catastrophic in reality, and vice-versa. This tool answers *"does the
  message shape match the declared machine?"* — nothing more.

---

## License

MIT — see [`LICENSE`](LICENSE). Changelog in [`CHANGELOG.md`](CHANGELOG.md).

---

## Milestones

- [x] **v0.1** - transcript parser, Noise-XX linting (2016)
- [x] **v0.2** - Noise framework model, first mutation operators (2019)
- [x] **v0.3** - TLS 1.3 model, ASCII/SVG viewer rendering (2021)
- [x] **v0.4** - minimization, timing annotations (2022)
- [x] **v0.5** - MLS model, reorder mutation (2024)
- [x] **v0.6** - replay mode, stable JSON export (2025)
- [x] **v1.0** - frozen schema, seeded deterministic mutations, offline viewer (2026)
- [ ] **v1.1** - SSH handshake model (in progress)

All milestones through v1.0 are shipped and verified by `cargo test` plus the
fixture smoke suite. Open work lives under the [Unreleased] heading in the
[CHANGELOG](CHANGELOG.md).

---

## License

MIT - see [LICENSE](LICENSE).

<!-- docs pass by laylae3063: fixture walkthrough -->
