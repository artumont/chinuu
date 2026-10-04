import { HighlightStyle } from "@codemirror/language";
import { EditorView } from "@codemirror/view";
import { tags as t } from "@lezer/highlight";
import { CLASS, HEADING_CLASSES } from "../classes.ts";
import { v } from "./tokens.ts";

/**
 * Inline highlighting. Constant across themes every value is a variable.
 *
 * Deliberately defines **no rule for `tags.heading1`…`tags.heading6`**. That tag
 * is what `defaultHighlightStyle` styles, and its rule is
 * `{text-decoration: underline, font-weight: bold}` a code-editor convention
 * for headings that are inline source. Worse, `@lezer/markdown` registers
 * `"ATXHeading1/..."`, and the `/...` suffix propagates the tag to every
 * descendant, so the `#` HeaderMark inherits it too and gets underlined as well.
 * Heading size and weight come from the `cm-md-h*` line classes instead.
 *
 * Same reasoning for `tags.quote`: it is registered as `"Blockquote/..."` and
 * would colour every descendant. The `cm-md-quote` line class owns that.
 */
export const syntaxHighlightingStyle = HighlightStyle.define([
  // Syntax marks, shown only while the caret is inside their construct. Muted so
  // the prose stays dominant this is the difference between "the `**` appeared"
  // and "bold text appeared".
  { tag: t.processingInstruction, color: v("mark") },
  { tag: t.labelName, color: v("mark") },

  // Inline content
  { tag: t.strong, fontWeight: "700" },
  { tag: t.emphasis, fontStyle: "italic" },
  {
    tag: t.strikethrough,
    textDecoration: "line-through",
    color: v("strikethrough"),
  },
  // No padding or background here: `InlineCode` and `CodeText` both carry this
  // tag, so a box on the tag would nest and double the padding. The box comes
  // from the `cm-md-inline-code` mark decoration on the outer node.
  {
    tag: t.monospace,
    fontFamily: v("fontMono"),
    fontSize: "0.9em",
    color: v("codeForeground"),
  },

  // Links. Underlined as well as coloured: colour alone is a weak signal, and the
  // markdown source that would otherwise distinguish them is hidden.
  { tag: t.link, color: v("link"), textDecoration: "underline" },
  { tag: t.url, color: v("link") },

  { tag: t.contentSeparator, color: v("mark") },

  // Code-block syntax. Nested language parsing puts these tags inside fenced
  // code, where the markdown rules above say nothing, so without them every
  // token would fall back to the plain code foreground.
  {
    tag: [t.keyword, t.controlKeyword, t.moduleKeyword, t.operatorKeyword],
    color: v("codeKeyword"),
  },
  {
    tag: [t.string, t.special(t.string), t.regexp, t.escape],
    color: v("codeString"),
  },
  {
    tag: [t.number, t.bool, t.null, t.atom, t.constant(t.name)],
    color: v("codeNumber"),
  },
  {
    tag: [t.lineComment, t.blockComment, t.comment, t.docComment],
    color: v("codeComment"),
  },
  {
    tag: [
      t.function(t.variableName),
      t.function(t.propertyName),
      t.definition(t.function(t.variableName)),
    ],
    color: v("codeFunction"),
  },
  {
    tag: [t.typeName, t.className, t.namespace, t.definition(t.typeName)],
    color: v("codeType"),
  },
  // Muted punctuation keeps dense code readable in both light and dark themes.
  {
    tag: [t.operator, t.punctuation, t.separator, t.bracket],
    color: v("codeComment"),
  },
  {
    tag: [t.variableName, t.propertyName, t.definition(t.variableName)],
    color: v("codeForeground"),
  },
]);

/**
 * Heading sizes, one entry per level.
 *
 * `paddingTop` rather than `marginTop`: CM6 measures line boxes, and collapsed
 * margins make it guess wrong about scroll height.
 */
const HEADING_STYLES: readonly Record<string, string>[] = [
  {
    fontSize: "1.9em",
    fontWeight: "700",
    lineHeight: "1.3",
    paddingTop: "0.6em",
  },
  {
    fontSize: "1.5em",
    fontWeight: "700",
    lineHeight: "1.3",
    paddingTop: "0.55em",
  },
  {
    fontSize: "1.22em",
    fontWeight: "650",
    lineHeight: "1.35",
    paddingTop: "0.45em",
  },
  { fontSize: "1.08em", fontWeight: "650", paddingTop: "0.35em" },
  { fontSize: "1em", fontWeight: "650", paddingTop: "0.3em" },
  {
    fontSize: "0.92em",
    fontWeight: "650",
    color: v("headingMuted"),
    paddingTop: "0.3em",
  },
];

const headingRules: Record<string, Record<string, string>> = {};
HEADING_CLASSES.forEach((headingClass, index) => {
  headingRules[`.${headingClass}`] = HEADING_STYLES[index];
});

/**
 * What makes the editor read as a page instead of a text buffer. Constant across
 * themes: every colour is a `--chinuu-*` variable.
 *
 * The `cm-md-*` classes are applied as line decorations by the live-preview
 * feature rules, straight from the Lezer tree.
 */
export const documentTheme = EditorView.theme({
  "&": {
    height: "100%",
    fontSize: "16px",
    color: v("foreground"),
    backgroundColor: v("background"),
  },
  "&.cm-focused": { outline: "none" },
  ".cm-scroller": {
    overflow: "auto",
    fontFamily: v("fontBody"),
    lineHeight: "1.7",
  },
  ".cm-content": {
    maxWidth: "44rem",
    margin: "0 auto",
    padding: "3rem 1.5rem 60vh",
    caretColor: v("caret"),
  },
  ".cm-line": { padding: "0" },
  ".cm-activeLine": { backgroundColor: v("activeLine") },

  // `drawSelection` renders selection into its own layer, and CM6's built-in
  // rule for the focused case is a four-class selector. Matching that shape is
  // required or the built-in light/dark selection colour wins on specificity.
  ".cm-selectionBackground": { backgroundColor: v("selection") },
  "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground": {
    backgroundColor: v("selection"),
  },

  // Caret. `caretColor` on `.cm-content` is not enough: it only colours the
  // native text caret, which `drawSelection()` hides because it paints the
  // caret itself into a separate layer.
  //
  // The bar cursor. CM6 ships `border-left: 1.2px solid black`
  // (`.\u0341 3 .cm-cursor {border-left-color: #ddd}` is its dark variant) both
  // two-class selectors. `.cm-scroller` makes these three, so they win on
  // specificity instead of on whichever stylesheet StyleModule injected last.
  ".cm-scroller .cm-cursor": { borderLeftColor: v("caret") },
  ".cm-scroller .cm-dropCursor": { borderLeftColor: v("caret") },

  // Vim's block cursor (normal mode). `@replit/codemirror-vim` hardcodes
  // `background: #ff9696` in `.\u0341 o .cm-fat-cursor` two classes. Qualifying
  // with `.cm-vimMode` (set on the scroller) and the cursor layer takes this to
  // four. Text under the block inverts to the page background, so `caret` plus
  // `background` compose into a readable block for any theme without needing a
  // separate token.
  ".cm-vimMode .cm-cursorLayer .cm-fat-cursor": {
    backgroundColor: v("caret"),
    color: v("background"),
  },
  // Unfocused vim cursor is an outline, not a filled block. Vim sets
  // `color: transparent !important`, which is fine only the outline needs the
  // theme colour.
  "&:not(.cm-focused) .cm-vimMode .cm-cursorLayer .cm-fat-cursor": {
    outlineColor: v("caret"),
  },

  ...headingRules,

  // Blockquote. The `>` marks are hidden, so the rule and indent are the only
  // signal that this is a quote.
  [`.${CLASS.quote}`]: {
    borderLeft: `3px solid ${v("quoteBorder")}`,
    paddingLeft: "1em",
    color: v("quoteText"),
    fontStyle: "italic",
  },

  // Fenced code. `FencedCode` includes the fence lines, whose content is hidden,
  // so the block gets blank first/last lines that read as padding.
  [`.${CLASS.code}`]: {
    backgroundColor: v("codeBackground"),
    fontFamily: v("fontMono"),
    fontSize: "0.9em",
    lineHeight: "1.55",
  },

  // Inline code box. Applied as a mark decoration on the `InlineCode` node only.
  [`.${CLASS.inlineCode}`]: {
    backgroundColor: v("inlineCodeBackground"),
    border: `1px solid ${v("inlineCodeBorder")}`,
    borderRadius: "4px",
    padding: "0.08em 0.28em",
  },

  // Tints the `---` source, which is only visible while the caret is on the
  // line; otherwise the widget below draws a real rule instead.
  [`.${CLASS.rule}`]: { color: v("mark") },

  // The widget that replaces `---`. Zero-height inline-block with a top border:
  // a real element rather than a row of dashes. `vertical-align: middle` puts
  // the border through the middle of the line box.
  [`.${CLASS.horizontalRule}`]: {
    display: "inline-block",
    width: "100%",
    borderTop: `1px solid ${v("rule")}`,
    verticalAlign: "middle",
  },

  // The language chip. A point widget sitting where the hidden info string was,
  // so the opening fence line shows `ts` instead of nothing.
  // Anchors the picker. The opening fence line is the top of the block, so the
  // picker can sit at its right edge with absolute positioning which keeps it
  // out of flow, unlike a float, so it cannot disturb CM6's line measurement.
  [`.${CLASS.codeOpen}`]: { position: "relative" },

  [`.${CLASS.codeLanguage}`]: {
    position: "absolute",
    top: "0.1em",
    right: "0.5em",
    fontFamily: v("fontMono"),
    fontSize: "0.72em",
    color: v("mark"),
    backgroundColor: v("codeBackground"),
    border: "none",
    padding: "0 1.2em 0 0.3em",
    lineHeight: "1.6",
    cursor: "pointer",
  },
  [`.${CLASS.codeLanguage}:hover`]: { color: v("foreground") },

  // The line-number gutter. `::before` rather than a widget, so the numbers stay
  // out of selections and the clipboard.
  [`.${CLASS.codeLine}`]: { position: "relative", paddingLeft: "2.4em" },
  [`.${CLASS.codeLine}::before`]: {
    content: "attr(data-code-line)",
    position: "absolute",
    left: "0",
    width: "1.9em",
    textAlign: "right",
    color: v("mark"),
    opacity: "0.65",
    userSelect: "none",
  },
  // The caret line's number reads brighter, as in the reference.
  [`.${CLASS.codeLine}.cm-activeLine::before`]: {
    color: v("foreground"),
    opacity: "1",
  },

  // The `<a>` that stands in for a `Link`. A real anchor, so hover, the pointer
  // cursor and copy-link-address come from the browser rather than from us.
  [`.${CLASS.link}`]: {
    color: v("link"),
    textDecoration: "underline",
    textUnderlineOffset: "0.15em",
    cursor: "pointer",
  },

  // A wiki link, and the `[[…]]` source while it is revealed. Same colour as an
  // external link, because to a reader they are the same thing. The pointer is
  // scoped to the anchor, so the revealed source keeps the normal text cursor.
  [`.${CLASS.wikiLink}`]: {
    color: v("link"),
    textDecoration: "underline",
    textUnderlineOffset: "0.15em",
  },
  [`a.${CLASS.wikiLink}`]: { cursor: "pointer" },

  // The `<img>` that stands in for an `Image`. Inline, so it grows the line box
  // rather than becoming a block of its own; `max-width` keeps a wide image from
  // pushing the page sideways.
  [`.${CLASS.image}`]: {
    maxWidth: "100%",
    height: "auto",
    verticalAlign: "middle",
    borderRadius: "4px",
  },

  // Rendered inline math. The same chip as inline code, so `$…$` and `` `…` ``
  // read as one kind of thing a span of non-prose and each has a visible edge.
  // The colours are the inline-code tokens rather than new ones, because matching
  // them is the entire point of the rule; adding `--chinuu-math-*` would make the
  // two drift apart the first time one of them was tuned.
  [`.${CLASS.math}`]: {
    backgroundColor: v("inlineCodeBackground"),
    border: `1px solid ${v("inlineCodeBorder")}`,
    borderRadius: "4px",
    padding: "0.08em 0.28em",
  },
  // Display math wears the code block's background, so a `$$` block reads as a
  // delimited block instead of as loose text that happens to be centred. The
  // `$$` delimiters are hidden while the caret is elsewhere, so this box is the
  // only thing marking where the block starts and ends. A radius is safe here
  // because the whole block is one element unlike a fence, which paints its
  // background per line and so cannot have rounded corners.
  [`.${CLASS.mathDisplay}`]: {
    display: "block",
    textAlign: "center",
    padding: "0.9em 1em",
    overflowX: "auto",
    backgroundColor: v("codeBackground"),
    borderRadius: "4px",
  },
  // The `$$` source lines, shown while the block is revealed for editing. Same
  // background as a fence so the two read alike, but with horizontal padding of its
  // own: a fence line gets its inset from the line-number gutter, which this has
  // none of, so the TeX would otherwise sit flush against the edge.
  [`.${CLASS.mathBlock}`]: {
    backgroundColor: v("codeBackground"),
    fontFamily: v("fontMono"),
    fontSize: "0.9em",
    paddingLeft: "1em",
    paddingRight: "1em",
  },

  // Shown when KaTeX rejects the source, so a typo is visible rather than silent.
  [`.${CLASS.mathError}`]: {
    fontFamily: v("fontMono"),
    color: v("mark"),
    borderBottom: `1px dashed ${v("mark")}`,
  },

  // Reading mode must not merely refuse edits, it must not advertise them. The
  // transaction filter already rejects these changes, but a drag handle and a
  // checkbox still inviting a click read as broken rather than as read-only.
  // `.tbl-*` is the table library's own markup rather than ours, so these are the
  // only selectors here that reach into a dependency's DOM.
  //
  // `&.` rather than `.`, because the reading class is set on the editor root the
  // same element these themes are scoped to. Written as a descendant it compiles to
  // `.gen .cm-md-reading …` and silently matches nothing, which is what the first
  // attempt did.
  [`&.${CLASS.readingMode} .tbl-handle`]: { display: "none" },
  [`&.${CLASS.readingMode} .tbl-menu-tooltip`]: { display: "none" },
  [`&.${CLASS.readingMode} .${CLASS.taskCheckbox}`]: {
    pointerEvents: "none",
    cursor: "default",
  },
});
