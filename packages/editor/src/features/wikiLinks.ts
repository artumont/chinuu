import { Facet } from "@codemirror/state";
import { Decoration, type EditorView, WidgetType } from "@codemirror/view";
import type {
  InlineContext,
  InlineParser,
  MarkdownConfig,
} from "@lezer/markdown";
import { CLASS } from "../classes.ts";
import { markClass } from "../livePreview/decorations.ts";
import type { DecorationRule } from "../livePreview/types.ts";
import { hasControlChars, schemeOf } from "./media.ts";
import type { EditorFeature } from "./types.ts";

const WIKI_LINK = "WikiLink";

const OPEN_BRACKET = 0x5b;
const CLOSE_BRACKET = 0x5d;
const NEWLINE = 0x0a;

/**
 * Parses `[[target]]` and `[[target|alias]]`.
 *
 * `before: "Link"` is what makes this work at all. `Link` claims `[`, so without
 * the precedence `[[note]]` is parsed as a shortcut reference link and never
 * reaches this parser which is exactly how it rendered before: as prose wearing
 * `tags.link` because the inner `[note]` looked like a link with no definition.
 *
 * A `]` that is not part of `]]` declines the whole construct rather than letting
 * the delimiters span it, and so does a line break: nesting is not a thing in a
 * wiki link, and a target that swallowed a paragraph would be worse than one left
 * as text.
 */
const wikiLinkParser: InlineParser = {
  name: "WikiLink",
  before: "Link",

  parse(cx: InlineContext, next: number, pos: number): number {
    if (next !== OPEN_BRACKET || cx.char(pos + 1) !== OPEN_BRACKET) return -1;

    const contentStart = pos + 2;
    let close = -1;

    for (let i = contentStart; i < cx.end - 1; i++) {
      const code = cx.char(i);
      if (code === NEWLINE || code === OPEN_BRACKET) return -1;
      if (code !== CLOSE_BRACKET) continue;
      if (cx.char(i + 1) === CLOSE_BRACKET) {
        close = i;
        break;
      }
      return -1;
    }

    // `[[]]` closes where it opens.
    if (close <= contentStart) return -1;

    return cx.addElement(cx.elt(WIKI_LINK, pos, close + 2));
  },
};

const wikiLinkParserExtension: MarkdownConfig = {
  defineNodes: [WIKI_LINK],
  parseInline: [wikiLinkParser],
};

/**
 * Maps a wiki link's target onto an `href`.
 *
 * A wiki link names a note, not a location, so the webview cannot resolve it on its
 * own the host is the only thing that knows how a note maps onto a route or an
 * app scheme. With no resolver the anchor is inert but still styled, because
 * guessing a path would be worse than doing nothing.
 */
export type WikiLinkResolver = (target: string) => string;

export const wikiLinkResolver = Facet.define<
  WikiLinkResolver,
  WikiLinkResolver | null
>({
  combine: (values) => values[0] ?? null,
});

/** Schemes a resolved `href` may carry. A relative one is fine: the host made it. */
const SAFE_SCHEMES = new Set([
  "http:",
  "https:",
  "mailto:",
  "chinuu:",
  "asset:",
  "file:",
]);

/**
 * Guards against a hostile *target* steering a trusted resolver into something
 * dangerous. The resolver is the app's own code, but its input is document text, so
 * a note could contain `[[javascript:...]]` and get back a live `href`.
 */
const isSafeHref = (href: string): boolean => {
  const trimmed = href.trim();
  if (trimmed.length === 0) return false;
  if (hasControlChars(trimmed)) return false;

  const scheme = schemeOf(trimmed);
  return scheme === null || SAFE_SCHEMES.has(scheme);
};

/** Splits `target|alias` out of the node's source text. */
const wikiLinkParts = (
  text: string,
): { target: string; label: string } | null => {
  const inner = text.slice(2, -2);
  const pipe = inner.indexOf("|");
  const target = (pipe < 0 ? inner : inner.slice(0, pipe)).trim();
  const label = (pipe < 0 ? inner : inner.slice(pipe + 1)).trim();
  if (target.length === 0 || label.length === 0) return null;
  return { target, label };
};

class WikiLinkWidget extends WidgetType {
  constructor(
    private readonly target: string,
    private readonly label: string,
  ) {
    super();
  }

  eq(other: WikiLinkWidget): boolean {
    return other.target === this.target && other.label === this.label;
  }

  /**
   * `ignoreEvent` is left at its default of `true`, so pressing a wiki link does not
   * move the caret onto it moving it would reveal the source and swap the anchor
   * out from under the pointer mid-click.
   *
   * The resolver is read here rather than when the rule runs, because a widget is the
   * first place with the view in hand.
   */
  toDOM(view: EditorView): HTMLElement {
    const anchor = document.createElement("a");
    anchor.className = CLASS.wikiLink;
    anchor.textContent = this.label;

    const resolve = view.state.facet(wikiLinkResolver);
    const href = resolve ? resolve(this.target) : null;
    if (href && isSafeHref(href)) {
      anchor.href = href;
      anchor.rel = "noopener noreferrer";
    }
    return anchor;
  }
}

const wikiLinkRule: DecorationRule = {
  id: "wiki-link",
  nodes: [WIKI_LINK],

  build({ from, to, text, reveals, add }) {
    const parts = wikiLinkParts(text);
    // An empty target stays as source text rather than becoming a dead link.
    if (!parts) return;

    // Editing: show `[[target|alias]]`, kept in the wiki-link colour so the
    // construct stays recognisable `[[` is easy to lose in a sentence.
    if (reveals(from, to)) {
      add(markClass(CLASS.wikiLink), from, to);
      return;
    }

    add(
      Decoration.replace({
        widget: new WikiLinkWidget(parts.target, parts.label),
      }),
      from,
      to,
    );
  },
};

export const wikiLinksFeature: EditorFeature = {
  id: "wiki-links",
  markdown: wikiLinkParserExtension,
  rules: [wikiLinkRule],
};
