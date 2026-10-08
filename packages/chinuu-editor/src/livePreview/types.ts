import type { Decoration } from "@codemirror/view";
import type { SyntaxNodeRef } from "@lezer/common";

/** A node's name and span, used for the reveal test. */
export interface NodeSpan {
  readonly name: string;
  readonly from: number;
  readonly to: number;
}

/** A line's number, span and text. */
export interface LineInfo {
  readonly number: number;
  readonly from: number;
  readonly to: number;
  readonly text: string;
}

/**
 * What a rule receives for each node it declared interest in.
 *
 * Deliberately narrow no access to the whole `EditorState`. Everything a rule
 * needs arrives resolved, so rules stay small and can be read on their own.
 */
export interface RuleContext {
  /** The matched node, for rules that need to walk its children. */
  readonly node: SyntaxNodeRef;
  readonly name: string;
  readonly from: number;
  readonly to: number;
  /** Enclosing node, for rules that reveal a construct rather than a single token. */
  readonly parent: NodeSpan | undefined;
  /** Verbatim document text of this node. */
  readonly text: string;
  /** The line containing `pos`. */
  readonly lineAt: (pos: number) => LineInfo;
  /**
   * Whether the construct at `[from, to]` should reveal its markdown source.
   *
   * True when a selection range touches that span *and* the view mode permits
   * revealing. Always false in reading mode, which is what keeps a rendered view
   * rendered so rules never need to know about view modes.
   */
  readonly reveals: (from: number, to: number) => boolean;
  /** Append a decoration. */
  readonly add: (decoration: Decoration, from: number, to: number) => void;
  /** Append a line class at the start of `line`, de-duplicated. */
  readonly addLineClass: (line: number, className: string) => void;
  /**
   * Append a line decoration at the start of `line`.
   *
   * Not de-duplicated, unlike `addLineClass`: use it for per-line chrome that one
   * rule owns, such as a line-number attribute.
   */
  readonly addLine: (line: number, decoration: Decoration) => void;
}

/**
 * A decoration rule the unit of live-preview behaviour.
 *
 * Rules declare the node names they handle so the plugin can dispatch by name
 * during a single tree walk. Rules never walk the tree themselves, so adding a
 * feature costs a map lookup rather than a second traversal.
 */
export interface DecorationRule {
  /** Stable identifier, for debugging and per-feature toggling. */
  readonly id: string;
  /** Node names this rule runs for. */
  readonly nodes: readonly string[];
  /**
   * Which decoration source may carry this rule.
   *
   * `"block"` routes it through a state field instead of the view plugin, which
   * is required for block widgets and for replacements that span a line break
   * CM6 throws on both when they come from a plugin. Defaults to `"inline"`.
   */
  readonly scope?: "inline" | "block";
  build(context: RuleContext): void;
}
