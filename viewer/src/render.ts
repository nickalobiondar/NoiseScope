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
    SVG.marginTop + nodes.length * SVG.gapY + SVG.marginTop / 2;
  const cx = SVG.width / 2;

  const parts: string[] = [];
  parts.push(
    `<svg xmlns="http://www.w3.org/2000/svg" width="${SVG.width}" height="${height}" viewBox="0 0 ${SVG.width} ${height}" role="img" aria-label="Handshake path for ${escapeXml(doc.protocol)}">`,
  );
  parts.push(`<title>${escapeXml(doc.protocol)} handshake path</title>`);
  parts.push(defs());
  parts.push(
    `<rect width="${SVG.width}" height="${height}" fill="#0b1020"/>`,
  );
  parts.push(
    `<text x="${cx}" y="34" fill="#8ab4f8" font-family="monospace" font-size="16" text-anchor="middle">${escapeXml(doc.protocol)}</text>`,
  );
  const status = doc.reached_accepting ? "reached accepting" : "did NOT accept";
  const statusColor = doc.reached_accepting ? "#7ee787" : "#ff7b72";
  parts.push(
    `<text x="${cx}" y="52" fill="${statusColor}" font-family="monospace" font-size="11" text-anchor="middle">${escapeXml(doc.final_state)} — ${status}</text>`,
  );

  const nodeY = (i: number): number => SVG.marginTop + i * SVG.gapY;

  // Edges first (so nodes render on top).
  for (let i = 0; i < doc.path.length; i++) {
    const y1 = nodeY(i) + SVG.nodeH;
    const y2 = nodeY(i + 1);
    parts.push(
      `<line x1="${cx}" y1="${y1}" x2="${cx}" y2="${y2}" stroke="#30475e" stroke-width="2" marker-end="url(#arrow)"/>`,
    );
    // Animated pulse dot travelling down the edge.
    const dur = 2.4;
    const begin = (i * 0.5).toFixed(2);
    parts.push(
      `<circle r="4" fill="#8ab4f8"><animate attributeName="cy" from="${y1}" to="${y2}" dur="${dur}s" begin="${begin}s" repeatCount="indefinite"/><animate attributeName="cx" values="${cx};${cx}" dur="${dur}s" begin="${begin}s" repeatCount="indefinite"/><animate attributeName="opacity" values="0;1;1;0" dur="${dur}s" begin="${begin}s" repeatCount="indefinite"/></circle>`,
    );
    // Edge label.
    const midY = (y1 + y2) / 2 + 4;
    parts.push(
      `<text x="${cx + 14}" y="${midY}" fill="#c9d1d9" font-family="monospace" font-size="11">${escapeXml(doc.path[i].event)}</text>`,
    );
  }

  // Nodes.
  for (let i = 0; i < nodes.length; i++) {
    const y = nodeY(i);
    const x = cx - SVG.nodeW / 2;
    const accepting = doc.accepting.includes(nodes[i]);
    const fill = accepting ? "#132e1a" : "#111a2e";
