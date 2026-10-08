import {
  EditorView,
  ViewPlugin,
  type DecorationSet,
  type ViewUpdate,
} from "@codemirror/view";
import type { Transaction } from "@codemirror/state";
import { buildDecorations, indexRules, treeChanged } from "./rules.ts";
import { StateField } from "@codemirror/state";
import type { DecorationRule } from "./types.ts";
import { vimInsertMode } from "../viewMode.ts";

/**
 * Inline decoration source.
 *
 * Carries everything a view plugin is allowed to emit: marks, line classes and
 * replacements contained within a single line. Anything that needs a block
 * widget or a replacement spanning a line break must go through
 * `createBlockDecorations` instead CM6 rejects those from a plugin with
 * "Block decorations may not be specified via plugins".
 */
export const createLivePreview = (rules: readonly DecorationRule[]) => {
  const index = indexRules(rules);

  return ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;

      constructor(view: EditorView) {
        this.decorations = buildDecorations(view.state, index);
      }

      update(update: ViewUpdate) {
        if (
          update.docChanged ||
          update.selectionSet ||
          update.viewportChanged ||
          treeChanged(update.startState, update.state) ||
          // `reveals` reads this, so entering or leaving vim's insert mode has to
          // rebuild the decorations. Without it the line-granular reveal never
          // appeared: the mode changed, but nothing here noticed.
          update.startState.field(vimInsertMode, false) !==
            update.state.field(vimInsertMode, false)
        ) {
          this.decorations = buildDecorations(update.view.state, index);
        }
      }
    },
    {
      // No `atomicRanges`.
      //
      // A mark is only hidden while the caret is outside its construct, so the
      // reveal replaces the range before the caret could enter it measured:
      // moving right through `**bold**` visits every offset already. It was
      // restored once, when normal mode stopped revealing and the caret could sit
      // inside a hidden `**`; with reveal back on caret entry there is nothing for
      // it to fix, and a stale atomic range on a line whose leading markup is
      // hidden can snap the caret to the range boundary on arrival.
      decorations: (plugin) => plugin.decorations,
    },
  );
};

/**
 * Block decoration source.
 *
 * A state field rather than a plugin, which is what makes block widgets and
 * multi-line replacements legal. Verified: the same decoration from a
 * `ViewPlugin` throws `RangeError: Block decorations may not be specified via
 * plugins`, and from here renders.
 *
 * The selection check is what drives source-reveal, and `treeChanged` covers the
 * asynchronous parse a state field cannot see view updates, so without it the
 * decorations would lag behind the tree.
 */
export const createBlockDecorations = (rules: readonly DecorationRule[]) => {
  const index = indexRules(rules);

  return StateField.define<DecorationSet>({
    create: (state) => buildDecorations(state, index),

    update(decorations, transaction: Transaction) {
      if (
        transaction.docChanged ||
        transaction.selection !== undefined ||
        treeChanged(transaction.startState, transaction.state)
      ) {
        return buildDecorations(transaction.state, index);
      }
      return decorations.map(transaction.changes);
    },

    provide: (field) => EditorView.decorations.from(field),
  });
};
