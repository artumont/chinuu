import { Decoration, WidgetType } from "@codemirror/view";
import { CLASS } from "../classes.ts";
import type { DecorationRule } from "../livePreview/types.ts";
import type { EditorFeature } from "./types.ts";
import { hasControlChars, mediaParts, schemeOf } from "./media.ts";

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
 */

/** Schemes that may appear in an `href`. */
const SAFE_SCHEMES = new Set(["http:", "https:", "mailto:", "tel:"]);

/**
 * Whether a destination may become an `href`.
 *
 * The destination is document text, so a note can contain `[x](javascript:…)`.
 * Anything not on the allowlist renders as plain text instead of a link, and a
 * relative destination is refused too: it has no meaning outside a vault, and the
 * host is the only thing that could resolve it.
 */
const isSafeHref = (url: string): boolean => {
  const trimmed = url.trim();
  if (trimmed.length === 0) return false;
  if (hasControlChars(trimmed)) return false;

  const scheme = schemeOf(trimmed);
  // A relative destination is refused here, unlike an image's. It has no meaning
  // outside a vault, and resolving one needs a host capability this package does
  // not have.
  if (!scheme) return false;
  return SAFE_SCHEMES.has(scheme);
};

class LinkWidget extends WidgetType {
  constructor(
    private readonly url: string,
    private readonly label: string,
  ) {
    super();
  }

  eq(other: LinkWidget): boolean {
    return other.url === this.url && other.label === this.label;
  }

  /**
   * `ignoreEvent` is deliberately left at its default of `true`.
   *
   * It keeps CM6 from acting on the press, which matters for more than tidiness:
   * if the caret moved onto the link, the reveal would swap this anchor out for
   * source text mid-click and the navigation would never happen.
   */
  toDOM(): HTMLElement {
    const anchor = document.createElement("a");
    anchor.className = CLASS.link;
    anchor.href = this.url;
    anchor.target = "_blank";
    anchor.rel = "noopener noreferrer";
    anchor.textContent = this.label;
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
    // An unsafe destination is left as source text rather than silently dropped.
    if (!parts || !isSafeHref(parts.url)) return;

    add(
      Decoration.replace({ widget: new LinkWidget(parts.url, parts.label) }),
      from,
      to,
    );
  },
};

export const linksFeature: EditorFeature = {
  id: "links",
  rules: [linkRule],
};
