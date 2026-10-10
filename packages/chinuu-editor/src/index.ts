/**
 * Public surface of `@chinuu/editor`.
 *
 * Internal modules are importable directly if you need them, but everything
 * needed to embed the editor, theme it, or author a feature is re-exported here.
 */

export { initEditor } from "./editor.ts";
export type { EditorHandle, EditorOptions } from "./editor.ts";

export { builtinFeatures, defaultFeatures } from "./features/index.ts";
export { imageResolver, type ImageResolver } from "./features/images.ts";
export {
  tauriImageResolver,
  type TauriImageResolverOptions,
} from "./features/tauriImages.ts";
export {
  fileLinkOpener,
  fileLinkResolver,
  type FileLinkOpener,
  type FileLinkResolver,
} from "./features/links.ts";
export {
  wikiLinkOpener,
  wikiLinkResolver,
  type WikiLinkOpener,
  type WikiLinkResolver,
} from "./features/wikiLinks.ts";
export type { EditorFeature } from "./features/types.ts";

// Feature authors need these: a rule plus the decoration helpers to build one.
export { hidden, lineClass, markClass } from "./livePreview/decorations.ts";
export type { DecorationRule, RuleContext } from "./livePreview/types.ts";
export { CLASS, HEADING_CLASSES } from "./classes.ts";

// Menu pieces, for adding your own items alongside the built-in format ones.
export { contextMenu } from "./menu/contextMenu.ts";
export type { MenuContext, MenuItem, MenuSection } from "./menu/types.ts";
export {
  applyLinePrefix,
  insertHorizontalRule,
  insertLink,
  insertTable,
  splitLine,
  targetOf,
  toggleComment,
  toggleFence,
  toggleWrap,
} from "./features/format/commands.ts";

export {
  builtinThemes,
  chinuuLight,
  chinuuDark,
  tokyoNight,
  tokyoNightLight,
  tokyoNightStorm,
  tokyoNightVariants,
  themeVariables,
  TOKEN_NAMES,
  type ChinuuTheme,
  type ThemeTokens,
} from "./theme/index.ts";
