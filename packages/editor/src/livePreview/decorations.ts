import { Decoration } from "@codemirror/view";

/**
 * Shared decoration factories.
 *
 * CM6 compares decorations by identity, so reusing one instance per class lets
 * it skip work it would otherwise repeat for every line or every mark.
 */

/** Replaces a range with nothing the basis of every "hidden mark". */
export const hidden = Decoration.replace({});

const lineClasses = new Map<string, Decoration>();
const markClasses = new Map<string, Decoration>();

/** One shared `Decoration.line` per class instead of allocating per line. */
export const lineClass = (className: string): Decoration => {
  let decoration = lineClasses.get(className);
  if (!decoration) {
    lineClasses.set(
      className,
      (decoration = Decoration.line({ class: className })),
    );
  }
  return decoration;
};

/** One shared `Decoration.mark` per class, for styling an existing range. */
export const markClass = (className: string): Decoration => {
  let decoration = markClasses.get(className);
  if (!decoration) {
    markClasses.set(
      className,
      (decoration = Decoration.mark({ class: className })),
    );
  }
  return decoration;
};
