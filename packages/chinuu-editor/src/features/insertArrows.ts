import { cursorLineDown, cursorLineUp } from "@codemirror/commands";
import { EditorView, ViewPlugin } from "@codemirror/view";
import { getCM } from "@replit/codemirror-vim";
import type { EditorFeature } from "./types.ts";

/**
 * Keeps ↑/↓ on the goal column while vim is in insert mode.
 *
 * Measured: in insert mode the caret drifts right on lines with hidden leading
 * markup `##` → column 2, `- [ ]` → column 1, `> ` → column 1, and a ```
 * fence → column 3. On the fence line that reads as the cursor jumping to the end
 * of the code block instead of entering it. Normal mode is unaffected because vim
 * runs its own goal-column motion there.
 *
 * Cause: vim binds no arrow key and neither does CM6, so in insert mode the
 * *browser* moves the caret natively to the nearest DOM position and replaced
 * markup is not in the DOM, so the caret lands after it.
 *
 * A CM6 `keymap` did not work, at plain precedence nor at `Prec.highest`: vim's
 * keymap is also `Prec.highest` and still won. So this listens in the capture
 * phase on the editor root, which is an ancestor of the content DOM, and stops
 * the event before CM6's keymap can hand it to vim. `cursorLineDown`/`Up` keep the
 * goal column, so the caret lands on column 0 and the reveal then shows the fence.
 *
 * Returning nothing in normal and visual modes leaves them entirely to vim.
 */
const insertArrowKeys = ViewPlugin.fromClass(
  class {
    private readonly onKeyDown = (event: KeyboardEvent): void => {
      if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
      if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey)
        return;

      const vimState = getCM(this.view)?.state?.vim as
        { insertMode?: boolean } | undefined;
      if (!vimState?.insertMode) return;

      event.preventDefault();
      event.stopImmediatePropagation();
      const command = event.key === "ArrowDown" ? cursorLineDown : cursorLineUp;
      command(this.view);
    };

    constructor(readonly view: EditorView) {
      // `view.dom`, not `view.contentDOM`: the root is an ancestor and is
      // guaranteed to exist at construction time, and capture still beats the
      // content DOM's own listeners.
      this.view.dom.addEventListener("keydown", this.onKeyDown, true);
    }

    destroy() {
      this.view.dom.removeEventListener("keydown", this.onKeyDown, true);
    }
  },
);

export const insertArrowFeature: EditorFeature = {
  id: "insert-arrows",
  extensions: [insertArrowKeys],
};
