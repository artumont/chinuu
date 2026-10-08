import { Compartment, EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { getCM, vim } from "@replit/codemirror-vim";
import {
  behaviourExtensions,
  inputExtensions,
  markdownSupport,
  selectionExtensions,
} from "./extensions.ts";
import { imageResolver, type ImageResolver } from "./features/images.ts";
import {
  blockRulesFor,
  defaultFeatures,
  extensionsFor,
  inlineRulesFor,
  markdownSupportFor,
  menuSectionsFor,
} from "./features/index.ts";
import type { EditorFeature } from "./features/types.ts";
import {
  wikiLinkResolver,
  type WikiLinkResolver,
} from "./features/wikiLinks.ts";
import {
  createBlockDecorations,
  createLivePreview,
} from "./livePreview/plugin.ts";
import { contextMenu } from "./menu/contextMenu.ts";
import { menuTheme } from "./menu/menuTheme.ts";
import type { MenuSection } from "./menu/types.ts";
import {
  applyModeClass,
  viewModeExtensions,
  vimModeTracking,
  type ViewMode,
} from "./viewMode.ts";
import {
  chinuuLight,
  documentTheme,
  themeVariables,
  type ChinuuTheme,
} from "./theme/index.ts";

export interface EditorOptions {
  /** Colour theme. Defaults to `chinuuLight`. */
  readonly theme?: ChinuuTheme;
  /** Feature set. Defaults to `defaultFeatures`; pass `[]` for bare markdown. */
  readonly features?: readonly EditorFeature[];
  /**
   * Extra right-click menu sections, appended after the features' own.
   *
   * The lightest way to add an action no feature object needed:
   *
   * ```ts
   * initEditor(parent, doc, {
   *   menu: [{
   *     id: "my-actions",
   *     label: "My actions",
   *     items: [{
   *       id: "uppercase",
   *       label: "Uppercase",
   *       run: ({ view }) => {
   *         const target = targetOf(view.state);
   *         const upper = view.state.sliceDoc(target.from, target.to).toUpperCase();
   *         view.dispatch({ changes: { from: target.from, to: target.to, insert: upper } });
   *       },
   *     }],
   *   }],
   * });
   * ```
   *
   * For anything that also needs parser support, decorations or keymaps, use
   * `EditorFeature.menu` instead so the whole capability plugs and unplugs
   * together.
   */
  readonly menu?: readonly MenuSection[];
  /** Presentation mode. Defaults to `"live"`. */
  readonly mode?: ViewMode;
  /** Blink the caret. Defaults to `true`. */
  readonly blinkCursor?: boolean;
  /**
   * Called whenever vim changes mode, for a status bar.
   *
   * `@replit/codemirror-vim` emits `vim-mode-change` on its own CM5 compatibility
   * object rather than through CM6 state, so it is passed straight on. The names are
   * vim's own: `normal`, `insert`, `visual` or `replace`.
   */
  readonly vimModeCallback?: (mode: string) => void;
  /**
   * Maps an image path from the document onto something the webview can load.
   *
   * A note says `![fig](./fig.png)`, and that path is relative to the note rather
   * than to the application, so a webview cannot fetch it unaided. The desktop app
   * should return a Tauri `asset:` URL here. With nothing supplied the path is used
   * verbatim, which is right when a server publishes notes and their images from
   * one directory.
   */
  readonly resolveImage?: ImageResolver;
  /**
   * Maps a wiki link's target onto an `href`.
   *
   * `[[another-note]]` names a note rather than a location, so only the host knows
   * how a note maps onto a route or an app scheme the desktop app can return its
   * own scheme here and intercept the navigation. With nothing supplied the anchor
   * renders inert but styled: there is no path to guess.
   */
  readonly resolveWikiLink?: WikiLinkResolver;
}

export interface EditorHandle {
  /** Underlying CM6 view, for widgets and file wiring. */
  readonly view: EditorView;
  /** Current document text. */
  getValue(): string;
  /** Replace the whole document, e.g. when opening another note. */
  setValue(doc: string): void;
  /** Swap the colour theme on the live view, without rebuilding the document. */
  setTheme(theme: ChinuuTheme): void;
  /** Replug the feature set on the live view. */
  setFeatures(features: readonly EditorFeature[]): void;
  /** Replace the extra menu sections supplied through `EditorOptions.menu`. */
  setMenu(sections: readonly MenuSection[]): void;
  /** Switch between live and reading presentation. */
  setMode(mode: ViewMode): void;
  /** Turn caret blinking on or off. */
  setCursorBlink(enabled: boolean): void;
  /** Tear down the view and release DOM + listeners. */
  destroy(): void;
}

export const initEditor = (
  parent: HTMLElement,
  contents = "",
  options: EditorOptions = {},
): EditorHandle => {
  // Per-editor, not module-level: a shared compartment is shared mutable state,
  // so one view's swap would reach every other view.
  const themeCompartment = new Compartment();
  const modeCompartment = new Compartment();
  const markdownCompartment = new Compartment();
  const featureCompartment = new Compartment();
  const blockCompartment = new Compartment();
  const menuCompartment = new Compartment();
  const blinkCompartment = new Compartment();

  /**
   * Everything a feature set contributes to the view.
   *
   * Both decoration sources come from the same feature set, so replacing that set at
   * runtime swaps the whole rendering without rebuilding the document.
   */
  const featureExtensionsOf = (features: readonly EditorFeature[]) => [
    ...extensionsFor(features),
    createLivePreview(inlineRulesFor(features)),
  ];

  const blockDecorationsOf = (features: readonly EditorFeature[]) =>
    createBlockDecorations(blockRulesFor(features));

  let currentFeatures = options.features ?? defaultFeatures;
  let currentMode = options.mode ?? "live";
  let currentBlink = options.blinkCursor ?? true;
  let extraMenu: readonly MenuSection[] = options.menu ?? [];

  /** Feature sections first, then whatever the embedding app supplied. */
  const buildMenu = () =>
    contextMenu([...menuSectionsFor(currentFeatures), ...extraMenu]);

  const extensions = [
    // `vim()` stays first so its keymap outranks everything below it.
    vim(),
    // Presentation
    themeCompartment.of(themeVariables(options.theme ?? chinuuLight)),
    modeCompartment.of(viewModeExtensions(currentMode)),
    documentTheme,
    // Base theme, not a scoped one: the menu renders in `document.body`.
    menuTheme,
    EditorView.lineWrapping,
    // Editing behaviour
    ...behaviourExtensions,
    blinkCompartment.of(selectionExtensions(currentBlink)),
    // Mirrors vim's insert state into the editor state, which is what selects
    // line-granular reveal over the default node-granular one.
    vimModeTracking,
    // Markdown semantics and the feature set. All three sit behind compartments
    // so a live view can be replugged without rebuilding the document: a feature
    // can contribute parser support, extensions and decorations.
    markdownCompartment.of(
      markdownSupport(markdownSupportFor(currentFeatures)),
    ),
    featureCompartment.of(featureExtensionsOf(currentFeatures)),
    blockCompartment.of(blockDecorationsOf(currentFeatures)),
    menuCompartment.of(buildMenu()),
    // A note's image paths are relative to the note, which a webview cannot resolve
    // on its own. Without a resolver they are used verbatim.
    ...(options.resolveImage ? [imageResolver.of(options.resolveImage)] : []),
    // Related, but not the same: an image path can at least be fetched as-is, while a
    // wiki link's target means nothing until the host maps it.
    ...(options.resolveWikiLink
      ? [wikiLinkResolver.of(options.resolveWikiLink)]
      : []),
    // Input aids
    ...inputExtensions,
  ];

  const view = new EditorView({
    parent,
    state: EditorState.create({ doc: contents, extensions }),
  });

  // Vim's status-bar hook. The event is emitted on the CM5 compatibility object, not
  // through CM6 state, so there is nothing to read from a facet. `getCM` returns null
  // when the vim extension is not installed, hence the optional subscription.
  //
  // The payload is `{ mode, subMode }`, which is not vim's internal state, so it gets
  // its own type rather than borrowing `vimState` for something it does not describe.
  // `mode` is always present in the emissions this library makes.
  const { vimModeCallback } = options;
  const onVimModeChange = (event: { mode: string }): void => {
    vimModeCallback?.(event.mode);
  };
  const cm = getCM(view);
  cm?.on("vim-mode-change", onVimModeChange);

  // Vim emits its opening mode while the editor is still being constructed, before
  // this subscription exists, so without this a status bar would sit empty until the
  // first mode change.
  //
  // Measured: `state.vim.mode` is undefined even once construction has finished, so it
  // cannot be read back. `insertMode` is populated, so the answer is derived from that.
  // Only normal and insert are reachable here, since visual and replace both need a
  // keystroke, so they always arrive through the event instead.
  const vimAtStart = cm?.state?.vim as
    { mode?: string; insertMode?: boolean } | undefined;
  if (vimAtStart) {
    const mode =
      vimAtStart.mode ?? (vimAtStart.insertMode ? "insert" : "normal");
    onVimModeChange({ mode });
  }

  applyModeClass(view, currentMode);

  return {
    view,
    getValue: () => view.state.doc.toString(),
    setValue: (doc: string) => {
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: doc },
      });
    },
    setTheme: (theme: ChinuuTheme) => {
      view.dispatch({
        effects: themeCompartment.reconfigure(themeVariables(theme)),
      });
    },
    setFeatures: (next: readonly EditorFeature[]) => {
      currentFeatures = next;
      view.dispatch({
        effects: [
          markdownCompartment.reconfigure(
            markdownSupport(markdownSupportFor(next)),
          ),
          featureCompartment.reconfigure(featureExtensionsOf(next)),
          blockCompartment.reconfigure(blockDecorationsOf(next)),
          menuCompartment.reconfigure(buildMenu()),
        ],
      });
    },
    setMenu: (sections: readonly MenuSection[]) => {
      extraMenu = sections;
      view.dispatch({
        effects: menuCompartment.reconfigure(buildMenu()),
      });
    },
    setMode: (mode: ViewMode) => {
      currentMode = mode;
      applyModeClass(view, mode);
      view.dispatch({
        // Only the mode compartment changes. The feature set contributes the same
        // extensions in either mode, so there is nothing else to rebuild.
        effects: [modeCompartment.reconfigure(viewModeExtensions(mode))],
      });
    },
    setCursorBlink: (enabled: boolean) => {
      currentBlink = enabled;
      view.dispatch({
        effects: blinkCompartment.reconfigure(selectionExtensions(enabled)),
      });
    },
    destroy: () => {
      // Detach first: the handler closes over the caller's callback, and leaving it
      // attached would keep calling into a view that is being torn down.
      cm?.off("vim-mode-change", onVimModeChange);
      view.destroy();
    },
  };
};
