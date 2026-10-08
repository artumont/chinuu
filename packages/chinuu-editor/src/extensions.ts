import {
  autocompletion,
  closeBrackets,
  closeBracketsKeymap,
  completionKeymap,
} from "@codemirror/autocomplete";
import {
  defaultKeymap,
  history,
  historyKeymap,
  indentWithTab,
} from "@codemirror/commands";
import {
  markdown,
  markdownKeymap,
  markdownLanguage,
} from "@codemirror/lang-markdown";
import { bracketMatching, syntaxHighlighting } from "@codemirror/language";
import { searchKeymap } from "@codemirror/search";
import { EditorState, type Extension } from "@codemirror/state";
import {
  drawSelection,
  dropCursor,
  highlightActiveLine,
  highlightSpecialChars,
  keymap,
  rectangularSelection,
} from "@codemirror/view";
import type { MarkdownExtension } from "@lezer/markdown";
import type { CodeLanguages } from "./features/types.ts";
import { syntaxHighlightingStyle } from "./theme/index.ts";

export interface MarkdownSupportConfig {
  readonly extensions: readonly MarkdownExtension[];
  readonly codeLanguages?: CodeLanguages;
}

/**
 * Markdown language support, with the parser extensions and fenced-code language
 * resolution the feature set supplies.
 *
 * `markdown()` alone is strict CommonMark, where `[x] task` parses as a *link*;
 * `markdownLanguage` adds GFM.
 *
 * Both optional keys are spread conditionally, so a feature set contributing
 * neither produces exactly the extension this module produced before the feature
 * system existed.
 */
export const markdownSupport = ({
  extensions,
  codeLanguages,
}: MarkdownSupportConfig): Extension =>
  markdown({
    base: markdownLanguage,
    ...(extensions.length > 0 ? { extensions: [...extensions] } : {}),
    ...(codeLanguages ? { codeLanguages } : {}),
  });

/**
 * Editing behaviour, in the order it was originally composed.
 *
 * Spelled out instead of using `basicSetup`, which is a code-editor preset: line
 * numbers, two gutters, fold markers, crosshair cursor. This is a document, so
 * those are out.
 */
export const behaviourExtensions: readonly Extension[] = [
  highlightSpecialChars(),
  history(),
  // `drawSelection()` lives in `selectionExtensions` instead, because the blink
  // rate is a `drawSelection` option and has to be reconfigurable.
  dropCursor(),
  EditorState.allowMultipleSelections.of(true),
  rectangularSelection(),
  highlightActiveLine(),
];

/**
 * Selection drawing, including the caret.
 *
 * `cursorBlinkRate` is a `drawSelection` option rather than a facet, and the
 * documented value that disables blinking is `0`. Passing no config at all when
 * blinking is wanted leaves CM6's own 1200ms default in place.
 */
export const selectionExtensions = (cursorBlink: boolean): Extension =>
  cursorBlink ? drawSelection() : drawSelection({ cursorBlinkRate: 0 });

/** Styling and input aids. Trails the feature set so its keymaps lose ties. */
export const inputExtensions: readonly Extension[] = [
  syntaxHighlighting(syntaxHighlightingStyle),
  bracketMatching(),
  closeBrackets(),
  autocompletion(),
  keymap.of([
    // Nothing bound Tab before this, so the browser's default ran: focus moved
    // to the next focusable element and the caret went with it. `indentWithTab`
    // is `{key: "Tab", run: indentMore, shift: indentLess}` indentation, not a
    // literal tab character.
    indentWithTab,
    // Before `defaultKeymap`: both bind Enter, and the first binding to return
    // true wins, so markdown's list/quote continuation has to come first or
    // `defaultKeymap`'s plain newline swallows it.
    ...markdownKeymap,
    ...closeBracketsKeymap,
    ...defaultKeymap,
    ...searchKeymap,
    ...historyKeymap,
    ...completionKeymap,
  ]),
];
