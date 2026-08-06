# Contributing to NoiseScope

Thanks for looking at NoiseScope - a control instrument for handshake state
machines. The Rust core (this crate) parses protocol models, replays
transcripts and mutates them deterministically; the TypeScript viewer renders
the constellation.

## Ground rules

- Determinism is the point: the same model + transcript + seed must always
  produce the same mutations and the same report. No wall-clock, no map
  iteration order in output.
- The transcript/protocol JSON schema is additive-only; field names are frozen.
- Every new mutation operator ships with a fixture pair (`.ok` transcript and a
  diverging one).

## Workflow

1. Fork, create a topic branch.
2. `cargo fmt --check && cargo clippy --all-targets -- -D warnings`.
3. `cargo test` and the fixture smoke tests from CI must pass.
4. For viewer changes: `npm run check && npm test` in `viewer/`.
5. Conventional commits (`feat:`, `fix:`, `docs:`, `test:`, `chore:`).
