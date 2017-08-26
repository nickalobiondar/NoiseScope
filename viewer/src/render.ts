/**
 * noisescope-viewer — render module.
 *
 * Consumes the JSON emitted by `noisescope paths ...` and renders the handshake
 * as either an ASCII diagram (for terminals) or a standalone SVG (for docs).
 *
 * This is a *structural* visualizer. It draws the state-machine path a
 * transcript took; it makes no cryptographic claim whatsoever.
 */

/** A single step in a handshake path, as produced by `noisescope paths`. */
export interface PathStep {
  event_index: number;
  event: string;
  from: string;
  to: string;
}

/** The full path document produced by `noisescope paths`. */
export interface PathDoc {
  protocol: string;
  initial: string;
  accepting: string[];
  states: string[];
  final_state: string;
  reached_accepting: boolean;
  path: PathStep[];
}

/** Type guard that validates an unknown value is a {@link PathDoc}. */
export function isPathDoc(value: unknown): value is PathDoc {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.protocol === "string" &&
    typeof v.initial === "string" &&
    Array.isArray(v.accepting) &&
    Array.isArray(v.states) &&
    typeof v.final_state === "string" &&
    typeof v.reached_accepting === "boolean" &&
    Array.isArray(v.path)
  );
}

/** Parse a JSON string into a validated {@link PathDoc}. Throws on mismatch. */
export function parsePathDoc(json: string): PathDoc {
  const parsed: unknown = JSON.parse(json);
  if (!isPathDoc(parsed)) {
    throw new Error("input is not a valid noisescope path document");
  }
