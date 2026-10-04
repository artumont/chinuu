import type { EditorState, Range } from "@codemirror/state";
import type { Decoration } from "@codemirror/view";
import type { SyntaxNodeRef } from "@lezer/common";
import { revealsSource } from "../viewMode.ts";
import { lineClass } from "./decorations.ts";
import type { NodeSpan, RuleContext } from "./types.ts";

/**
 * Accumulates the decorations every rule produces for one document version.
 *
 * Line classes are de-duplicated by `(position, class)`: nested blocks a fence
 * inside a quote have two rules writing to the same line, and a set with exact
 * duplicates is both wasteful and easy to get subtly wrong.
 */
export interface Collector {
  readonly ranges: Range<Decoration>[];
  readonly add: (decoration: Decoration, from: number, to: number) => void;
  readonly addLineClass: (line: number, className: string) => void;
  readonly addLine: (line: number, decoration: Decoration) => void;
}

export const createCollector = (state: EditorState): Collector => {
  const ranges: Range<Decoration>[] = [];
  const seenLines = new Set<string>();

  return {
    ranges,
    add: (decoration, from, to) => {
      ranges.push(decoration.range(from, to));
    },
    addLineClass: (line, className) => {
      const pos = state.doc.line(line).from;
      const key = `${pos}:${className}`;
      if (seenLines.has(key)) return;
      seenLines.add(key);
      ranges.push(lineClass(className).range(pos));
    },
    addLine: (line, decoration) => {
      ranges.push(decoration.range(state.doc.line(line).from));
    },
  };
};

export const createRuleContext = (
  state: EditorState,
  node: SyntaxNodeRef,
  collector: Collector,
): RuleContext => {
  const parentNode = node.node.parent;
  const parent: NodeSpan | undefined = parentNode
    ? { name: parentNode.name, from: parentNode.from, to: parentNode.to }
    : undefined;

  return {
    node,
    name: node.name,
    from: node.from,
    to: node.to,
    parent,
    // A getter, not an eager slice: block rules match whole code blocks and
    // tables, and most of them never look at the text at all.
    get text(): string {
      return state.doc.sliceString(node.from, node.to);
    },
    lineAt: (pos) => {
      const line = state.doc.lineAt(pos);
      return {
        number: line.number,
        from: line.from,
        to: line.to,
        text: line.text,
      };
    },
    // The mode check lives here rather than in each rule, so adding a mode or a
    // rule never means touching the other.
    reveals: (from, to) => {
      if (!revealsSource(state)) return false;

      const first = state.doc.lineAt(from).number;
      const last = state.doc.lineAt(to).number;

      // Any caret on a line the construct touches reveals it, so a construct is
      // unrendered only while the caret is elsewhere.
      //
      // There is deliberately no lookahead to the following line. An earlier
      // version revealed one line ahead in insert mode, so that a downward move
      // would not land past hidden markup. It did land on column 0 as intended,
      // but it left the whole next line unrendered bullets showing `-`,
      // checkboxes showing `[ ]` because *every* construct on that line was
      // revealed, not just the one being approached. That is a far louder defect
      // than the column drift it was fixing.
      return state.selection.ranges.some((range) => {
        const caret = state.doc.lineAt(range.from).number;
        return caret >= first && caret <= last;
      });
    },
    add: collector.add,
    addLineClass: collector.addLineClass,
    addLine: collector.addLine,
  };
};
