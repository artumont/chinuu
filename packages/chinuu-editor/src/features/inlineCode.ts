import { CLASS } from "../classes.ts";
import { markClass } from "../livePreview/decorations.ts";
import type { DecorationRule } from "../livePreview/types.ts";
import type { EditorFeature } from "./types.ts";

/**
 * Boxes inline code.
 *
 * The box cannot come from the `tags.monospace` highlight rule: markdown
 * registers `"InlineCode CodeText": tags.monospace`, so the node and its text
 * child both carry the tag and a box there would nest and double the padding.
 * Hence a mark decoration on the outer node only.
 */
const rule: DecorationRule = {
  id: "inline-code",
  nodes: ["InlineCode"],

  build({ from, to, add }) {
    add(markClass(CLASS.inlineCode), from, to);
  },
};

export const inlineCodeFeature: EditorFeature = {
  id: "inline-code",
  rules: [rule],
};
