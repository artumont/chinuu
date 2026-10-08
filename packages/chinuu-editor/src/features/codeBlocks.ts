import { LanguageDescription, syntaxTree } from "@codemirror/language";
import { languages } from "@codemirror/language-data";
import { Decoration, EditorView, WidgetType } from "@codemirror/view";
import { CLASS } from "../classes.ts";
import type { DecorationRule } from "../livePreview/types.ts";
import type { EditorFeature } from "./types.ts";

/**
 * Fenced code blocks.
 *
 * Highlighting is `markdown({ codeLanguages: languages })`, i.e. the nested
 * language parser from `@codemirror/language-data` the same library MDXEditor's
 * code block uses for its embedded editor.
 *
 * What MDXEditor's node adds on top of that is that a code block's language is a
 * *field* with a picker in front of it, rather than the fence info string being
 * opaque text. That part is portable and is what this feature contributes; the
 * node itself is a Lexical `DecoratorNode` with React and Radix UI, which cannot
 * be transplanted into a CodeMirror host. Nor is its embedded CodeMirror editor
 * wanted here: this host already parses fenced code natively, so a nested editor
 * would add a second scroll container and a second focus target for no gain.
 */

/** Sorted once: ~100 entries, and the browser's type-ahead covers the rest. */
const choices: readonly LanguageDescription[] = [...languages].sort((a, b) =>
  a.name.localeCompare(b.name),
);

/**
 * What gets written into the fence: the short alias when the language has one.
 * `markdown()` resolves an info string through `LanguageDescription.matchLanguageName`,
 * which matches name *or* alias, so `ts` and `TypeScript` both resolve.
 */
const fenceName = (description: LanguageDescription): string =>
  description.alias?.[0] ?? description.name.toLowerCase();

class LanguagePickerWidget extends WidgetType {
  constructor(private readonly language: string) {
    super();
  }

  eq(other: LanguagePickerWidget): boolean {
    return other.language === this.language;
  }

  /**
   * Let the event through. The default is `true`, which makes CM6 ignore events
   * targeting the widget and with it the `change` event this picker depends on.
   * Measured: the picker rendered and reflected the current language, but choosing
   * one never reached the handler and the fence never changed.
   */
  ignoreEvent(): boolean {
    return false;
  }

  toDOM(): HTMLElement {
    const select = document.createElement("select");
    select.className = CLASS.codeLanguage;
    select.setAttribute("aria-label", "Code block language");
    select.title = "Code block language";

    const plain = document.createElement("option");
    plain.value = "";
    plain.textContent = "Plain text";
    select.appendChild(plain);

    for (const description of choices) {
      const option = document.createElement("option");
      // LanguageDescription has no stable id: the name is what `matchLanguageName`
      // resolves against, so it is what the option carries.
      option.value = description.name;
      option.textContent = description.name;
      select.appendChild(option);
    }

    // Resolve the fence text to an option the same way the parser does, so the
    // picker reflects what is actually being highlighted.
    const matched = this.language
      ? LanguageDescription.matchLanguageName(choices, this.language)
      : null;
    select.value = matched?.name ?? "";

    // Keep the mousedown away from the content DOM: CM6 would otherwise move the
    // caret and pull focus out of the select before the dropdown opens.
    select.addEventListener("mousedown", (event) => event.stopPropagation());
    return select;
  }
}

/** Position just after the opening fence run, for a fence with no info string. */
const afterFence = (lineText: string, lineFrom: number): number => {
  const fence = /^(\s*)(`{3,}|~{3,})/.exec(lineText);
  return lineFrom + (fence ? fence[0].length : 3);
};

/**
 * A language picker on each fence's opening line.
 *
 * A point widget after the fence rather than a replacement of it: `markers`
 * already replaces `CodeMark` and `CodeInfo`, and two replace decorations over the
 * same range overlap.
 */
const pickerRule: DecorationRule = {
  id: "code-language",
  nodes: ["FencedCode"],

  build({ from, to, node, reveals, lineAt, add, addLineClass }) {
    // Reveal the fence while the caret is inside the block, or (in insert mode) on
    // the line above, matching every other construct.
    if (reveals(from, to)) return;

    const opener = lineAt(from);
    // `position: relative` on the opening line, so the picker can be anchored to
    // the block's top-right corner rather than sitting inline after the fence.
    addLineClass(opener.number, CLASS.codeOpen);

    const info = node.node.getChild("CodeInfo");
    const fence = /^(\s*)(`{3,}|~{3,})(.*)$/.exec(opener.text);
    const language = fence ? fence[3].trim() : "";

    const at = info ? info.to : afterFence(opener.text, opener.from);
    add(
      Decoration.widget({
        widget: new LanguagePickerWidget(language),
        side: 1,
      }),
      at,
      at,
    );
  },
};

/**
 * Line numbers down the left of each block, restarting at 1.
 *
 * Drawn from a `data-code-line` attribute via a `::before` pseudo-element rather
 * than as gutter widgets: pseudo-element content stays out of the selection and
 * the clipboard, so copying a block does not pick the numbers up.
 */
const lineNumbersRule: DecorationRule = {
  id: "code-line-numbers",
  nodes: ["FencedCode"],

  build({ from, to, node, reveals, lineAt, addLine }) {
    // While the fence is revealed there is no gutter: it is source text then.
    if (reveals(from, to)) return;

    const text = node.node.getChild("CodeText");
    if (!text || text.to <= text.from) return;

    const first = lineAt(text.from).number;
    const last = lineAt(text.to).number;

    for (let number = first; number <= last; number++) {
      addLine(
        number,
        Decoration.line({
          class: CLASS.codeLine,
          attributes: { "data-code-line": String(number - first + 1) },
        }),
      );
    }
  },
};

/** Writes the chosen language into the fence's info string. */
const setFenceLanguage = (
  view: EditorView,
  pos: number,
  language: string,
): boolean => {
  const state = view.state;
  const edits: { from: number; to: number }[] = [];

  syntaxTree(state).iterate({
    enter(node) {
      if (edits.length > 0 || node.name !== "FencedCode") return;
      if (pos < node.from || pos > node.to) return;
      const info = node.node.getChild("CodeInfo");
      if (info) {
        edits.push({ from: info.from, to: info.to });
        return;
      }
      const line = state.doc.lineAt(node.from);
      const at = afterFence(line.text, line.from);
      edits.push({ from: at, to: at });
    },
  });

  if (edits.length === 0) return false;
  const [edit] = edits;
  view.dispatch({
    changes: { from: edit.from, to: edit.to, insert: language },
  });
  return true;
};

const languagePicks = EditorView.domEventHandlers({
  change(event, view) {
    const target = event.target;
    if (!(target instanceof HTMLSelectElement)) return false;
    if (!target.classList.contains(CLASS.codeLanguage)) return false;

    const description = LanguageDescription.matchLanguageName(
      choices,
      target.value,
    );
    // `posAtDOM` is resolved live, so it stays right even when CM6 reuses the
    // element for an unchanged widget.
    return setFenceLanguage(
      view,
      view.posAtDOM(target),
      description ? fenceName(description) : "",
    );
  },
});

export const codeBlocksFeature: EditorFeature = {
  id: "code-blocks",
  codeLanguages: languages,
  rules: [pickerRule, lineNumbersRule],
  extensions: [languagePicks],
};
