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
