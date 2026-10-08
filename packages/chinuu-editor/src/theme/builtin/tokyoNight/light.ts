import type { ChinuuTheme } from "../../tokens.ts";

/**
 * Tokyo Night **Light**.
 *
 * Upstream divergence worth knowing about: `folke/tokyonight.nvim` has no
 * concrete light palette. `colors/day.lua` is a *function* that deepcopies the
 * active dark style and calls `Util.invert()`, so there are no fixed hex values
 * to copy. The concrete values therefore come from
 * `enkia/tokyo-night-vscode-theme` "Tokyo Night Light".
 *
 * Two deliberate departures from that file:
 *
 * 1. `dark: false`. The upstream JSON declares `"type": "dark"`, which looks
 *    like a mistake in a light theme. CM6 needs the real value here or it selects
 *    its dark built-in colours for selections.
 * 2. `link` is `#2959aa`, not `editorLink.activeForeground` (`#1f2335`). That
 *    key is near-identical to the body text colour, so links would be
 *    indistinguishable from prose. `#2959aa` is the theme's blue accent
 *    `charts.blue`, `button.background`, `progressBar.background`,
 *    `gitDecoration.stageModifiedResourceForeground`.
 *
 * Palette: MIT, (c) Enkia.
 */
export const tokyoNightLight: ChinuuTheme = {
  name: "Tokyo Night Light",
  dark: false,
  // Same stacks as the dark variants: only the colours differ.
  fontBody:
    "ui-sans-serif, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, system-ui, sans-serif",
  fontMono:
    "ui-monospace, SFMono-Regular, Menlo, Consolas, 'Liberation Mono', monospace",
  background: "#e6e7ed", // editor.background
  foreground: "#343b59", // editor.foreground
  caret: "#363c4d", // editorCursor.foreground
  selection: "#acb0bf40", // editor.selectionBackground
  activeLine: "#dcdee3", // editor.lineHighlightBackground
  mark: "#888b94", // `comment` token colour
  headingMuted: "#707280", // panelTitle.inactiveForeground
  link: "#2959aa", // charts.blue / button.background see note above
  strikethrough: "#707280", // panelTitle.inactiveForeground
  quoteText: "#707280",
  quoteBorder: "#c1c2c7", // panel.border
  codeBackground: "#d6d8df", // editorWidget.background
  codeForeground: "#343b59", // editor.foreground
  inlineCodeBackground: "#d6d8df",
  inlineCodeBorder: "#c1c2c7",
  tableBackground: "#d6d8df",
  tableForeground: "#363c4d", // editorWidget.foreground
  rule: "#c1c2c7", // panel.border
  // Code syntax. Keyword, number, comment and function are the colours this theme
  // assigns to `storage.modifier`, `constant.numeric`, `comment` and
  // `support.function`. String and type are my assignments from its verified
  // accents (`charts.green`, `charts.blue`), because the per-scope colours for
  // those two were not among the parts of the file I read.
  codeKeyword: "#7b43ba", // storage.modifier
  codeString: "#33635c", // charts.green
  codeNumber: "#965027", // constant.numeric
  codeComment: "#888b94", // comment
  codeFunction: "#006c86", // support.function
  codeType: "#2959aa", // charts.blue
  menuBackground: "#d6d8df", // editorWidget.background
  menuBorder: "#c1c2c7", // panel.border
  menuHighlight: "#c1c2c7", // panel.border
};
