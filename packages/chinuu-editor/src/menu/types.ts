import type { EditorView } from "@codemirror/view";

/** What a menu item acts on. */
export interface MenuContext {
  /** The editor. Items should dispatch through it. */
  readonly view: EditorView;
}

export interface MenuItem {
  /** Stable id, also used by the test page to activate an item. */
  readonly id: string;
  readonly label: string;
  /** Right-aligned hint, normally the marker the item applies. */
  readonly hint?: string;
  run(context: MenuContext): void;
}

/**
 * A top-level menu row and the submenu it opens.
 *
 * Rendered as `icon │ label │ chevron`, with the items in a flyout rather than
 * inline. That keeps the menu three rows tall however many actions are
 * registered, and the chevron is what tells the user there is more behind the row.
 */
export interface MenuSection {
  /** Stable id, used for the row's `data-section` and for opening it in tests. */
  readonly id: string;
  readonly label: string;
  /** Inline SVG path data for the row's leading icon, drawn at 16x16. */
  readonly icon?: readonly string[];
  readonly items: readonly MenuItem[];
}
