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

.PHONY: build-viewer
build-viewer: ## Build the TypeScript viewer (skipped if npm missing)
	@command -v $(NPM) >/dev/null 2>&1 && { \
		cd $(VIEWER_DIR) && $(NPM) install --no-audit --no-fund && $(NPM) run build; \
	} || echo "note: npm not found, skipping viewer build"

.PHONY: test
test: test-rust test-viewer ## Run all tests

.PHONY: test-rust
test-rust: ## Run Rust unit + integration tests
	$(CARGO) test

.PHONY: test-viewer
test-viewer: build-viewer ## Run the viewer tests (skipped if npm missing)
	@command -v $(NPM) >/dev/null 2>&1 && { \
		cd $(VIEWER_DIR) && $(NPM) test; \
	} || echo "note: npm not found, skipping viewer tests"

.PHONY: fmt
fmt: ## Format Rust sources (skipped if rustfmt missing)
	@command -v rustfmt >/dev/null 2>&1 && $(CARGO) fmt || echo "note: rustfmt not found, skipping"

.PHONY: fmt-check
fmt-check: ## Check formatting (skipped if rustfmt missing)
	@command -v rustfmt >/dev/null 2>&1 && $(CARGO) fmt --check || echo "note: rustfmt not found, skipping fmt-check"

.PHONY: clippy
clippy: ## Run clippy (skipped if clippy missing)
	@command -v cargo-clippy >/dev/null 2>&1 && $(CARGO) clippy --all-targets -- -D warnings || echo "note: clippy not found, skipping"

.PHONY: check
check: fmt-check clippy test ## fmt-check + clippy + all tests

.PHONY: demo
demo: build-rust ## Run the README demos against the fixtures
	@echo "== lint =="
	-$(CARGO) run --quiet -- lint $(NOISE_SPEC)
	@echo "== check (conforming) =="
	-$(CARGO) run --quiet -- check $(NOISE_SPEC) $(NOISE_OK)
	@echo "== check (diverging: reorder) =="
	-$(CARGO) run --quiet -- check $(TLS_SPEC) $(TLS_REORD)
	@echo "== fuzz + minimize =="
	-$(CARGO) run --quiet -- fuzz $(NOISE_SPEC) $(NOISE_OK) --seed 0x5EED --trials 200 --max-findings 1

.PHONY: clean
clean: ## Remove build artifacts
	$(CARGO) clean
	@rm -rf $(VIEWER_DIR)/dist $(VIEWER_DIR)/node_modules

# draft note 39
