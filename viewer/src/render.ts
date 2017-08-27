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
  return parsed;
}

/**
 * Render an ASCII diagram of the handshake path.
 *
 * Example:
 *   noise-XX-abstract  [reached: established ✓]
 *   (await_e)
 *     └─ initiator:e#1001 ─▶ (await_ee)
 *     └─ responder:e_ee_s_es#2002 ─▶ (await_se)
 *     └─ initiator:s_se ─▶ ((established))
 */
export function renderAscii(doc: PathDoc): string {
  const lines: string[] = [];
  const mark = doc.reached_accepting ? "\u2713" : "\u2717";
  lines.push(`${doc.protocol}  [final: ${doc.final_state} ${mark}]`);
  lines.push(wrapState(doc.initial, doc.accepting));
  for (const step of doc.path) {
    const target = wrapState(step.to, doc.accepting);
    lines.push(`  \u2514\u2500 ${step.event} \u2500\u25B6 ${target}`);
  }
  if (doc.path.length === 0) {
    lines.push("  (no transitions taken)");
  }
  return lines.join("\n");
}

function wrapState(state: string, accepting: string[]): string {
  return accepting.includes(state) ? `((${state}))` : `(${state})`;
}

/** Layout constants for SVG rendering. */
const SVG = {
  nodeW: 150,
  nodeH: 40,
  gapY: 78,
  marginX: 40,
  marginTop: 70,
  width: 460,
};

/**
 * Render a standalone, self-contained SVG diagram of the handshake path with a
 * subtle animated "pulse" travelling along the accepted edges.
 */
export function renderSvg(doc: PathDoc): string {
  const nodes = [doc.initial, ...doc.path.map((s) => s.to)];
  const height =
