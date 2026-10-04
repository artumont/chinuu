import { defaultFonts, type ChinuuTheme } from "../../tokens.ts";

/**
 * Tokyo Night **Storm**.
 *
 * This is the base of the dark Tokyo Night variants: upstream
 * `folke/tokyonight.nvim` defines `colors/storm.lua` as the full palette and
 * `colors/night.lua` extends it, overriding only `bg`, `bg_dark` and `bg_dark1`.
 * `night.ts` mirrors that relationship.
 *
 * Values verified against two independent sources:
 * - `folke/tokyonight.nvim` `colors/storm.lua` named palette (`bg`, `fg`,
 *   `fg_dark`, `fg_gutter`, `bg_highlight`, `bg_dark`, `comment`, `blue`)
 * - `enkia/tokyo-night-vscode-theme` "Tokyo Night Storm" UI values
 *   (`editor.background`, `editor.foreground`, `editorCursor.foreground`,
 *   `editor.selectionBackground`, `editor.lineHighlightBackground`,
 *   `editorLineNumber.foreground`, `editorHoverWidget.background`)
 *
 * Both sources agree on `#24283b` (background), `#292e42` (active line) and
 * `#3b4261` (borders).
 *
 * Palette: MIT, (c) folke and Enkia.
 */
export const tokyoNightStorm: ChinuuTheme = {
  name: "Tokyo Night Storm",
  dark: true,
  ...defaultFonts,
  background: "#24283b", // storm bg both sources
  foreground: "#c0caf5", // fg
  caret: "#c0caf5", // editorCursor.foreground
  selection: "#6f7bb640", // editor.selectionBackground
  activeLine: "#292e42", // bg_highlight both sources
  mark: "#565f89", // comment
  headingMuted: "#565f89",
  link: "#7aa2f7", // blue
  strikethrough: "#a9b1d6", // fg_dark
  quoteText: "#a9b1d6",
  quoteBorder: "#3b4261", // fg_gutter both sources
  codeBackground: "#1f2335", // bg_dark
  codeForeground: "#c0caf5",
  inlineCodeBackground: "#1f2335",
  inlineCodeBorder: "#3b4261",
  tableBackground: "#292e42", // bg_highlight
  tableForeground: "#a9b1d6",
  rule: "#3b4261", // fg_gutter
  // Code syntax. Keyword/string/number/comment/function are the colours enkia's
  // dark theme assigns to `storage.modifier`, `string`, `constant.numeric`,
  // `comment` and `entity.name.function`, and they match folke's palette
  // (`purple`, `green`, `orange`, `comment`, `blue`).
  codeKeyword: "#9d7cd8", // purple
  codeString: "#9ece6a", // green
  codeNumber: "#ff9e64", // orange
  codeComment: "#565f89", // comment
  codeFunction: "#7aa2f7", // blue
  // Assigned from the palette rather than extracted: enkia's `support.class`
  // colour was not among the scopes I read, so this is my own mapping of
  // folke's `cyan` onto types.
  codeType: "#7dcfff", // cyan
  // Menu surfaces. `bg_highlight` and `fg_gutter` from folke's palette. Inherited
  // by `night`, which only overrides its background set the separation from
  // `#1a1b26` is still large enough to read as a raised panel.
  menuBackground: "#292e42", // bg_highlight
  menuBorder: "#3b4261", // fg_gutter
  menuHighlight: "#3b4261", // fg_gutter
};
