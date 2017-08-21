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
  reached_accepting: true,
  path: [
    { event_index: 0, event: "initiator:e#1001", from: "await_e", to: "await_ee" },
    { event_index: 1, event: "responder:e_ee_s_es#2002", from: "await_ee", to: "await_se" },
    { event_index: 2, event: "initiator:s_se", from: "await_se", to: "established" },
  ],
};

test("isPathDoc accepts valid docs", () => {
  assert.equal(isPathDoc(sample), true);
});

test("isPathDoc rejects garbage", () => {
  assert.equal(isPathDoc({ protocol: 1 }), false);
  assert.equal(isPathDoc(null), false);
  assert.equal(isPathDoc("nope"), false);
});

test("parsePathDoc round-trips JSON", () => {
  const doc = parsePathDoc(JSON.stringify(sample));
  assert.equal(doc.protocol, "noise-XX-abstract");
  assert.equal(doc.path.length, 3);
});

test("parsePathDoc throws on invalid", () => {
  assert.throws(() => parsePathDoc("{}"));
});
