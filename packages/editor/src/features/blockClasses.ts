import { CLASS, HEADING_CLASSES } from "../classes.ts";
import type { DecorationRule } from "../livePreview/types.ts";
import type { EditorFeature } from "./types.ts";

/** Block node name → the line class `theme/styles.ts` styles. */
const BLOCK_CLASS: Record<string, string> = {
  Blockquote: CLASS.quote,
  FencedCode: CLASS.code,
  CodeBlock: CLASS.code,
  HorizontalRule: CLASS.rule,
};

/*
 * `Table` is absent on purpose: the `tables` feature styles table lines itself
 * when the source is being edited, and replaces them with a rendered grid
 * otherwise. Adding line classes here too would put decorations on lines the
 * block widget has already removed.
 */

/** Heading node name → level, so lines get `cm-md-h1` … `cm-md-h6`. */
const HEADING_LEVEL: Record<string, number> = {
  ATXHeading1: 1,
  SetextHeading1: 1,
  ATXHeading2: 2,
  SetextHeading2: 2,
  ATXHeading3: 3,
  ATXHeading4: 4,
  ATXHeading5: 5,
  ATXHeading6: 6,
};

/**
 * Puts a class on every line of a block, and sizes headings.
 *
 * Heading size has to come from here rather than from syntax highlighting,
 * because the `#` marks are hidden and the text carries no emphasis of its own.
 */
const rule: DecorationRule = {
  id: "block-classes",
  nodes: [...Object.keys(BLOCK_CLASS), ...Object.keys(HEADING_LEVEL)],

  build({ name, from, to, lineAt, addLineClass }) {
    const level = HEADING_LEVEL[name];
    const firstLine = lineAt(from).number;
    // A setext heading spans its underline as well. Only the text line is the
    // heading, or the `===` rule would render at heading size.
    const lastLine = level ? firstLine : lineAt(to).number;
    const className = level ? HEADING_CLASSES[level - 1] : BLOCK_CLASS[name];

    for (let line = firstLine; line <= lastLine; line++) {
      addLineClass(line, className);
    }
  },
};

export const blockClassesFeature: EditorFeature = {
  id: "block-classes",
  rules: [rule],
};
