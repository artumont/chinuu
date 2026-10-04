import type { ChangeSpec, EditorState } from "@codemirror/state";
import { EditorSelection } from "@codemirror/state";
import type { EditorView } from "@codemirror/view";

/**
 * The range a formatting command acts on.
 *
 * Selection when there is one, otherwise the line under the caret which is
 * what makes the same menu item work for "bold this word" and "comment this
 * line" without the user thinking about it.
 */
export interface FormatTarget {
  readonly from: number;
  readonly to: number;
  readonly hasSelection: boolean;
  readonly firstLine: number;
  readonly lastLine: number;
}

export const targetOf = (state: EditorState): FormatTarget => {
  const range = state.selection.main;
  const hasSelection = !range.empty;
  const from = hasSelection ? range.from : state.doc.lineAt(range.from).from;
  const to = hasSelection ? range.to : state.doc.lineAt(range.from).to;
  return {
    from,
    to,
    hasSelection,
    firstLine: state.doc.lineAt(from).number,
    lastLine: state.doc.lineAt(to).number,
  };
};

/**
 * Wraps or unwraps the target in `marker`.
 *
 * Three cases, because all three are things people actually do:
 * - nothing selected → insert the pair and leave the caret between them
 * - the whole `**bold**` selected → strip the inner markers
 * - markers sitting just outside the selection → remove them, which is the
 *   "select the word and unbold it" case
 */
export const toggleWrap = (view: EditorView, marker: string): void => {
  view.dispatch(
    view.state.changeByRange((range) => {
      const { state } = view;

      if (range.empty) {
        return {
          changes: { from: range.from, insert: marker + marker },
          range: EditorSelection.cursor(range.from + marker.length),
        };
      }

      const text = state.sliceDoc(range.from, range.to);
      if (
        text.length > marker.length * 2 &&
        text.startsWith(marker) &&
        text.endsWith(marker)
      ) {
        const inner = text.slice(marker.length, -marker.length);
        return {
          changes: { from: range.from, to: range.to, insert: inner },
          range: EditorSelection.range(range.from, range.from + inner.length),
        };
      }

      const before = state.sliceDoc(
        Math.max(0, range.from - marker.length),
        range.from,
      );
      const after = state.sliceDoc(
        range.to,
        Math.min(state.doc.length, range.to + marker.length),
      );
      if (before === marker && after === marker) {
        return {
          changes: [
            { from: range.from - marker.length, to: range.from },
            { from: range.to, to: range.to + marker.length },
          ],
          range: EditorSelection.range(
            range.from - marker.length,
            range.to - marker.length,
          ),
        };
      }

      return {
        changes: [
          { from: range.from, insert: marker },
          { from: range.to, insert: marker },
        ],
        range: EditorSelection.range(
          range.from + marker.length,
          range.to + marker.length,
        ),
      };
    }),
  );
  view.focus();
};

/** Every block prefix a menu item might replace. Longest forms first. */
const BLOCK_PREFIXES: readonly RegExp[] = [
  /^(#{1,6})[ \t]+/,
  /^([-*+])[ \t]+\[[ xX]\][ \t]+/,
  /^([-*+])[ \t]+/,
  /^(\d+)\.[ \t]+/,
  /^(>)[ \t]+/,
];

export interface LineParts {
  readonly indent: string;
  /** The block prefix already on the line, or `""`. */
  readonly prefix: string;
  readonly content: string;
}

/** Splits a line into indentation, existing block prefix, and content. */
export const splitLine = (text: string): LineParts => {
  const indent = /^[ \t]*/.exec(text)?.[0] ?? "";
  const rest = text.slice(indent.length);
  for (const pattern of BLOCK_PREFIXES) {
    const match = pattern.exec(rest);
    if (match) {
      return {
        indent,
        prefix: match[0].trimEnd(),
        content: rest.slice(match[0].length),
      };
    }
  }
  return { indent, prefix: "", content: rest };
};

/**
 * Adds, replaces or removes a block prefix across the target lines.
 *
 * Applying the prefix a line already has removes it, so the menu item toggles.
 * Any *other* block prefix is replaced rather than stacked, so a heading does
 * not become a bulleted heading.
 */
export const applyLinePrefix = (view: EditorView, prefix: string): void => {
  const target = targetOf(view.state);
  const changes: ChangeSpec[] = [];

  for (let number = target.firstLine; number <= target.lastLine; number++) {
    const line = view.state.doc.line(number);
    const { indent, prefix: existing, content } = splitLine(line.text);
    const next =
      existing === prefix.trimEnd()
        ? indent + content
        : indent + prefix + content;
    changes.push({ from: line.from, to: line.to, insert: next });
  }

  const set = view.state.changes(changes);
  view.dispatch({
    changes: set,
    // `assoc: 1` keeps the caret with the text instead of parking it before the
    // prefix that was just inserted.
    selection: view.state.selection.map(set, 1),
  });
  view.focus();
};

const COMMENT_OPEN = "<!--";
const COMMENT_CLOSE = "-->";

/**
 * Wraps the target in an HTML comment, the portable way to comment markdown.
 *
 * `<!-- -->` is used rather than Obsidian's `%%…%%` because it survives every
 * renderer and is already valid in CommonMark and GFM.
 */
export const toggleComment = (view: EditorView): void => {
  const target = targetOf(view.state);
  const text = view.state.sliceDoc(target.from, target.to);
  const lead = text.length - text.trimStart().length;
  const trail = text.length - text.trimEnd().length;
  const start = text.slice(lead);
  const end = trail ? text.slice(0, -trail) : text;

  const insert =
    start.startsWith(COMMENT_OPEN) && end.endsWith(COMMENT_CLOSE)
      ? text
          .slice(
            lead + COMMENT_OPEN.length,
            text.length - trail - COMMENT_CLOSE.length,
          )
          .trim()
      : `${COMMENT_OPEN} ${text} ${COMMENT_CLOSE}`;

  const set = view.state.changes({ from: target.from, to: target.to, insert });
  view.dispatch({ changes: set, selection: view.state.selection.map(set, 1) });
  view.focus();
};

/**
 * Turns the target into `[text](url)` and selects the `url` placeholder, so the
 * next thing typed replaces it.
 */
export const insertLink = (view: EditorView): void => {
  const target = targetOf(view.state);
  const text = view.state.sliceDoc(target.from, target.to);
  const insert = `[${text}](url)`;
  // past "[", the text, "]("
  const urlStart = target.from + text.length + 3;

  view.dispatch({
    changes: { from: target.from, to: target.to, insert },
    selection: EditorSelection.range(urlStart, urlStart + 3),
  });
  view.focus();
};

/** Inserts a `---` rule on its own line after the target. */
export const insertHorizontalRule = (view: EditorView): void => {
  const target = targetOf(view.state);
  const line = view.state.doc.line(target.lastLine);
  // Avoid stacking blank lines when the target is already empty.
  const insert = `${line.text.trim() === "" ? "" : "\n"}---\n`;
  const set = view.state.changes({ from: line.to, insert });
  view.dispatch({ changes: set, selection: view.state.selection.map(set, 1) });
  view.focus();
};

/**
 * Wraps the target lines in a fenced code block, or removes the fences if the
 * target is already inside one.
 */
export const toggleFence = (view: EditorView, language = ""): void => {
  const { doc } = view.state;
  const target = targetOf(view.state);
  const first = doc.line(target.firstLine);
  const last = doc.line(target.lastLine);
  const before = first.number > 1 ? doc.line(first.number - 1) : null;
  const after = last.number < doc.lines ? doc.line(last.number + 1) : null;

  // Each removal takes the line's own newline with it, except on the last line
  // where there is none to take.
  const wholeLine = (line: { from: number; to: number; number: number }) => ({
    from: line.from,
    to: line.number < doc.lines ? line.to + 1 : line.to,
  });

  if (
    before &&
    after &&
    /^\s*```/.test(before.text) &&
    /^\s*```\s*$/.test(after.text)
  ) {
    const set = view.state.changes([wholeLine(before), wholeLine(after)]);
    view.dispatch({
      changes: set,
      selection: view.state.selection.map(set, 1),
    });
    view.focus();
    return;
  }

  const set = view.state.changes([
    { from: first.from, insert: "```" + language + "\n" },
    { from: last.to, insert: "\n```" },
  ]);
  view.dispatch({ changes: set, selection: view.state.selection.map(set, 1) });
  view.focus();
};

/** Inserts a 2x2 GFM table after the target line. */
export const insertTable = (view: EditorView): void => {
  const target = targetOf(view.state);
  const line = view.state.doc.line(target.lastLine);
  const insert = `\n|     |     |\n| --- | --- |\n|     |     |`;
  const set = view.state.changes({ from: line.to, insert });
  view.dispatch({ changes: set, selection: view.state.selection.map(set, 1) });
  view.focus();
};
