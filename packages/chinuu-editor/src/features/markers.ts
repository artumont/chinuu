import { hidden } from "../livePreview/decorations.ts";
import type { DecorationRule } from "../livePreview/types.ts";
import type { EditorFeature } from "./types.ts";

/**
 * Pure syntax markers, hidden unless the caret is inside the construct that owns
 * them.
 *
 * `ListMark`, `TaskMarker`, `TableDelimiter` and image syntax are deliberately
 * absent: replacing them with nothing leaves a hole where the marker was, which
 * reads as a broken render rather than live preview. Those need a widget to
 * paint something in their place.
 */
const MARKERS = [
  "HeaderMark",
  "EmphasisMark",
  "StrikethroughMark",
  "QuoteMark",
  "CodeMark",
  "CodeInfo",
];

/**
 * Note: `LinkMark` and `URL` are deliberately absent.
 *
 * The `links` feature replaces the whole `Link` node with an anchor, and two
 * replace decorations over the same range overlap. Keeping them here would make
 * the two features mutually exclusive rather than composable.
 */

const rule: DecorationRule = {
  id: "markers",
  nodes: [...MARKERS],

  build({ from, to, parent, text, reveals, add }) {
    // A zero-length replacement needs a widget to attach to.
    if (from === to) return;

    // Reveal only the construct the caret is actually inside. Hiding per *line*
    // instead would reveal every mark on the caret's line and reflow the whole
    // line on each cursor move.
    if (reveals(parent?.from ?? from, parent?.to ?? to)) return;

    // A replace decoration may not span a line break.
    if (text.includes("\n")) return;

    add(hidden, from, to);
  },
};

export const markersFeature: EditorFeature = { id: "markers", rules: [rule] };
