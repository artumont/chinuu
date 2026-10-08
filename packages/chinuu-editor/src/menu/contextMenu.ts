import type { Extension } from "@codemirror/state";
import { EditorView, ViewPlugin, type ViewUpdate } from "@codemirror/view";
import { CLASS } from "../classes.ts";
import type { MenuItem, MenuSection } from "./types.ts";

const SVG_NS = "http://www.w3.org/2000/svg";

/**
 * Builds a 16x16 stroke icon from SVG path data.
 *
 * Path data rather than markup on purpose: building the nodes with
 * `createElementNS` means a feature contributing an icon never gets string
 * interpolation into `innerHTML`, so a third-party feature cannot smuggle markup
 * in through its icon.
 */
const iconNode = (
  paths: readonly string[],
  strokeWidth = "1.4",
): SVGSVGElement => {
  const svg = document.createElementNS(SVG_NS, "svg");
  svg.setAttribute("viewBox", "0 0 16 16");
  svg.setAttribute("width", "15");
  svg.setAttribute("height", "15");
  svg.setAttribute("fill", "none");
  svg.setAttribute("stroke", "currentColor");
  svg.setAttribute("stroke-width", strokeWidth);
  svg.setAttribute("stroke-linecap", "round");
  svg.setAttribute("stroke-linejoin", "round");
  svg.setAttribute("aria-hidden", "true");

  for (const d of paths) {
    const path = document.createElementNS(SVG_NS, "path");
    path.setAttribute("d", d);
    svg.appendChild(path);
  }
  return svg;
};

/** Shown on every row that opens a submenu. */
const CHEVRON = ["M6 3.5 10.5 8 6 12.5"];

/**
 * Owns one editor's menu.
 *
 * Two elements: the rows, and one flyout per section. The flyouts are siblings
 * rather than nested inside their row so they can be positioned `fixed` without
 * inheriting the row's box.
 *
 * Everything is attached to the editor root, following CM6's own convention for
 * popups: the theme rules are prefixed with a class on that root, so an element
 * elsewhere would render unstyled. Being inside the root also means the menu
 * inherits the `--chinuu-*` variables with no copying.
 */
class MenuController {
  private readonly element: HTMLElement;
  private readonly items = new Map<string, MenuItem>();
  private readonly rows = new Map<string, HTMLElement>();
  private readonly panels = new Map<string, HTMLElement>();
  private openSection: string | null = null;

  /**
   * `mousedown` rather than `click`, and preventDefault so the editor never loses
   * focus: a blurred editor has no selection to format.
   *
   * Attached to the rows container *and* to every flyout. The flyouts are siblings
   * of the container they have to be, to be positioned independently of the
   * rows so listening only on the container meant item clicks were never handled
   * at all. Measured: every one of the three items reported no change.
   */
  private readonly onMouseDown = (event: MouseEvent): void => {
    event.preventDefault();
    const target = event.target as HTMLElement | null;

    const row = target?.closest<HTMLElement>(`.${CLASS.menuRow}`);
    if (row?.dataset.section) {
      this.toggleSection(row.dataset.section);
      return;
    }

    const item = target?.closest<HTMLElement>(`.${CLASS.menuItem}`);
    if (item?.dataset.id) this.activate(item.dataset.id);
  };

  /** Hovering a row opens its submenu, which is what the chevron promises. */
  private readonly onMouseOver = (event: MouseEvent): void => {
    const row = (event.target as HTMLElement | null)?.closest<HTMLElement>(
      `.${CLASS.menuRow}`,
    );
    if (row?.dataset.section) this.showSection(row.dataset.section);
  };

  constructor(
    private readonly view: EditorView,
    sections: readonly MenuSection[],
  ) {
    this.element = document.createElement("div");
    this.element.className = CLASS.menu;
    this.element.setAttribute("role", "menu");
    this.element.hidden = true;

    for (const section of sections) {
      if (section.items.length === 0) continue;
      this.element.appendChild(this.buildRow(section));
      // One flyout per section, so each can anchor to its own row.
      this.view.dom.appendChild(this.buildPanel(section));
    }

    this.element.addEventListener("mousedown", this.onMouseDown);
    this.element.addEventListener("mouseover", this.onMouseOver);

    this.view.dom.appendChild(this.element);

    // A fixed-position menu would detach from its target on any scroll.
    window.addEventListener("resize", this.hide);
    this.view.dom.addEventListener("scroll", this.hide, true);
    document.addEventListener("scroll", this.hide, true);
  }

  get isEmpty(): boolean {
    return this.items.size === 0;
  }

  private buildRow(section: MenuSection): HTMLElement {
    const row = document.createElement("button");
    row.type = "button";
    row.className = CLASS.menuRow;
    row.dataset.section = section.id;
    row.setAttribute("role", "menuitem");
    row.setAttribute("aria-haspopup", "true");
    row.setAttribute("aria-expanded", "false");

    if (section.icon) {
      const icon = document.createElement("span");
      icon.className = CLASS.menuIcon;
      icon.appendChild(iconNode(section.icon));
      row.appendChild(icon);
    }

    const label = document.createElement("span");
    label.className = CLASS.menuLabel;
    label.textContent = section.label;
    row.appendChild(label);

    const chevron = document.createElement("span");
    chevron.className = CLASS.menuChevron;
    chevron.appendChild(iconNode(CHEVRON, "1.7"));
    row.appendChild(chevron);

    this.rows.set(section.id, row);
    return row;
  }

  private buildPanel(section: MenuSection): HTMLElement {
    const panel = document.createElement("div");
    panel.className = CLASS.menuSubmenu;
    panel.dataset.section = section.id;
    panel.setAttribute("role", "menu");
    panel.hidden = true;
    panel.addEventListener("mousedown", this.onMouseDown);

    for (const item of section.items) {
      this.items.set(item.id, item);

      const button = document.createElement("button");
      button.type = "button";
      button.className = CLASS.menuItem;
      button.dataset.id = item.id;
      button.setAttribute("role", "menuitem");

      const label = document.createElement("span");
      label.className = CLASS.menuLabel;
      label.textContent = item.label;
      button.appendChild(label);

      if (item.hint) {
        const hint = document.createElement("span");
        hint.className = CLASS.menuHint;
        hint.textContent = item.hint;
        button.appendChild(hint);
      }
      panel.appendChild(button);
    }

    this.panels.set(section.id, panel);
    return panel;
  }

  private showSection(id: string): void {
    for (const [key, panel] of this.panels) panel.hidden = key !== id;
    for (const [key, row] of this.rows) {
      const active = key === id;
      row.setAttribute("aria-expanded", String(active));
      row.classList.toggle(CLASS.menuRowActive, active);
    }
    this.openSection = id;
    this.positionPanel(id);
  }

  private toggleSection(id: string): void {
    if (this.openSection === id) this.closeSections();
    else this.showSection(id);
  }

  private closeSections(): void {
    for (const panel of this.panels.values()) panel.hidden = true;
    for (const row of this.rows.values()) {
      row.setAttribute("aria-expanded", "false");
      row.classList.remove(CLASS.menuRowActive);
    }
    this.openSection = null;
  }

  /** Prefers the right of the row, flipping left when it would overflow. */
  private positionPanel(id: string): void {
    const row = this.rows.get(id);
    const panel = this.panels.get(id);
    if (!row || !panel) return;

    panel.style.left = "0px";
    panel.style.top = "0px";
    const rowRect = row.getBoundingClientRect();
    const size = panel.getBoundingClientRect();
    const margin = 8;

    let left = rowRect.right - 2;
    if (left + size.width > window.innerWidth - margin) {
      left = rowRect.left - size.width + 2;
    }
    const top = Math.min(
      Math.max(margin, rowRect.top - 6),
      Math.max(margin, window.innerHeight - size.height - margin),
    );

    panel.style.left = `${Math.max(margin, Math.round(left))}px`;
    panel.style.top = `${Math.round(top)}px`;
  }

  private activate(id: string): void {
    const item = this.items.get(id);
    // Close first: the command may reconfigure the editor, and a stale menu
    // hovering over the result is worse than a flicker.
    this.hide();
    this.view.focus();
    item?.run({ view: this.view });
  }

  openAt(x: number, y: number): void {
    this.element.hidden = false;

    // Clamp after measuring, so a click near an edge still shows the whole menu.
    const margin = 8;
    this.element.style.left = `${x}px`;
    this.element.style.top = `${y}px`;
    const rect = this.element.getBoundingClientRect();
    this.element.style.left = `${Math.max(margin, Math.min(x, window.innerWidth - rect.width - margin))}px`;
    this.element.style.top = `${Math.max(margin, Math.min(y, window.innerHeight - rect.height - margin))}px`;

    // Any open flyout belonged to the previous position.
    this.closeSections();
  }

  hide = (): void => {
    this.element.hidden = true;
    this.closeSections();
  };

  destroy(): void {
    window.removeEventListener("resize", this.hide);
    this.view.dom.removeEventListener("scroll", this.hide, true);
    document.removeEventListener("scroll", this.hide, true);
    for (const panel of this.panels.values()) panel.remove();
    this.element.remove();
  }
}

/**
 * A right-click menu built from the feature set's sections.
 *
 * Takes over `contextmenu` inside the editor only, and only when there is
 * something to show with no items the browser's own menu is left alone.
 */
export const contextMenu = (sections: readonly MenuSection[]): Extension => {
  // Keyed by view: the extension object is shared between editors, the elements
  // and their listeners must not be.
  const controllers = new WeakMap<EditorView, MenuController>();

  return [
    ViewPlugin.fromClass(
      class {
        // CM6 constructs the plugin with the view, and also assigns `view`
        // afterwards; declaring it here is what makes `this.view` typed.
        constructor(readonly view: EditorView) {
          controllers.set(view, new MenuController(view, sections));
        }

        update(update: ViewUpdate) {
          // Any edit or selection change invalidates the target the menu was
          // opened for, so an item can never act on the wrong range.
          if (
            update.docChanged ||
            update.selectionSet ||
            update.viewportChanged
          ) {
            controllers.get(update.view)?.hide();
          }
        }

        destroy() {
          controllers.get(this.view)?.destroy();
          controllers.delete(this.view);
        }
      },
    ),

    EditorView.domEventHandlers({
      contextmenu(event, view) {
        const controller = controllers.get(view);
        if (!controller || controller.isEmpty) return false;
        // Read-only modes keep the browser's own menu, which at least offers
        // copy. Showing actions that cannot run would be worse than nothing.
        if (view.state.readOnly) return false;
        event.preventDefault();
        controller.openAt(event.clientX, event.clientY);
        return true;
      },
      mousedown(_event, view) {
        // Covers a left-click elsewhere, and the right-click's own mousedown
        // that precedes `contextmenu`.
        controllers.get(view)?.hide();
        return false;
      },
      keydown(event, view) {
        if (event.key === "Escape") controllers.get(view)?.hide();
        return false;
      },
      blur(_event, view) {
        controllers.get(view)?.hide();
        return false;
      },
    }),
  ];
};
