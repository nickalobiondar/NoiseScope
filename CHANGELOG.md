# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- planning: SSH handshake model alongside the Noise/MLS/TLS13 models

## [1.0.0] - 2026-07-08

### Added
- stable transcript-lint schema (frozen field names, additive-only)
- minimization pass: shrink a diverging trace to the smallest reproducing case
- deterministic mutation engine with seeded RNG (same seed, same mutation set)
- viewer ships prebuilt `dist/` for offline use

### Verified
- `cargo test` - 40 unit + 6 integration tests green
- fixture smoke tests green for noise-xx, tls13, mls models

## [0.6.0] - 2025-06-19

### Added
- replay transcript mode for re-checking previously diverging traces
- JSON model export with stable field ordering

## [0.5.0] - 2024-04-12

### Added
- MLS (RFC 9420) handshake model
- reordering mutation (covers transcript-reorder attacks)

## [0.4.0] - 2022-09-27
