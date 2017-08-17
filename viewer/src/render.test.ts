/**
 * Tests for the noisescope-viewer render module, using the built-in node:test
 * runner (no third-party test framework required).
 */

import { test } from "node:test";
import assert from "node:assert/strict";
import {
  isPathDoc,
  parsePathDoc,
  renderAscii,
  renderSvg,
  type PathDoc,
} from "./render.js";

const sample: PathDoc = {
  protocol: "noise-XX-abstract",
  initial: "await_e",
  accepting: ["established"],
  states: ["await_e", "await_ee", "await_se", "established"],
  final_state: "established",
