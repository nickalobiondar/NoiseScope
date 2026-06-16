//! # noisescope
//!
//! A **cryptographic handshake transcript state-machine analyzer and fuzzer**.
//!
//! `noisescope` treats a handshake as a declarative finite state machine plus a
//! set of structural metadata invariants (role, order, nonce freshness,
//! sequence monotonicity). It replays observed *transcripts* against a *spec*,
//! reports where they diverge, and can *fuzz* a conforming transcript with
//! deterministic mutations (drop / duplicate / reorder / corrupt-metadata) to
//! discover — and then **minimize** — the shortest mutation sequences that
//! break conformance.
//!
//! ## What this is not
//!
//! This is **not** a cryptographic proof tool. It does not evaluate secrecy,
//! authentication, forward secrecy, or any computational/symbolic security
//! property. It knows nothing about real key schedules, AEAD constructions, or
//! wire formats. The bundled Noise/TLS/MLS-inspired fixtures are *abstract*
//! teaching models and make **no claim of wire compatibility**.
//!
//! What it *does* give you is a fast, deterministic, dependency-free way to
//! reason about the **shape** of a handshake: which message orders a state
//! machine accepts, and how small perturbations cause divergence.
//!
//! ## Module map
//!
//! * [`json`] — dependency-free JSON parse/serialize.
//! * [`model`] — protocol spec + transcript data types.
//! * [`parser`] — JSON → model.
//! * [`engine`] — transcript replay + invariant checking.
//! * [`mutate`] — deterministic mutation generation/application.
//! * [`minimize`] — delta-debugging-style plan minimization.
//! * [`divergence`] — fuzz orchestration + JSON/text reports.

pub mod divergence;
pub mod engine;
pub mod json;
pub mod minimize;
pub mod model;
pub mod mutate;
pub mod parser;

/// Semantic version of the library/CLI.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// draft note 49
