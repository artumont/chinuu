import { syntaxTree } from "@codemirror/language";
import { Decoration, EditorView, WidgetType } from "@codemirror/view";
import { CLASS } from "../classes.ts";
import { hidden, markClass } from "../livePreview/decorations.ts";
import type { DecorationRule } from "../livePreview/types.ts";
import type { EditorFeature } from "./types.ts";

/** GFM allows `[x]` or `[X]`, with or without the space. */
const CHECKED = /\[\s*[xX]\]/;

class TaskCheckboxWidget extends WidgetType {
  constructor(private readonly checked: boolean) {
    super();
  }

  eq(other: TaskCheckboxWidget): boolean {
    return other.checked === this.checked;
  }

  /**
   * Let the event through. Returning `true` would make CM6 swallow the click,
   * and the toggle is driven by the editor's own handler below.
   */
  ignoreEvent(): boolean {
    return false;
  }

  toDOM(): HTMLElement {
    const box = document.createElement("input");
    box.type = "checkbox";
    box.className = CLASS.taskCheckbox;
    box.checked = this.checked;
    box.setAttribute(
      "aria-label",
      this.checked ? "Completed task" : "Incomplete task",
    );
    return box;
  }
}

/**
 * Replaces `[ ]` / `[x]` with a real checkbox.
 *
 * Gated on the selection like every other construct, so putting the caret inside
 * the marker brings the plaintext back and it can be edited by hand. Clicking the
 * box toggles it instead; because that dispatches a document change rather than
 * moving the caret, the checkbox stays rendered.
 */
const taskMarkerRule: DecorationRule = {
  id: "task-checkbox",
  nodes: ["TaskMarker"],

  build({ from, to, text, reveals, add }) {
    if (reveals(from, to)) return;
    add(
      Decoration.replace({
        widget: new TaskCheckboxWidget(CHECKED.test(text)),
      }),
      from,
      to,
    );
  },
};

/** Drops the bullet in front of a task, so it reads as `☐ text` rather than `- ☐ text`. */
const taskListMarkRule: DecorationRule = {
  id: "task-list-mark",
  nodes: ["ListMark"],

  build({ from, to, node, reveals, add }) {
    // Only for list items that actually carry a task; a plain bullet keeps its `-`.
    if (!node.node.parent?.getChild("Task")) return;
    // Gated like every other construct. Without this the `-` was hidden
    // unconditionally, so it never came back for editing not even with the caret
    // sitting on its own line.
    if (reveals(from, to)) return;
    add(hidden, from, to);
  },
};

/**
 * Dims and strikes the text of a completed task.
 *
 * Starts *after* the `TaskMarker`, so the replaced checkbox is not inside the
 * decorated inline box. Decoration geometry around a replaced child with a
 * negative `vertical-align` is what browsers get wrong, and the visible symptom
 * was the strike landing a line too low through the following line's text.
 *
 * Also clipped to the task's first line, so a task whose node spans a line break
 * cannot draw the strike across the break.
 */
const taskDoneRule: DecorationRule = {
  id: "task-done",
  nodes: ["Task"],

  build({ from, to, text, node, lineAt, add }) {
    if (!CHECKED.test(text)) return;

    const marker = node.node.getChild("TaskMarker");
    const start = marker ? marker.to : from;
    const end = Math.min(to, lineAt(from).to);
    if (start >= end) return;

    add(markClass(CLASS.taskDone), start, end);
  },
};

/**
 * Finds the task marker covering `pos` and flips it.
 *
 * The scan is over the whole tree rather than `resolveInner`, because a replaced
 * range has no DOM inside it and `posAtDOM` can report either edge of the
 * widget. Checking containment handles both.
 */
const toggleTaskAt = (view: EditorView, pos: number): boolean => {
  const found: { from: number; to: number; checked: boolean }[] = [];

  syntaxTree(view.state).iterate({
    enter(node) {
      if (node.name !== "TaskMarker") return;
      if (pos < node.from || pos > node.to) return;
      found.push({
        from: node.from,
        to: node.to,
        checked: CHECKED.test(view.state.sliceDoc(node.from, node.to)),
      });
    },
  });

  if (found.length === 0) return false;
  const [target] = found;
  view.dispatch({
    changes: {
      from: target.from,
      to: target.to,
      insert: target.checked ? "[ ]" : "[x]",
    },
  });
  return true;
};

const checkboxClicks = EditorView.domEventHandlers({
  mousedown(event, view) {
    const target = event.target;
    if (!(target instanceof HTMLInputElement)) return false;
    if (!target.classList.contains(CLASS.taskCheckbox)) return false;

    // `posAtDOM` is resolved live, so it stays correct even when CM6 reuses the
    // element for an unchanged widget.
    const toggled = toggleTaskAt(view, view.posAtDOM(target));
    // Stops the native flip and the focus change; the document change rebuilds
    // the widget in the right state.
    if (toggled) event.preventDefault();
    return toggled;
  },
});

export const taskListsFeature: EditorFeature = {
  id: "task-lists",
  rules: [taskMarkerRule, taskListMarkRule, taskDoneRule],
  extensions: [checkboxClicks],
};
