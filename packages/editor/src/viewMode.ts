import {
  EditorState,
  StateEffect,
  StateField,
  Facet,
  type Extension,
} from "@codemirror/state";
import { EditorView, ViewPlugin } from "@codemirror/view";
import { getCM } from "@replit/codemirror-vim";
import { CLASS } from "./classes.ts";

/**
 * How the document is presented.
 *
 * - `live` the default. Markdown renders in place, and constructs reveal their
 *   source when the caret is inside them.
 * - `reading` fully rendered and read-only. Nothing ever reveals, because there
 *   is no editing to do.
 */
export type ViewMode = "live" | "reading";

export const viewMode = Facet.define<ViewMode, ViewMode>({
  combine: (values) => values[0] ?? "live",
});

/**
 * Whether constructs should reveal their markdown source.
 *
 * False in reading mode: a renderer that showed `**` as soon as you clicked a
 * bold word would not be a reading view. Every reveal rule consults this through
 * `RuleContext.reveals`, so no rule needs to know about modes.
 */
export const revealsSource = (state: EditorState): boolean =>
  state.facet(viewMode) !== "reading";

/** Internal: only the mirroring plugin below dispatches this. */
const setVimInsert = StateEffect.define<boolean>();
export const vimInsertMode = StateField.define<boolean>({
  create: () => false,
  update(value, transaction) {
    for (const effect of transaction.effects) {
      if (effect.is(setVimInsert)) return effect.value;
    }
    return value;
  },
});

/**
 * Re-reads vim's mode into the state field, synchronously where allowed.
 *
 * The dispatch used to be deferred through a microtask unconditionally, which left
 * the field one microtask behind. That mattered as soon as a caller dispatched a
 * selection immediately afterwards: `reveals` reads this field, so clicking a
 * rendered table or `$$` block moved the caret but left the construct rendered.
 * The `catch` keeps the original safety net a dispatch is not allowed while an
 * update is in progress without paying for it in the common case.
 */
const syncInsertMode = (view: EditorView): void => {
  const vimState = getCM(view)?.state?.vim as
    { insertMode?: boolean } | undefined;
  // No vim means no modal editing, so the editor is always entering text.
  const insert = vimState ? Boolean(vimState.insertMode) : true;
  if (insert === view.state.field(vimInsertMode, false)) return;

  try {
    view.dispatch({ effects: setVimInsert.of(insert) });
  } catch {
    queueMicrotask(() => {
      if (insert !== view.state.field(vimInsertMode, false)) {
        view.dispatch({ effects: setVimInsert.of(insert) });
      }
    });
  }
};

/**
 * Mirrors vim's text-entering state into `vimInsertMode`.
 *
 * Verified reachable: `getCM(view).state.vim.insertMode` reads `false` on a fresh
 * editor and `true` after a synthetic `i` keydown, so this works headlessly too.
 * `getCM` yields undefined when vim is not installed, and that case counts as
 * *entering text* true, not false so an editor without modal editing keeps the
 * caret-based reveal it had before this field existed. Only vim's normal and
 * visual modes turn it off, which is what stops a rendered link reverting to
 * source under the cursor.
 *
 * Entering insert mode changes neither the document nor the selection, so vim may
 * dispatch no CM6 transaction at all measured: the mode read as `true` while the
 * state field stayed `false`. Hence the DOM listeners; `vim-mode-change` is what
 * vim emits, and `keyup` is the fallback that covers any build that does not emit
 * it. `syncInsertMode` re-reads vim's own state either way, so the two cannot
 * disagree.
 */
const mirrorVimMode = ViewPlugin.fromClass(
  class {
    private readonly resync = (): void => syncInsertMode(this.view);

    constructor(readonly view: EditorView) {
      // On `document`, not `view.contentDOM`: a plugin is constructed before the
      // editor's DOM is guaranteed to be in place, and `document` always exists.
      // Capture phase still sees events dispatched inside the content DOM.
      document.addEventListener("vim-mode-change", this.resync, true);
      document.addEventListener("keyup", this.resync, true);
      // Needed so a vim editor settles on `false` (normal mode) before the first
      // paint, rather than briefly revealing.
      syncInsertMode(this.view);
    }

    destroy() {
      document.removeEventListener("vim-mode-change", this.resync, true);
      document.removeEventListener("keyup", this.resync, true);
    }
  },
);

export const vimModeTracking: Extension = [vimInsertMode, mirrorVimMode];

/**
 * Puts vim into insert mode.
 *
 * `@replit/codemirror-vim` exports only `vim`, `getCM` and `CodeMirror` its
 * `Vim.handleKey` entry point is internal, and flipping `cm.state.vim.insertMode`
 * by hand would skip the rest of the mode transition. Dispatching the key a user
 * would press goes through vim's own handling instead.
 *
 * Guarded on both sides: with no vim installed that key would be *inserted into
 * the document as text*, and when already in insert mode there is nothing to do.
 */
export const enterInsertMode = (view: EditorView): void => {
  const vimState = getCM(view)?.state?.vim as
    { insertMode?: boolean } | undefined;
  if (!vimState || vimState.insertMode) return;

  view.contentDOM.dispatchEvent(
    new KeyboardEvent("keydown", {
      key: "i",
      code: "KeyI",
      bubbles: true,
      cancelable: true,
    }),
  );

  // Settle the field now, not on the next microtask. Callers dispatch a selection
  // straight after this, and `reveals` reads the field a lagging field meant the
  // caret moved into a construct without it revealing.
  syncInsertMode(view);
};

/**
 * Toggles the reading-mode class on an editor root.
 *
 * Called explicitly by the editor rather than from a `ViewPlugin`, because a
 * plugin's constructor runs during `new EditorView(...)` and the class was
 * silently never applied from there. The theme needs it to drop the caret and the
 * active-line band, which are editing affordances a reading view should not show.
 */
export const applyModeClass = (view: EditorView, mode: ViewMode): void => {
  view.dom.classList.toggle(CLASS.readingMode, mode === "reading");
};

/**
 * Refuses every transaction that would alter the document.
 *
 * `EditorState.readOnly` is a facet that *commands* consult: `insertNewline` and its
 * neighbours return false, and `editable: false` takes the content DOM out of
 * contenteditable. Neither stops an extension calling `view.dispatch({changes})`
 * itself. Measured in reading mode before this existed: a bare dispatch changed the
 * document, 629 → 630 characters, and both the task checkbox and the table library
 * used exactly that route.
 *
 * A transaction filter is the only place that sees every transaction whoever sent
 * it. Selection-only transactions pass, so reading still allows navigating.
 */
const noDocumentChanges = EditorState.transactionFilter.of((transaction) =>
  transaction.docChanged ? [] : transaction,
);

export const viewModeExtensions = (mode: ViewMode): Extension => {
  const editable = mode !== "reading";
  return [
    viewMode.of(mode),
    EditorState.readOnly.of(!editable),
    EditorView.editable.of(editable),
    // Reading mode only. Live mode has to keep dispatching freely.
    ...(editable ? [] : [noDocumentChanges]),
  ];
};
