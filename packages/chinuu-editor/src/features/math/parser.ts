import type {
  InlineContext,
  InlineParser,
  MarkdownConfig,
} from "@lezer/markdown";

export const MATH_INLINE = "MathInline";
export const MATH_DISPLAY = "MathDisplay";

const DOLLAR = 0x24;
const BACKSLASH = 0x5c;

const isSpace = (code: number): boolean =>
  code === 0x20 || code === 0x09 || code === 0x0a || code === 0x0d;

const isDigit = (code: number): boolean => code >= 0x30 && code <= 0x39;

/**
 * Parses `$...$` as inline math and `$$...$$` as display math.
 *
 * Runs `before: "Emphasis"` so `$` is claimed before the delimiter machinery
 * treats it as plain text.
 *
 * The interesting part is refusing to fire. `$5 and $6` in prose must stay
 * prose, so a closer is only accepted when it neither follows whitespace nor
 * precedes a digit, and an opener may not be followed by whitespace. A `$$`
 * pair cannot close inline math, which keeps `$$` from being read as an empty
 * inline span. Unterminated `$` declines rather than swallowing the paragraph.
 *
 * Because a Markdown inline section is a whole paragraph, this also matches
 * `$$` blocks spread over several lines those are emitted as `MathDisplay`
 * spanning a line break, which is why the render rule for them has to live in
 * the block decoration source.
 */
const mathParser: InlineParser = {
  name: "Math",
  before: "Emphasis",

  parse(cx: InlineContext, next: number, pos: number): number {
    if (next !== DOLLAR) return -1;

    const display = cx.char(pos + 1) === DOLLAR;
    const contentStart = pos + (display ? 2 : 1);
    if (contentStart >= cx.end) return -1;

    let close = -1;

    if (display) {
      for (let i = contentStart; i < cx.end - 1; i++) {
        if (cx.char(i) !== DOLLAR || cx.char(i + 1) !== DOLLAR) continue;
        if (cx.char(i - 1) === BACKSLASH) continue;
        close = i;
        break;
      }
      // `$$$$` and empty display math.
      if (close <= contentStart) return -1;
    } else {
      // An opener followed by whitespace is prose, not math.
      if (isSpace(cx.char(contentStart))) return -1;

      for (let i = contentStart; i < cx.end; i++) {
        if (cx.char(i) !== DOLLAR) continue;
        if (cx.char(i - 1) === BACKSLASH) continue;
        if (cx.char(i + 1) === DOLLAR) continue;
        if (isSpace(cx.char(i - 1))) continue;
        if (isDigit(cx.char(i + 1))) continue;
        close = i;
        break;
      }
      if (close < 0) return -1;
    }

    const to = close + (display ? 2 : 1);
    return cx.addElement(cx.elt(display ? MATH_DISPLAY : MATH_INLINE, pos, to));
  },
};

export const mathParserExtension: MarkdownConfig = {
  defineNodes: [MATH_INLINE, MATH_DISPLAY],
  parseInline: [mathParser],
};
