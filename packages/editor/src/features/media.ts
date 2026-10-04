import type { SyntaxNode } from "@lezer/common";

/**
 * Reading the destination and label out of a `Link` or an `Image`.
 *
 * Both nodes share one child structure `LinkMark`, the text, `LinkMark`,
 * `URL` so the offsets come from the marks rather than from string surgery on
 * the source, which would have to cope with brackets and parentheses inside a
 * destination.
 *
 * Positions are made relative to `text`, so callers can hand over the node's own
 * text and do not need the document.
 */
export const mediaParts = (
  node: SyntaxNode,
  text: string,
): { url: string; label: string } | null => {
  const marks: SyntaxNode[] = [];
  let url: SyntaxNode | null = null;

  for (let child = node.firstChild; child; child = child.nextSibling) {
    if (child.name === "LinkMark") marks.push(child);
    else if (child.name === "URL") url = child;
  }

  // `[`, `]`, `(`, `)` the label sits between the first two. An image's opening
  // mark is `![`, which lands at the same index, so this is shared.
  if (!url || marks.length < 2) return null;

  const slice = (from: number, to: number) =>
    text.slice(from - node.from, to - node.from);
  return {
    url: slice(url.from, url.to),
    label: slice(marks[0].to, marks[1].from),
  };
};

/** The scheme of a destination, lowercased with its colon, or `null` for a path. */
export const schemeOf = (url: string): string | null => {
  const match = /^([a-zA-Z][a-zA-Z0-9+.-]*):/.exec(url.trim());
  return match ? `${match[1].toLowerCase()}:` : null;
};

/**
 * Control characters have no place in a URL and are a sign of a crafted one.
 *
 * The `no-control-regex` rule exists to keep escapes like these out of application
 * code by accident. Matching them is the entire purpose of this pattern, so the
 * rule is silenced on the one line that declares it. Naming the pattern also
 * matches how the other regexes in this package are written.
 */
const CONTROL_CHARS = /[\u0000-\u001f\u007f]/; // eslint-disable-line no-control-regex
export const hasControlChars = (url: string): boolean =>
  CONTROL_CHARS.test(url);
