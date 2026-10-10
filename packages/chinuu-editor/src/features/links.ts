import { Facet } from "@codemirror/state";
import { Decoration, type EditorView, WidgetType } from "@codemirror/view";
import { CLASS } from "../classes.ts";
import type { DecorationRule } from "../livePreview/types.ts";
import type { EditorFeature } from "./types.ts";
import {
  hasControlChars,
  isSafeResolvedHref,
  mediaParts,
  schemeOf,
} from "./media.ts";

/**
 * Links, rendered as real `<a>` elements.
 *
 * An actual anchor rather than a click handler: the browser then supplies hover,
 * the pointer cursor, copy-link-address, middle-click and keyboard activation, and
 * nothing has to map a click back to a document position. A previous attempt did
 * the mapping by hand and never worked.
 *
 * This rule owns the whole `Link` node, so `markers` no longer hides `LinkMark`
 * and `URL` two replaces over the same range would overlap each other. Reading
 * the node is shared with `images` through `media.ts`.
 *
 * A destination is one of two things. A **web** destination carries a scheme the
 * browser opens on its own, and is left entirely to the browser as before. A
 * **file** destination points into the vault, which a webview cannot resolve, so
 * the host owns it through `fileLinkResolver` (what the `href` becomes) and
 * `fileLinkOpener` (what a press does). With neither, a file destination keeps
 * rendering as source text.
 */

/** Schemes the browser opens on its own, so a press never needs the host. */
const WEB_SCHEMES = new Set(["http:", "https:", "mailto:", "tel:"]);

/** Whether a destination is one the browser can navigate to for real. */
const isWebDestination = (url: string): boolean => {
  if (hasControlChars(url)) return false;
  const scheme = schemeOf(url);
  return scheme !== null && WEB_SCHEMES.has(scheme);
};

/** A file in the vault: a relative destination, or an explicit `file:` URL. */
const isFileDestination = (url: string): boolean => {
  if (hasControlChars(url)) return false;
  const scheme = schemeOf(url);
  return scheme === null || scheme === "file:";
};

/**
 * Maps a file destination written in a note onto an `href`.
 *
 * A relative destination means nothing to a webview, so only the host can turn
 * it into something worth copying or hovering. Optional: with a resolver but no
 * opener, a press navigates the resolved `href`; that is the host saying the URL
 * is a real destination, such as an `asset:` URL for a file the app wants the
 * webview to show.
 */
export type FileLinkResolver = (url: string) => string;

export const fileLinkResolver = Facet.define<
  FileLinkResolver,
  FileLinkResolver | null
>({
  combine: (values) => values[0] ?? null,
});

/**
 * Called when a file link is pressed, in place of navigating.
 *
 * A plain `<a href>` makes the webview navigate, and under Tauri that replaces
 * the running application. A host that owns the destination passes this and the
 * editor cancels the press instead, so the app decides: open the note in a tab,
 * or hand anything else to the system opener.
 */
export type FileLinkOpener = (url: string) => void;

export const fileLinkOpener = Facet.define<
  FileLinkOpener,
  FileLinkOpener | null
>({
  combine: (values) => values[0] ?? null,
});

type LinkKind = "web" | "file";

class LinkWidget extends WidgetType {
  /**
   * Whether CodeMirror should keep out of events, decided in `toDOM`.
   *
   * A controlled link keeps CM6 away, because a caret moving in would reveal the
   * source and replace the anchor mid-press, so the press never lands. A file
   * destination with no host capability renders as source text instead, and there
   * the press *should* reach CM6: the caret moves in, the construct reveals, and
   * the real text comes back for editing.
   */
  private controlled = true;

  constructor(
    private readonly url: string,
    private readonly label: string,
    private readonly kind: LinkKind,
    private readonly source: string,
  ) {
    super();
  }

  eq(other: LinkWidget): boolean {
    return (
      other.url === this.url &&
      other.label === this.label &&
      other.kind === this.kind &&
      other.source === this.source
    );
  }

  /**
   * A controlled link keeps CM6 out. That matters for more than tidiness: if the
   * caret moved onto the link, the reveal would swap the anchor out for source
   * text mid-click and the navigation would never happen. The unresolved fallback
   * answers `false` instead, and `toDOM` is what decides which is which.
   */
  ignoreEvent(): boolean {
    return this.controlled;
  }
  toDOM(view: EditorView): HTMLElement {
    if (this.kind === "web") {
      const anchor = document.createElement("a");
      anchor.className = CLASS.link;
      anchor.href = this.url;
      anchor.target = "_blank";
      anchor.rel = "noopener noreferrer";
      anchor.textContent = this.label;
      return anchor;
    }

    const opener = view.state.facet(fileLinkOpener);
    const resolve = view.state.facet(fileLinkResolver);

    // No host capability: keep the markdown, as an unresolved file link has
    // always rendered. `controlled` stays false so a press reaches CM6 and the
    // caret can move in, revealing the real, editable text.
    if (!opener && !resolve) {
      this.controlled = false;
      const source = document.createElement("span");
      source.className = CLASS.linkSource;
      source.textContent = this.source;
      return source;
    }

    const href = resolve ? resolve(this.url) : null;
    const anchor = document.createElement("a");
    anchor.className = CLASS.link;
    anchor.textContent = this.label;
    if (href && isSafeResolvedHref(href)) {
      anchor.href = href;
      anchor.rel = "noopener noreferrer";
    }

    if (opener) {
      const activate = (event: MouseEvent): void => {
        event.preventDefault();
        opener(this.url);
      };
      anchor.addEventListener("click", activate);
      // Middle-click and ctrl-click dispatch `auxclick`, and would otherwise
      // start a navigation CM6 never sees.
      anchor.addEventListener("auxclick", activate);
    }

    // An anchor without an `href` is neither focusable nor announced as a link,
    // so an app-owned one states both for itself.
    if (!anchor.hasAttribute("href")) {
      anchor.tabIndex = 0;
      anchor.setAttribute("role", "link");
    }

    return anchor;
  }
}

const linkRule: DecorationRule = {
  id: "link",
  nodes: ["Link"],

  build({ from, to, node, text, reveals, add }) {
    // Editing: show `[label](url)` so it can be changed.
    if (reveals(from, to)) return;

    const parts = mediaParts(node.node, text);
    if (!parts) return;

    if (isWebDestination(parts.url)) {
      add(
        Decoration.replace({
          widget: new LinkWidget(parts.url, parts.label, "web", text),
        }),
        from,
        to,
      );
      return;
    }

    if (isFileDestination(parts.url)) {
      add(
        Decoration.replace({
          widget: new LinkWidget(parts.url, parts.label, "file", text),
        }),
        from,
        to,
      );
      return;
    }

    // `javascript:`, `data:` and any other scheme stay as source text rather
    // than becoming a live `href`.
  },
};

export const linksFeature: EditorFeature = {
  id: "links",
  rules: [linkRule],
};
