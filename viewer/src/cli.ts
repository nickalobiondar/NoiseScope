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
        "noisescope-viewer [--svg] [path.json]\n" +
          "Renders `noisescope paths` output as ASCII or SVG.\n" +
          "(structural visualization only; not a cryptographic proof tool)\n",
      );
      return 0;
    } else if (!a.startsWith("--")) file = a;
    else {
      process.stderr.write(`unknown flag: ${a}\n`);
      return 2;
    }
  }

  const raw = file ? readFileSync(file, "utf8") : readStdin();
  if (!raw.trim()) {
    process.stderr.write("no input (pass a file or pipe JSON on stdin)\n");
    return 2;
  }

  let doc;
  try {
    doc = parsePathDoc(raw);
  } catch (e) {
    process.stderr.write(`error: ${(e as Error).message}\n`);
    return 2;
  }

  process.stdout.write((svg ? renderSvg(doc) : renderAscii(doc)) + "\n");
  return 0;
}

process.exit(main(process.argv));

# draft note 14
