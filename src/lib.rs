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
