#!/usr/bin/env node
/**
 * noisescope-viewer CLI.
 *
 * Reads a `noisescope paths` JSON document from a file argument or stdin and
 * renders it as ASCII (default) or SVG.
 *
 * Usage:
 *   noisescope-viewer [--svg] [path.json]
 *   noisescope paths spec.json transcript.json | noisescope-viewer --svg > path.svg
 */

import { readFileSync } from "node:fs";
import { parsePathDoc, renderAscii, renderSvg } from "./render.js";

function readStdin(): string {
  try {
    return readFileSync(0, "utf8");
  } catch {
    return "";
  }
}

function main(argv: string[]): number {
  const args = argv.slice(2);
  let svg = false;
  let file: string | undefined;
  for (const a of args) {
    if (a === "--svg") svg = true;
    else if (a === "--help" || a === "-h") {
      process.stdout.write(
