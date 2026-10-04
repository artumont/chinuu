import { defaultKeymap, historyKeymap } from "@codemirror/commands";
import { searchKeymap } from "@codemirror/search";
import { EditorView, keymap } from "@codemirror/view";
import { GFM } from "@lezer/markdown";
import { markdownTables } from "codemirror-markdown-tables";
import { v } from "../theme/tokens.ts";
import type { EditorFeature } from "./types.ts";

/**
 * Binds `codemirror-markdown-tables` to our palette.
 *
 * These have to be set **on the editor root**, not passed to `markdownTables`'
 * `theme` option. The library declares its `--tbl-*` properties in `:root` scope,
 * so a value of `var(--chinuu-rule)` written there is resolved at `:root` where
 * our editor-scoped variables do not exist and every colour silently becomes
 * empty. Measured before this change: the table rendered with no borders and no
 * background at all.
 *
 * Declaring them here instead means they cascade from the editor root, where
 * `--chinuu-*` are defined, and they follow `setTheme` with no extra plumbing.
 * Anything not listed keeps the library's own light/dark default.
 */
const tableVariables = EditorView.theme({
  "&": {
    "--tbl-theme-text-color": v("tableForeground"),
    "--tbl-theme-border-color": v("rule"),
    "--tbl-theme-border-hover-color": v("link"),
    "--tbl-theme-border-active-color": v("link"),
    "--tbl-theme-outline-color": v("link"),
    "--tbl-theme-header-row-background": v("tableBackground"),
    // On-palette zebra striping from the two surface tokens we already have.
    "--tbl-theme-even-row-background": v("tableBackground"),
    "--tbl-theme-odd-row-background": v("codeBackground"),
    // The table's own menus reuse the editor menu's colours.
    "--tbl-theme-menu-background": v("menuBackground"),
    "--tbl-theme-menu-border-color": v("menuBorder"),
    "--tbl-theme-menu-text-color": v("foreground"),
    "--tbl-theme-menu-hover-background": v("menuHighlight"),
    "--tbl-theme-menu-hover-text-color": v("foreground"),

    // Proportional, not monospace: this is a document table, not source.
    "--tbl-style-font-family": v("fontBody"),
    "--tbl-style-menu-font-family": v("fontBody"),
    "--tbl-style-menu-font-size": "13px",
  },
});

export const tablesFeature: EditorFeature = {
  id: "tables",
  extensions: [
    tableVariables,
    markdownTables({
      // The cell editor is a separate CodeMirror instance and inherits nothing.
      // `defaultKeymap` is what makes the usual shortcuts act on the cell's text
      // rather than the whole document.
      extensions: [keymap.of([...defaultKeymap])],
      // History and search should act on the document, not on one cell.
      globalKeyBindings: [...historyKeymap, ...searchKeymap],
      // Cell text is parsed on its own; GFM keeps it consistent with the root
      // language, which already includes GFM.
      markdownConfig: { extensions: GFM },
    }),
  ],
};
