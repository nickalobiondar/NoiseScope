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
