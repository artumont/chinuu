import { render } from "katex";
// KaTeX's own stylesheet. Imported here rather than left to the consumer so the
// rendered math is never unstyled. Requires a bundler that understands CSS
// imports (Vite does); `vite/client` types in `tsconfig.json` cover the import.
import "katex/dist/katex.min.css";
import { WidgetType } from "@codemirror/view";
import { CLASS } from "../../classes.ts";

/**
 * Renders TeX with KaTeX.
 *
 * KaTeX is a direct dependency of this package rather than of
 * `@chinuu/preview`: the preview package renders whole documents for a separate
 * pane, while this renders a single expression in place. Same library, two
 * different jobs.
 */
export class MathWidget extends WidgetType {
  constructor(
    private readonly source: string,
    private readonly display: boolean,
  ) {
    super();
  }

  eq(other: MathWidget): boolean {
    return other.source === this.source && other.display === this.display;
  }

  /** Let clicks through so the caret can enter and reveal the source. */
  ignoreEvent(): boolean {
    return false;
  }

  toDOM(): HTMLElement {
    const element = document.createElement(this.display ? "div" : "span");
    element.className = this.display ? CLASS.mathDisplay : CLASS.math;
    try {
      render(this.source, element, {
        displayMode: this.display,
        throwOnError: false,
      });
    } catch {
      // Malformed TeX must never take the editor down: show the source instead.
      element.classList.add(CLASS.mathError);
      element.textContent = this.display
        ? `$$${this.source}$$`
        : `$${this.source}$`;
    }
    return element;
  }
}
