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
