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

test("renderAscii marks accepting state with double parens", () => {
  const out = renderAscii(sample);
  assert.match(out, /\(\(established\)\)/);
  assert.match(out, /initiator:e#1001/);
});

test("renderAscii handles empty path", () => {
  const empty: PathDoc = { ...sample, path: [], final_state: "await_e", reached_accepting: false };
  const out = renderAscii(empty);
  assert.match(out, /no transitions taken/);
});

test("renderSvg emits well-formed root and animation", () => {
  const svg = renderSvg(sample);
  assert.match(svg, /^<svg xmlns="http:\/\/www\.w3\.org\/2000\/svg"/);
  assert.match(svg, /<\/svg>$/);
  assert.match(svg, /<animate /);
  // Every node label appears.
  for (const s of sample.states) {
    assert.ok(svg.includes(s), `svg should mention state ${s}`);
  }
});

test("renderSvg escapes XML metacharacters", () => {
  const doc: PathDoc = {
    ...sample,
    protocol: "a<b>&\"'",
    path: [],
  };
  const svg = renderSvg(doc);
  assert.match(svg, /a&lt;b&gt;&amp;&quot;&apos;/);
  assert.ok(!svg.includes("a<b>"));
});

# draft note 54
