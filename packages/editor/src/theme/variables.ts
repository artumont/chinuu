import { EditorView } from "@codemirror/view";
import { TOKEN_NAMES, variableName, type ChinuuTheme } from "./tokens.ts";
import type { Extension } from "@codemirror/state";

/**
 * The variable block, plus the dark flag CM6's own defaults key off.
 *
 * This is the only part of the theme system that is per-theme; `documentTheme`
 * and `syntaxHighlightingStyle` are constants.
 */
export const themeVariables = (theme: ChinuuTheme): Extension =>
  EditorView.theme(
    {
      "&": Object.fromEntries(
        TOKEN_NAMES.map((token) => [variableName(token), theme[token]]),
      ),
    },
    { dark: theme.dark },
  );
