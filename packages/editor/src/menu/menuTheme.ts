import { EditorView } from "@codemirror/view";
import { CLASS } from "../classes.ts";
import { v } from "../theme/tokens.ts";

/**
 * Menu styling.
 *
 * `baseTheme` rather than `theme`: this is editor chrome, not document content,
 * so it does not belong in `documentTheme`. Both are nonetheless *prefixed with a
 * class on the editor root* `baseTheme` uses CM6's own base class, `theme` uses
 * one it generates which is why the menu is appended to `view.dom` instead of
 * `document.body`. That placement also means it inherits the `--chinuu-*`
 * variables declared on that root, so no bridging is needed.
 *
 * `position: fixed` escapes the editor's scroll container, so the menu stays put
 * while the document scrolls. It is relative to the viewport only while no
 * ancestor carries a transform or filter the same constraint CM6's own tooltips
 * have.
 *
 * The menu and the flyouts repeat their panel styling rather than sharing a
 * selector list, because StyleModule's handling of comma-separated selectors is
 * harder to reason about than one duplicated block.
 *
 * `display` is deliberately never set on either: the `hidden` attribute has to be
 * able to hide them.
 */
export const menuTheme = EditorView.baseTheme({
  [`.${CLASS.menu}`]: {
    position: "fixed",
    zIndex: "1000",
    minWidth: "11rem",
    padding: "0.2rem",
    border: `1px solid ${v("menuBorder")}`,
    borderRadius: "0.5rem",
    backgroundColor: v("menuBackground"),
    color: v("foreground"),
    fontFamily: v("fontBody"),
    fontSize: "13px",
    lineHeight: "1.5",
    boxShadow: "0 8px 24px rgba(0, 0, 0, 0.28)",
    userSelect: "none",
  },

  // Top-level rows: icon, label, chevron. Separators between rows.
  [`.${CLASS.menuRow}`]: {
    display: "flex",
    alignItems: "center",
    gap: "0.65rem",
    width: "100%",
    padding: "0.45rem 0.5rem",
    border: "none",
    background: "none",
    color: "inherit",
    font: "inherit",
    textAlign: "left",
    cursor: "pointer",
  },
  [`.${CLASS.menuRow}:not(:last-of-type)`]: {
    borderBottom: `1px solid ${v("menuBorder")}`,
  },
  [`.${CLASS.menuRow}:hover, .${CLASS.menuRowActive}`]: {
    backgroundColor: v("menuHighlight"),
  },
  [`.${CLASS.menuIcon}`]: {
    flex: "0 0 auto",
    display: "inline-flex",
    color: v("mark"),
  },
  [`.${CLASS.menuChevron}`]: {
    flex: "0 0 auto",
    marginLeft: "auto",
    display: "inline-flex",
    color: v("mark"),
  },

  // The flyout opened by a row.
  [`.${CLASS.menuSubmenu}`]: {
    position: "fixed",
    zIndex: "1001",
    minWidth: "12rem",
    padding: "0.2rem",
    border: `1px solid ${v("menuBorder")}`,
    borderRadius: "0.5rem",
    backgroundColor: v("menuBackground"),
    color: v("foreground"),
    fontFamily: v("fontBody"),
    fontSize: "13px",
    lineHeight: "1.5",
    boxShadow: "0 8px 24px rgba(0, 0, 0, 0.28)",
    userSelect: "none",
  },

  [`.${CLASS.menuItem}`]: {
    display: "flex",
    alignItems: "baseline",
    justifyContent: "space-between",
    gap: "1.5rem",
    width: "100%",
    padding: "0.35rem 0.6rem",
    border: "none",
    borderRadius: "0.3rem",
    background: "none",
    color: "inherit",
    font: "inherit",
    textAlign: "left",
    cursor: "pointer",
  },
  [`.${CLASS.menuItem}:hover`]: { backgroundColor: v("menuHighlight") },
  [`.${CLASS.menuLabel}`]: { flex: "1 1 auto" },
  [`.${CLASS.menuHint}`]: {
    flex: "0 0 auto",
    color: v("mark"),
    fontFamily: v("fontMono"),
    fontSize: "0.85em",
  },

  // Task list checkboxes. `accentColor` is what actually tints a checkbox, and
  // reusing the link colour keeps one less token to theme.
  [`.${CLASS.taskCheckbox}`]: {
    width: "1em",
    height: "1em",
    marginRight: "0.4em",
    verticalAlign: "-0.12em",
    accentColor: v("link"),
    cursor: "pointer",
  },
  [`.${CLASS.taskDone}`]: {
    color: v("mark"),
    textDecoration: "line-through",
  },

  // Reading mode. The caret and the active-line band are editing affordances;
  // text selection stays visible so the content can still be copied.
  "&.cm-md-reading .cm-cursorLayer": { display: "none" },
  "&.cm-md-reading .cm-activeLine": { backgroundColor: "transparent" },
});
