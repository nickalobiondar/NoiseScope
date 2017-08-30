//! End-to-end integration tests exercising the bundled fixtures.
//!
//! These verify that the shipped Noise/TLS/MLS-inspired abstract fixtures parse,
//! lint clean, and produce the divergence outcomes documented in the README.

use std::fs;
use std::path::PathBuf;

use noisescope::divergence::{fuzz, report_json, FuzzConfig};
use noisescope::engine::{replay, ViolationKind};
use noisescope::mutate::apply_all;
