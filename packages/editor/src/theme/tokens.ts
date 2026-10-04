/**
 * A theme is a flat bag of semantic tokens.
 *
 * Tokens are published as CSS custom properties (`--chinuu-*`) on the editor
 * root rather than being interpolated into the CM6 spec. That is the whole
 * trick: `documentTheme` and `syntaxHighlightingStyle` in `styles.ts` are
 * *constants* that read `var(--chinuu-*)`, so switching theme swaps one small
 * variable block instead of rebuilding the highlight style and every generated
 * class name.
 *
 * It also means a theme can ship as plain CSS redeclare these variables on
 * `.cm-editor` and nothing in TypeScript has to change.
 */
export interface ThemeTokens {
  /** Body font stack for prose. */
  readonly fontBody: string;
  /** Font stack for code, tables and inline code. */
  readonly fontMono: string;
  readonly background: string;
  readonly foreground: string;
  readonly caret: string;
  /** Colour of a text selection while the editor has focus. */
  readonly selection: string;
  /** Background of the line holding the caret. */
  readonly activeLine: string;
  /** Revealed syntax marks (`**`, `#`, backticks). Muted on purpose. */
  readonly mark: string;
  /** h6, the one heading level that reads as a label rather than a title. */
  readonly headingMuted: string;
  readonly link: string;
  readonly strikethrough: string;
  readonly quoteText: string;
  readonly quoteBorder: string;
  readonly codeBackground: string;
  readonly codeForeground: string;
  readonly inlineCodeBackground: string;
  readonly inlineCodeBorder: string;
  readonly tableBackground: string;
  readonly tableForeground: string;
  /** The rule drawn in place of `---`. */
  readonly rule: string;
  /**
   * Code-block syntax colours.
   *
   * These exist because fenced code is parsed by the nested language, so the
   * tags below the markdown tree need colours of their own otherwise the
   * highlighting is invisible and all code reads as one flat colour.
   */
  readonly codeKeyword: string;
  readonly codeString: string;
  readonly codeNumber: string;
  readonly codeComment: string;
  readonly codeFunction: string;
  readonly codeType: string;
  /**
   * Context menu surface. The menu renders outside the editor's DOM subtree, so
   * these are bridged onto the menu element at open time.
   */
  readonly menuBackground: string;
  readonly menuBorder: string;
  /** Menu item hover fill. */
  readonly menuHighlight: string;
}

export interface ChinuuTheme extends ThemeTokens {
  readonly name: string;
  /** Drives `EditorView.darkTheme`, which CM6's own built-in colours key off. */
  readonly dark: boolean;
}

/**
 * Every token, listed once.
 *
 * `satisfies` makes this fail to compile if a token is added to `ThemeTokens`
 * and forgotten here, so the interface and the generated CSS variables cannot
 * drift apart.
 */
export const TOKEN_NAMES = [
  "fontBody",
  "fontMono",
  "background",
  "foreground",
  "caret",
  "selection",
  "activeLine",
  "mark",
  "headingMuted",
  "link",
  "strikethrough",
  "quoteText",
  "quoteBorder",
  "codeBackground",
  "codeForeground",
  "inlineCodeBackground",
  "inlineCodeBorder",
  "tableBackground",
  "tableForeground",
  "rule",
  "codeKeyword",
  "codeString",
  "codeNumber",
  "codeComment",
  "codeFunction",
  "codeType",
  "menuBackground",
  "menuBorder",
  "menuHighlight",
] as const satisfies readonly (keyof ThemeTokens)[];

export const FONT_SANS =
  "ui-sans-serif, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, system-ui, sans-serif";

export const FONT_MONO =
  "ui-monospace, SFMono-Regular, Menlo, Consolas, 'Liberation Mono', monospace";

/** Font stacks shared by the built-in themes; a theme may override either. */
export const defaultFonts = {
  fontBody: FONT_SANS,
  fontMono: FONT_MONO,
} as const;

const kebab = (name: string) =>
  name.replace(/[A-Z]/g, (c) => `-${c.toLowerCase()}`);

/** `fontMono` → `--chinuu-font-mono`. */
export const variableName = (token: keyof ThemeTokens): string =>
  `--chinuu-${kebab(token)}`;

export const v = (token: keyof ThemeTokens): string =>
  `var(${variableName(token)})`;
