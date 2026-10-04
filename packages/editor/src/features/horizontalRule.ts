import { Decoration, WidgetType } from "@codemirror/view";
import { CLASS } from "../classes.ts";
import type { DecorationRule } from "../livePreview/types.ts";
import type { EditorFeature } from "./types.ts";

/**
 * Draws a real rule in place of `---`.
 *
 * A zero-height inline-block carrying a top border. A literal `<hr>` would be
 * invalid nesting inside a `contenteditable` line, and a row of dashes is
 * exactly what this exists to replace. `vertical-align: middle` in
 * `theme/styles.ts` puts the border through the middle of the line box.
 */
class HorizontalRuleWidget extends WidgetType {
  /** Every rule is identical, so CM6 can reuse a single DOM node for all of them. */
  eq(): boolean {
    return true;
  }

  /**
   * Decorative, so let clicks through. The default (`true`) would make the rule
   * swallow clicks, leaving no way to put the caret on the line and reveal the
   * `---` source again.
   */
  ignoreEvent(): boolean {
    return false;
  }

  toDOM(): HTMLElement {
    const rule = document.createElement("span");
    rule.className = CLASS.horizontalRule;
    rule.setAttribute("aria-hidden", "true");
    return rule;
  }
}

const decoration = Decoration.replace({
  widget: new HorizontalRuleWidget(),
});

const rule: DecorationRule = {
  id: "horizontal-rule",
  nodes: ["HorizontalRule"],

  build({ from, to, reveals, add }) {
    // The source text returns when the caret is on the line, following the same
    // reveal rule as every hidden mark.
    if (reveals(from, to)) return;
    add(decoration, from, to);
  },
};

export const horizontalRuleFeature: EditorFeature = {
  id: "horizontal-rule",
  rules: [rule],
};
