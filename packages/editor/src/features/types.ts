import type { Language, LanguageDescription } from "@codemirror/language";
import type { Extension } from "@codemirror/state";
import type { MarkdownExtension } from "@lezer/markdown";
import type { DecorationRule } from "../livePreview/types.ts";
import type { MenuSection } from "../menu/types.ts";

/**
 * How a fenced code block's info string resolves to a language.
 *
 * Matches `markdown()`'s `codeLanguages` option exactly: either a list of
 * descriptors, or a resolver returning a description or language (note: *not*
 * `LanguageSupport`, which is what I first assumed).
 */
export type CodeLanguages =
  | readonly LanguageDescription[]
  | ((info: string) => LanguageDescription | Language | null);

/**
 * A feature is the unit of plug/unplug.
 *
 * It bundles everything one capability needs, so taking it out of the feature
 * list takes out all of it: parser support, editor extensions and decorations.
 * A new capability becomes a new feature rather than another branch inside the
 * preview plugin.
 */
export interface EditorFeature {
  /** Stable identifier, used for logging and for feature-set selection. */
  readonly id: string;

  /** Parser extensions, merged into `markdown({ extensions })`. */
  readonly markdown?: MarkdownExtension;

  /**
   * Fenced-code language resolution, merged into `markdown({ codeLanguages })`.
   * Only one feature in a set can meaningfully provide this.
   */
  readonly codeLanguages?: CodeLanguages;

  /** CM6 extensions: keymaps, event handlers, extra view plugins. */
  readonly extensions?: readonly Extension[];

  /** Decoration rules. See `DecorationRule.scope` for where each one is hosted. */
  readonly rules?: readonly DecorationRule[];

  /**
   * Right-click menu sections. Sections are concatenated in feature order, so a
   * feature owns the grouping inside its own contribution.
   */
  readonly menu?: readonly MenuSection[];
}
