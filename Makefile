# noisescope Makefile
#
# Targets degrade gracefully: optional tools (rustfmt, clippy) are skipped with a
# note when absent, so `make check` works on a bare Rust toolchain. The viewer
# targets are skipped if Node/npm are not installed.

CARGO ?= cargo
NPM ?= npm
VIEWER_DIR := viewer

NOISE_SPEC := fixtures/noise-xx.protocol.json
NOISE_OK   := fixtures/noise-xx.ok.transcript.json
TLS_SPEC   := fixtures/tls13.protocol.json
TLS_REORD  := fixtures/tls13.reorder.transcript.json

.DEFAULT_GOAL := help

.PHONY: help
help: ## Show this help
	@echo "noisescope make targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  %-14s %s\n", $$1, $$2}'

.PHONY: build
build: build-rust build-viewer ## Build the release binary and the viewer

.PHONY: build-rust
build-rust: ## Build the release Rust binary
	$(CARGO) build --release
