import type { Extension } from "@codemirror/state";
import type { MarkdownExtension } from "@lezer/markdown";
import type { MarkdownSupportConfig } from "../extensions.ts";
import { isBlockRule } from "../livePreview/rules.ts";
import type { DecorationRule } from "../livePreview/types.ts";
import type { MenuSection } from "../menu/types.ts";
import { blockClassesFeature } from "./blockClasses.ts";
import { codeBlocksFeature } from "./codeBlocks.ts";
import { formatFeature } from "./format/index.ts";
import { horizontalRuleFeature } from "./horizontalRule.ts";
import { imagesFeature } from "./images.ts";
import { wikiLinksFeature } from "./wikiLinks.ts";
import { inlineCodeFeature } from "./inlineCode.ts";
import { insertArrowFeature } from "./insertArrows.ts";
import { linksFeature } from "./links.ts";
import { markersFeature } from "./markers.ts";
import { mathFeature } from "./math/index.ts";
import { tablesFeature } from "./tables.ts";
import { taskListsFeature } from "./taskLists.ts";
import type { EditorFeature } from "./types.ts";

/**
 * The built-in features, addressable by id.
 *
 * This is the plug/unplug surface: `initEditor` takes any list of these, so an
 * editor without rendered tables is the same list minus `tables`, and a new
 * capability is a new entry here rather than a change to the preview plugin.
 */
export const builtinFeatures = {
  markers: markersFeature,
  "block-classes": blockClassesFeature,
  "inline-code": inlineCodeFeature,
  "horizontal-rule": horizontalRuleFeature,
  "code-blocks": codeBlocksFeature,
  math: mathFeature,
  tables: tablesFeature,
  "task-lists": taskListsFeature,
  format: formatFeature,
  "insert-arrows": insertArrowFeature,
  links: linksFeature,
  images: imagesFeature,
  "wiki-links": wikiLinksFeature,
} as const;

export const defaultFeatures: readonly EditorFeature[] =
  Object.values(builtinFeatures);

/** Every decoration rule the feature set asks for. */
export const rulesFor = (
  features: readonly EditorFeature[],
): DecorationRule[] => features.flatMap((feature) => feature.rules ?? []);

/** Rules the view plugin hosts: marks, line classes, single-line replacements. */
export const inlineRulesFor = (
  features: readonly EditorFeature[],
): DecorationRule[] => rulesFor(features).filter((rule) => !isBlockRule(rule));

/**
 * Rules the state field hosts: block widgets and replacements that may span a
 * line break. CM6 rejects both from a view plugin.
 */
export const blockRulesFor = (
  features: readonly EditorFeature[],
): DecorationRule[] => rulesFor(features).filter(isBlockRule);

/** CM6 extensions the feature set asks for. */
export const extensionsFor = (
  features: readonly EditorFeature[],
): Extension[] => features.flatMap((feature) => feature.extensions ?? []);

/** Right-click menu sections, concatenated in feature order. */
export const menuSectionsFor = (
  features: readonly EditorFeature[],
): MenuSection[] => features.flatMap((feature) => feature.menu ?? []);

/** Everything `markdownSupport` needs, gathered from the feature set. */
export const markdownSupportFor = (
  features: readonly EditorFeature[],
): MarkdownSupportConfig => ({
  extensions: features.flatMap<MarkdownExtension>(
    (feature) => feature.markdown ?? [],
  ),
  // Only one feature can meaningfully resolve code languages; first one wins.
  codeLanguages: features.find((feature) => feature.codeLanguages)
    ?.codeLanguages,
});
