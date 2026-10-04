import type { MenuSection } from "../../menu/types.ts";
import type { EditorFeature } from "../types.ts";
import {
  applyLinePrefix,
  insertHorizontalRule,
  insertLink,
  insertTable,
  toggleComment,
  toggleFence,
  toggleWrap,
} from "./commands.ts";

/**
 * Text formatting, exposed through the right-click menu.
 *
 * Three top-level rows with icons, and the actions behind a chevron the menu
 * stays that shape however many actions get registered, which is why the actions
 * live in flyouts rather than in one long list.
 *
 * The commands are exported as well, so a keymap or a toolbar can reuse them
 * without going through the menu.
 */

/** Path data for a 16x16 stroke icon; see `iconNode` in `menu/contextMenu.ts`. */
const ICON_FORMAT = ["M9.5 2.5l4 4L6 14H2v-4z", "M2 14h12"];
const ICON_PARAGRAPH = [
  "M7 2.5h6",
  "M10 2.5v11",
  "M7 2.5a3.25 3.25 0 0 0 0 6.5h3",
];
const ICON_INSERT = ["M2 4h7", "M2 8h7", "M2 12h4", "M12 9.5v5", "M9.5 12h5"];

const format: MenuSection = {
  id: "format",
  label: "Format",
  icon: ICON_FORMAT,
  items: [
    {
      id: "bold",
      label: "Bold",
      hint: "**",
      run: ({ view }) => toggleWrap(view, "**"),
    },
    {
      id: "italic",
      label: "Italic",
      hint: "_",
      run: ({ view }) => toggleWrap(view, "_"),
    },
    {
      id: "strikethrough",
      label: "Strikethrough",
      hint: "~~",
      run: ({ view }) => toggleWrap(view, "~~"),
    },
    {
      id: "inline-code",
      label: "Inline code",
      hint: "`",
      run: ({ view }) => toggleWrap(view, "`"),
    },
    {
      id: "link",
      label: "Link",
      hint: "[]()",
      run: ({ view }) => insertLink(view),
    },
    {
      id: "comment",
      label: "Comment",
      hint: "<!-- -->",
      run: ({ view }) => toggleComment(view),
    },
  ],
};

const paragraph: MenuSection = {
  id: "paragraph",
  label: "Paragraph",
  icon: ICON_PARAGRAPH,
  items: [
    {
      id: "h1",
      label: "Heading 1",
      hint: "#",
      run: ({ view }) => applyLinePrefix(view, "# "),
    },
    {
      id: "h2",
      label: "Heading 2",
      hint: "##",
      run: ({ view }) => applyLinePrefix(view, "## "),
    },
    {
      id: "h3",
      label: "Heading 3",
      hint: "###",
      run: ({ view }) => applyLinePrefix(view, "### "),
    },
    {
      id: "bullet",
      label: "Bullet list",
      hint: "-",
      run: ({ view }) => applyLinePrefix(view, "- "),
    },
    {
      id: "ordered",
      label: "Numbered list",
      hint: "1.",
      run: ({ view }) => applyLinePrefix(view, "1. "),
    },
    {
      id: "task",
      label: "Task",
      hint: "- [ ]",
      run: ({ view }) => applyLinePrefix(view, "- [ ] "),
    },
    {
      id: "quote",
      label: "Quote",
      hint: ">",
      run: ({ view }) => applyLinePrefix(view, "> "),
    },
  ],
};

const insert: MenuSection = {
  id: "insert",
  label: "Insert",
  icon: ICON_INSERT,
  items: [
    {
      id: "table",
      label: "Table",
      hint: "2 x 2",
      run: ({ view }) => insertTable(view),
    },
    {
      id: "rule",
      label: "Horizontal rule",
      hint: "---",
      run: ({ view }) => insertHorizontalRule(view),
    },
    {
      id: "code-block",
      label: "Code block",
      hint: "```",
      run: ({ view }) => toggleFence(view),
    },
  ],
};

export const formatFeature: EditorFeature = {
  id: "format",
  menu: [format, paragraph, insert],
};
