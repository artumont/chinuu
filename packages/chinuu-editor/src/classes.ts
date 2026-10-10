/**
 * Every `cm-md-*` class the live-preview rules apply, in one place.
 *
 * The rules add these as decorations and `theme/styles.ts` styles them, so the
 * two sides have to agree on the exact strings. Declaring them once turns a
 * typo into a compile error instead of a silently unstyled element.
 */
export const CLASS = {
  quote: "cm-md-quote",
  code: "cm-md-code",
  /** The language picker on a fence's opening line. */
  codeLanguage: "cm-md-code-language",
  /** The opening fence line, which anchors the picker to the block's top-right. */
  codeOpen: "cm-md-code-open",
  /** A code line carrying a number in its left gutter. */
  codeLine: "cm-md-code-line",
  // No table classes: `codemirror-markdown-tables` owns the table's DOM, and
  // themes it through its own `--tbl-theme-*` / `--tbl-style-*` variables.
  /** Rendered inline math. */
  math: "cm-md-math",
  /** The `<a>` standing in for a `Link`. */
  link: "cm-md-link",
  /** An unresolved file link, kept as markdown source until the host owns it. */
  linkSource: "cm-md-link-source",
  /** The `<img>` standing in for an `Image`. */
  image: "cm-md-image",
  /** An image whose destination failed to load, drawn as a placeholder. */
  imageBroken: "cm-md-image-broken",
  /** The `<a>` standing in for a `[[wiki link]]`, and its revealed source. */
  wikiLink: "cm-md-wiki-link",
  /** Rendered display math (a `$$` block). */
  mathDisplay: "cm-md-math-display",
  /** The `$$` source lines, boxed while the block is revealed for editing. */
  mathBlock: "cm-md-math-block",
  /** Applied when KaTeX rejects the source, so it falls back visibly. */
  mathError: "cm-md-math-error",
  /** The right-click menu. */
  menu: "cm-md-menu",
  /** A top-level row: icon, label, chevron. Opens a submenu. */
  menuRow: "cm-md-menu-row",
  /** The row whose submenu is showing. */
  menuRowActive: "cm-md-menu-row-active",
  /** The flyout holding a row's items. */
  menuSubmenu: "cm-md-menu-submenu",
  /** Leading icon slot on a row. */
  menuIcon: "cm-md-menu-icon",
  /** Trailing chevron marking a row as having a submenu. */
  menuChevron: "cm-md-menu-chevron",
  /** The checkbox standing in for `[ ]` / `[x]`. */
  taskCheckbox: "cm-md-task-checkbox",
  /** Mark over a completed task's text. */
  taskDone: "cm-md-task-done",
  /** On the editor root while in reading mode. */
  readingMode: "cm-md-reading",
  menuItem: "cm-md-menu-item",
  menuLabel: "cm-md-menu-label",
  /** Right-aligned marker hint, e.g. `**`. */
  menuHint: "cm-md-menu-hint",
  /** Tints the `---` source, which is only visible while the caret is on its line. */
  rule: "cm-md-rule",
  /** The element drawn in place of `---`. */
  horizontalRule: "cm-md-hr",
  inlineCode: "cm-md-inline-code",
  h1: "cm-md-h1",
  h2: "cm-md-h2",
  h3: "cm-md-h3",
  h4: "cm-md-h4",
  h5: "cm-md-h5",
  h6: "cm-md-h6",
} as const;

/** Indexed by heading level, so level 1 → `cm-md-h1`. */
export const HEADING_CLASSES = [
  CLASS.h1,
  CLASS.h2,
  CLASS.h3,
  CLASS.h4,
  CLASS.h5,
  CLASS.h6,
] as const;
