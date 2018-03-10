# noisescope Makefile
#
# Targets degrade gracefully: optional tools (rustfmt, clippy) are skipped with a
# note when absent, so `make check` works on a bare Rust toolchain. The viewer
# targets are skipped if Node/npm are not installed.

CARGO ?= cargo
