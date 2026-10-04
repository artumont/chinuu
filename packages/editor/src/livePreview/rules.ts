import { syntaxTree } from "@codemirror/language";
import type { EditorState } from "@codemirror/state";
import { Decoration, type DecorationSet } from "@codemirror/view";
import { createCollector, createRuleContext } from "./context.ts";
import type { DecorationRule } from "./types.ts";

/** Node name → the rules that declared interest in it. */
export type RuleIndex = ReadonlyMap<string, readonly DecorationRule[]>;

/**
 * Built once per feature set, not per keystroke, so the per-update cost is a map
 * lookup per matching node plus whatever the rules themselves do.
 */
export const indexRules = (rules: readonly DecorationRule[]): RuleIndex => {
  const byNode = new Map<string, DecorationRule[]>();
  for (const rule of rules) {
    for (const name of rule.nodes) {
      const existing = byNode.get(name);
      if (existing) existing.push(rule);
      else byNode.set(name, [rule]);
    }
  }
  return byNode;
};

/** One tree walk, dispatching to every rule by node name. */
export const buildDecorations = (
  state: EditorState,
  index: RuleIndex,
): DecorationSet => {
  const collector = createCollector(state);

  syntaxTree(state).iterate({
    enter: (node) => {
      const matched = index.get(node.name);
      if (!matched) return;
      const context = createRuleContext(state, node, collector);
      for (const rule of matched) rule.build(context);
    },
  });

  return Decoration.set(collector.ranges, true);
};

/**
 * Lezer parses incrementally off the main path, so the tree can change without
 * the document or selection changing. Both decoration sources need this check or
 * they stay stale until the next keystroke after a parse lands.
 */
export const treeChanged = (before: EditorState, after: EditorState): boolean =>
  syntaxTree(before) !== syntaxTree(after);

/** The rules that may only be provided from a state field, per CM6. */
export const isBlockRule = (rule: DecorationRule): boolean =>
  rule.scope === "block";
