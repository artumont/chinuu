import { Decoration } from "@codemirror/view";
import { CLASS } from "../../classes.ts";
import { markClass } from "../../livePreview/decorations.ts";
import { clickToEdit } from "../../livePreview/clickToEdit.ts";
import type { DecorationRule, RuleContext } from "../../livePreview/types.ts";
import type { EditorFeature } from "../types.ts";
import { mathParserExtension, MATH_DISPLAY, MATH_INLINE } from "./parser.ts";
import { MathWidget } from "./widget.ts";

/**
 * Inline math. Single line, so a view-plugin decoration is fine.
 */
const inlineRule: DecorationRule = {
  id: "math-inline",
  nodes: [MATH_INLINE],

  build({ from, to, text, reveals, add }) {
    // Editing: show the TeX inside the same chip it renders in. Returning early
    // without the mark left the expression as bare text the moment the caret
    // arrived the one state where the chip earns most, since `$` is two
    // characters lost against a sentence. Inline code already behaves this way:
    // its box stays put while its backticks are showing.
    if (reveals(from, to)) {
      add(markClass(CLASS.math), from, to);
      return;
    }
    add(
      Decoration.replace({ widget: new MathWidget(text.slice(1, -1), false) }),
      from,
      to,
    );
  },
};

/** Paints every line of the block with the code block's box. */
const boxLines = (
  from: number,
  to: number,
  lineAt: RuleContext["lineAt"],
  addLineClass: RuleContext["addLineClass"],
): void => {
  for (let line = lineAt(from).number; line <= lineAt(to).number; line++) {
    addLineClass(line, CLASS.mathBlock);
  }
};

/**
 * Display math (`$$…$$`).
 *
 * `"block"` scope because a paragraph's inline section can span lines, so this
 * replacement may cross a line break which CM6 refuses to accept from a view
 * plugin. It is *not* a block widget: `$$` can sit mid-paragraph, and block
 * widgets must replace whole lines.
 */
const displayRule: DecorationRule = {
  id: "math-display",
  scope: "block",
  nodes: [MATH_DISPLAY],

  build({ from, to, text, lineAt, reveals, addLineClass, add }) {
    // Editing the TeX: show it inside the same box a revealed fence gets, rather
    // than letting the block lose its edges exactly when it is being worked on.
    if (reveals(from, to)) {
      boxLines(from, to, lineAt, addLineClass);
      return;
    }
    add(
      Decoration.replace({ widget: new MathWidget(text.slice(2, -2), true) }),
      from,
      to,
    );
  },
};

export const mathFeature: EditorFeature = {
  id: "math",
  markdown: mathParserExtension,
  rules: [inlineRule, displayRule],
  // A `$$` block is a single element spanning lines, so clicking it needs the
  // same explicit anchor as a rendered table.
  extensions: [clickToEdit(CLASS.mathDisplay)],
};
