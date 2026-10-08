import { tokyoNightStorm } from "./storm.ts";
import type { ChinuuTheme } from "../../tokens.ts";

/**
 * Tokyo Night the default dark variant.
 *
 * Derived from Storm rather than restated, because that is what upstream does:
 * `folke/tokyonight.nvim` `colors/night.lua` deepcopies `colors/storm.lua` and
 * overrides only `bg`, `bg_dark` and `bg_dark1`. Everything else the whole
 * accent palette is identical between the two.
 *
 * The overrides below are the three background slots, plus the two UI values
 * that `enkia/tokyo-night-vscode-theme` gives differently for this variant
 * (`editor.selectionBackground`, `editor.lineHighlightBackground`).
 */
export const tokyoNight: ChinuuTheme = {
  ...tokyoNightStorm,
  name: "Tokyo Night",
  background: "#1a1b26", // night bg
  codeBackground: "#16161e", // night bg_dark
  inlineCodeBackground: "#16161e",
  tableBackground: "#1e202e",
  activeLine: "#1e202e", // editor.lineHighlightBackground
  selection: "#515c7e4d", // editor.selectionBackground
};
