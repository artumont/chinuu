import type { Extension } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { enterInsertMode, revealsSource } from "../viewMode.ts";

/**
 * Puts the caret into a rendered widget's source when the widget is clicked.
 *
 * A multi-line replacement is a single element in the layout but covers several
 * document lines, so CM6's own click handling cannot map a point inside it back
 * to a document position. Measured: clicking a rendered table put the caret on
 * the `$$` line, two blocks further down, and the grid stayed rendered so the
 * table was effectively not editable in live mode.
 *
 * `posAtDOM` on the widget root returns the widget's own start, which is inside
 * the construct, so the existing reveal rule converts it back to source.
 *
 * Deliberately inert in reading mode: there, revealing is off and letting the
 * click through is what allows text selection.
 */
export const clickToEdit = (className: string): Extension =>
  EditorView.domEventHandlers({
    mousedown(event, view) {
      if (!revealsSource(view.state)) return false;
      const target = event.target;
      if (!(target instanceof HTMLElement)) return false;

      const rendered = target.closest<HTMLElement>(`.${className}`);
      if (!rendered) return false;

      event.preventDefault();
      // Order matters: capture the anchor while the widget is still rendered, then
      // switch mode, then move. Entering insert mode last left the construct
      // rendered, because `reveals` reads the mode field that the selection change
      // is evaluated against.
      const anchor = view.posAtDOM(rendered);
      view.focus();
      enterInsertMode(view);
      view.dispatch({ selection: { anchor } });
      return true;
    },
  });
