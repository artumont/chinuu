# @chinuu/editor

A markdown editor for chinuu. It is CodeMirror 6 with live preview, vim keybindings,
and a file on disk that stays plain markdown.

```ts
import { chinuuLight, initEditor } from "@chinuu/editor";

const editor = initEditor(document.getElementById("editor")!, markdown, {
  theme: chinuuLight,
});

editor.getValue(); // exact bytes currently in the document
editor.destroy();
```

This is a private workspace package, so import it by name from `apps/desktop`. It ships
as TypeScript source rather than build output, and it imports KaTeX's stylesheet, so
whatever bundles it has to understand CSS imports. Vite does.

## How it works

Three decisions explain almost everything else about this editor.

**The document is the source of truth.** Nothing holds a model that renders _to_
markdown. The text in the editor is the text in the file, byte for byte, at all times.
Anything that looks rendered is a decoration drawn over that text.

**Live preview, not WYSIWYG.** A construct draws itself while the caret is somewhere
else, and shows its plain markdown while the caret is inside it. This is the most
CodeMirror can do, and it is why formatting marks reappear under the cursor instead of
disappearing for good.

**Vim is always on.** `@replit/codemirror-vim` is installed unconditionally, and normal
mode is the default state. A status bar can follow the mode with `vimModeCallback`, which
receives vim's own names: `normal`, `insert`, `visual` or `replace`. It is also called once
when the editor is created, so the bar has a value before the first keystroke.

```ts
initEditor(parent, doc, {
  vimModeCallback: (mode) => {
    statusbar.textContent = mode;
  },
});
```

## Options

| Option            | Default        | What it does                                                           |
| ----------------- | -------------- | ---------------------------------------------------------------------- |
| `theme`           | `chinuuLight`  | Starting colour theme.                                                 |
| `features`        | every built-in | Which capabilities to install. Pass `[]` for bare markdown.            |
| `menu`            | none           | Extra rows for the right-click menu.                                   |
| `mode`            | `"live"`       | `"live"` or `"reading"`.                                               |
| `blinkCursor`     | `true`         | Whether the caret blinks.                                              |
| `vimModeCallback` | none           | Called on every vim mode change, for a status bar.                     |
| `resolveImage`    | none           | Turns an image path from the note into something the webview can load. |
| `resolveWikiLink` | none           | Turns a wiki link target into a URL your app understands.              |

## The handle

`initEditor` returns an object for driving the running editor.

```ts
editor.view; // the EditorView, for anything not covered here
editor.getValue();
editor.setValue(doc);
editor.setTheme(tokyoNight);
editor.setFeatures(myFeatures); // install or remove capabilities at runtime
editor.setMenu(sections);
editor.setMode("reading");
editor.setCursorBlink(false);
editor.destroy();
```

## View modes

| What you get                          | `live` | `reading` |
| ------------------------------------- | ------ | --------- |
| Markdown is rendered                  | yes    | yes       |
| Markdown source shows under the caret | yes    | never     |
| Can be edited                         | yes    | no        |

**`live`** is the editor you work in. Markdown renders in place, and any construct
reveals its source when the caret moves into it.

**`reading`** renders everything and cannot be edited.

Enforcing that took more than it looks like it should. Setting `EditorState.readOnly`
and `editable: false` is not enough on its own, because both of those are only
consulted by commands. An extension that dispatches its own change, such as the
checkbox handler or the table library, goes straight past them. What actually
enforces read-only is a filter on transactions, `EditorState.transactionFilter`, which
rejects any change to the document.

Reading mode also hides the table library's drag handles and makes checkboxes
non-interactive. That part is about honesty rather than security: if the filter
refused the edit but the control still looked clickable, the editor would read as
broken instead of read-only.

## Images and wiki links

The package knows nothing about your file system or your note names, so it contains no
navigation code of its own. Two options exist to cover that gap.

**`resolveImage`.** A note says `![fig](./fig.png)`, and that path is relative to the
_note_, while a webview's origin is the _application_. Return a Tauri `asset:` URL or a
blob URL. With no resolver the path is used as written, which is correct when a server
publishes notes and their images from one directory, and wrong everywhere else.

**`resolveWikiLink`.** `[[another-note]]` names a note rather than a location, so only
your app knows what it should open. Return an href in a scheme you intercept. With no
resolver the link is styled but inert, because there is no path to guess at.

Both results are checked before use. `resolveImage` allows a list of schemes and
refuses everything else. `resolveWikiLink` additionally allows relative hrefs, since
your own code produced them. The check exists because the resolver is trustworthy but
its _input_ is document text, so a note containing `[[javascript:...]]` must not be
able to hand your app a live script URL.

## Theming

Every colour is a CSS variable named `--chinuu-*`, set on the editor root. A theme is
therefore just data, and one file holds one theme:

```ts
export const myTheme: ChinuuTheme = {
  name: "My Theme",
  dark: false,
  ...chinuuLight,
  background: "#fdfdfd",
  link: "#0b6",
};
```

`ThemeTokens` describes the palette, `TOKEN_NAMES` lists every key in it, and
`themeVariables(theme)` turns a theme into an extension. `builtinThemes` holds the four
that ship: `chinuuLight`, `tokyoNight`, `tokyoNightStorm` and `tokyoNightLight`.
`setTheme` applies a theme to a running editor, so nothing needs rebuilding.

## Adding a capability

A **feature** is one capability, described by one object. It bundles everything that
capability needs: parser support, view extensions, decoration rules and menu rows.
Because it is bundled, removing the feature from the list removes all of it, and adding
a capability never means opening `editor.ts`.

```ts
export interface EditorFeature {
  readonly id: string;
  readonly markdown?: MarkdownExtension; // new syntax for the parser
  readonly codeLanguages?: CodeLanguages; // fence info string to language
  readonly extensions?: readonly Extension[]; // keymaps and event handlers
  readonly rules?: readonly DecorationRule[]; // decorations
  readonly menu?: readonly MenuSection[]; // right-click menu rows
}
```

Register the feature in `src/features/index.ts`. That file is the entire plug and
unplug surface for the editor:

```ts
export const builtinFeatures = {
  // ...
  "wiki-links": wikiLinksFeature,
} as const;

export const defaultFeatures = Object.values(builtinFeatures);
```

`initEditor({ features })` and `setFeatures` accept any subset of that record, so a
host can offer its own feature picker without touching this package.

### Writing a decoration rule

A rule answers one question: when the parser finds this kind of node, how should it be
drawn? Each rule lists the node names it cares about, and the plugin dispatches to the
matching rules during a single walk of the syntax tree. Rules never walk the tree
themselves, so adding one costs a map lookup rather than another pass over the document.

```ts
const wikiLinkRule: DecorationRule = {
  id: "wiki-link",
  nodes: [WIKI_LINK],

  build({ from, to, text, reveals, add }) {
    const parts = wikiLinkParts(text);
    if (!parts) return; // no target, so leave the source alone

    if (reveals(from, to)) {
      // caret is inside, so show `[[target|alias]]`
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
```

The `build` function receives a `RuleContext`. It is deliberately small, with no access
to the whole editor state, so that a rule can be understood on its own:

| Member                          | What it gives you                                    |
| ------------------------------- | ---------------------------------------------------- |
| `node`, `name`, `from`, `to`    | the matched node                                     |
| `parent`                        | the enclosing node's name and span                   |
| `text`                          | the source text of this node                         |
| `lineAt(pos)`                   | line number, span and text                           |
| `reveals(from, to)`             | whether this span should show its markdown right now |
| `add(decoration, from, to)`     | add a mark                                           |
| `addLineClass(line, className)` | add a line class, duplicates ignored                 |
| `addLine(line, decoration)`     | add one decoration to a line, duplicates kept        |

Three helpers build the decorations themselves: `hidden` removes a range,
`lineClass(name)` styles a whole line, and `markClass(name)` styles a span inside one.

**Never check the view mode yourself.** `reveals` is the only mode-aware thing a rule
needs, and it already handles reading mode. A rule that reads `viewMode` directly will
sooner or later be wrong in a mode you did not consider.

**Give every class a name in `CLASS`.** Rules apply `cm-md-*` classes and
`theme/styles.ts` styles them. Declaring each string once in `classes.ts` means a typo
becomes a compile error instead of an element that quietly has no styling.

### Where decorations live: inline and block

Decorations are collected in two places, and CodeMirror decides which decorations each
place is allowed to produce. Choosing the wrong one is a crash, not a strange render.

- **`inline`**, the default, goes to a view plugin. This is for marks, line classes and
  replacements that stay inside one line.
- **`block`** goes to a state field, a CM6 `StateField`. Use it for block widgets and
  for replacements that may cross a line break.

Hand a block widget or a multi-line replacement to the plugin and CodeMirror throws
`Block decorations may not be specified via plugins`. The state field is what makes
them legal.

Worth noting that a rule does not have to be a block widget to need `block` scope.
Display math (`$$...$$`) sets `scope: "block"` only because a paragraph's inline
section can span lines, so its replacement might cross one. The widget itself is still
an ordinary inline widget, since `$$` can sit in the middle of a paragraph.

### Teaching the parser new syntax

Syntax that is neither CommonMark nor GFM needs an inline parser of its own.
`wikiLinks.ts` and `math/parser.ts` are the two worked examples in this package.

```ts
const wikiLinkParser: InlineParser = {
  name: "WikiLink",
  before: "Link", // usually the whole problem, see below
  parse(cx, next, pos) {
    /* ... */
  },
};

const wikiLinkParserExtension: MarkdownConfig = {
  defineNodes: [WIKI_LINK],
  parseInline: [wikiLinkParser],
};
```

**Precedence is usually the whole problem.** The parsers built in are `Escape`,
`Entity`, `InlineCode`, `HTMLTag`, `Emphasis`, `HardBreak`, `Link` and `Image`, and
without a `before` or `after` your parser is appended after all of them. `Link` claims
the `[` character, so `[[note]]` was parsed as a reference link for as long as the wiki
link parser ran behind it. That is why it used to render as ordinary prose that merely
_looked_ like a link. Setting `before: "Link"` is what fixed it.

**Then decline carefully.** A parser that fires too readily will swallow a whole
paragraph, and you will see the symptom far away from the cause. Two examples from this
codebase:

- Math refuses to close on a `$` that follows a space or precedes a digit, so that
  `$5 and $6` stays prose, and it refuses an unterminated `$` rather than eating the
  rest of the line.
- Wiki links refuse a `]` that is not part of a `]]`, and refuse a line break, so a
  target can never stretch across a paragraph.

Finally, the node names you list in `defineNodes` have to match the ones your rules
listen for. That pairing is the contract between the parser and the rule.

### Feature extensions

`extensions` is the list of CM6 extensions a feature installs: keymaps, event handlers,
view plugins, themes. They stay installed for as long as the feature is in the set, and
`setFeatures` swaps the whole list at once.

Not everything fits the decoration system. The table grid is an extension rather than a
rule, because the library owns its own DOM and its own nested editor.

### Menu rows

Menu sections become rows in the right-click menu, and each row opens a flyout holding
its items. `MenuItem.run` is called with the editor view.

```ts
const menu: MenuSection[] = [
  {
    id: "note",
    label: "Note",
    icon: ["M4 3h8v10H4z"], // SVG path data, never markup
    items: [
      {
        id: "insert-link",
        label: "Link",
        hint: "⌘K",
        run: ({ view }) => insertLink(view),
      },
    ],
  },
];
```

Icons are arrays of SVG path data, drawn through `createElementNS`. Nothing in this
package assigns `innerHTML`, and it should stay that way.

`editor.ts` assembles the built-in menu from feature contributions plus the `menu`
option, so a host can add an action without writing a feature at all. Reach for
`EditorFeature.menu` instead when the capability also needs parser support or
decorations, so that the whole thing can be plugged and unplugged together.

## Pitfalls

These are traps CodeMirror does not warn you about, and in each case the symptom shows
up a long way from the cause. Each entry says what you would notice, why it happens, and
where the fix already lives in this codebase, so you can read the real thing rather than
take my word for it.

Most of them come from a single fact: **a widget is not part of the document** as far as
CodeMirror is concerned. It is an opaque element standing in for a range of text, so
event handling, measurement and cursor placement all need care around it.

### A widget's checkbox or dropdown stops responding

**What you would see:** you click a checkbox inside a rendered widget and nothing
happens. There is no error and nothing in the console, and the element looks completely
normal. A `<select>` behaves the same way, with your `change` handler never running.

**Why:** every widget has an `ignoreEvent()` method, and it returns `true` by default.
That value answers one question, which is whether CodeMirror should stay out of events
inside this widget. The trap is what "stay out" turns out to mean. CodeMirror does not
merely skip its own handling of the event, it stops dispatching events from that point
entirely, and your `EditorView.domEventHandlers` are called from inside that same
dispatch. So the default quietly switches your handlers off.

CodeMirror's own event entry point shows where they are lost:

```js
// @codemirror/view, handleEvent
if (
  !eventBelongsToEditor(this.view, event) ||
  this.ignoreDuringComposition(event)
)
  return;
// ...
this.runHandlers(event.type, event);
```

The first line gives up early when the event does not belong to the editor, and
`runHandlers` on the last line is what calls your handlers. Give up before reaching it
and they never run.

**What to do:** return `false` when the widget contains something the user operates.
Keep the default when it contains something the user only looks at, such as a link or an
image. The reason is that `false` also tells CodeMirror the press belongs to it, so it
moves the caret into the widget. For a link that is fatal, because the caret arriving
reveals the markdown, which replaces the anchor in the middle of the click, so the
navigation never finishes.

**Where it is in the code:** all four choices are here, each with its reasoning written
next to it.

- `src/features/taskLists.ts:24` returns `false`, because the toggle handler needs to
  see the press.
- `src/features/codeBlocks.ts:52` returns `false` for the same reason, on the language
  `<select>` whose `change` handler would otherwise never fire.
- `src/features/links.ts:57` keeps the default on purpose, and the comment explains the
  trade described above. `src/features/images.ts:59` and `src/features/wikiLinks.ts:132`
  make the same choice.

### A widget ignores a setting you passed in

**What you would see:** you add an option such as `resolveImage`, and the widget that
should use it carries on as if it did not exist.

**Why:** the setting lives on the editor state, but the rule that builds the widget
receives a `RuleContext`, which has no access to that state on purpose. The widget's
`toDOM` method is called by CodeMirror itself, so it is the first place where the view
is available:

```ts
// src/features/images.ts, ImageWidget
const resolve = view.state.facet(imageResolver);
image.src = resolve ? resolve(this.src) : this.src;
```

**One consequence to know about:** `eq()` decides whether an existing widget can be
reused, and all it can see is the data the widget was built from. Change the resolver
alone and CodeMirror considers the widget unchanged, so `toDOM` never runs again and the
old `src` stays on screen. Something about the widget itself has to change, either the
text or a fresh install of the features.

**Where it is in the code:** `src/features/images.ts:66` and
`src/features/wikiLinks.ts:139` are the two widgets that read a setting this way.
`src/features/wikiLinks.ts:144` also shows the resulting href being checked before use.

### A reading-mode style silently does nothing

**What you would see:** you add a rule to `theme/styles.ts` and nothing changes. The
computed style still shows the old value, and no tool complains.

**Why:** `applyModeClass` puts the class `cm-md-reading` on the editor root, which is
the same element that `EditorView.theme` scopes its selectors to. A theme key is
treated as a _descendant_ of the editor, so the first line below looks for the class on
a child of the root, where it will never be:

```ts
[`.${CLASS.readingMode} .tbl-handle`]: { display: "none" },   // compiles to .gen .cm-md-reading ...
[`&.${CLASS.readingMode} .tbl-handle`]: { display: "none" },  // compiles to .gen.cm-md-reading ...
```

Leading with `&` refers to the editor element itself, which is what you want here.

**Where it is in the code:** `src/menu/menuTheme.ts:134` was already using the correct
form for the caret layer and the active line. The rules at the end of `documentTheme` in
`src/theme/styles.ts` use it for the table handles and the checkbox.

### Line numbers counted in CSS are wrong

**What you would see:** line numbers that are off by however far you have scrolled.
Correct at the top of the document, wrong everywhere else.

**Why:** CodeMirror only builds DOM for the lines it has measured, so the first rendered
line is wherever the viewport happens to be. A CSS counter counts rendered elements, so
it always starts at 1 at the top of the _viewport_ rather than the top of the
_document_. The fix is to stop counting and instead put the real number on each line as
an attribute, then let CSS read it back.

**Where it is in the code:** `src/features/codeBlocks.ts:156` does exactly this for the
lines inside a fence, putting the number into `data-code-line` for a `::before` to read.
`src/features/codeBlocks.ts:133` explains why the number lives in a pseudo-element at
all: it stays out of selections and the clipboard, which a gutter's text would not.

If you ever number lines by walking `view.visibleRanges` rather than by decorating a
block, one further caution: `Decoration.set` rejects two line decorations at the same
position, and two ranges can hand you the same boundary line.

### Writing a click handler when a real element would do

**Why it is worth avoiding anyway:** a real `<a href>` gets hover, the pointer cursor,
copy-link-address, middle-click and keyboard activation from the browser for free.
Hand-written navigation has to map a click back to a document position, and every part
of that can fail on its own, silently.

**Where it is in the code:** `src/features/links.ts`, `src/features/images.ts` and
`src/features/wikiLinks.ts` all build plain `<a>` and `<img>` elements for this reason.
An earlier link renderer that did it the other way, with a click handler and a
`posAtCoords` lookup, was replaced rather than debugged.

## Testing

```sh
pnpm format           # prettier format
pnpm lint             # eslint, including the type-aware rules
pnpm typecheck        # tsc --noEmit
pnpm test:browser     # vite dev server over tests/index.html
```
