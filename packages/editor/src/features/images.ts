import { Facet } from "@codemirror/state";
import { Decoration, type EditorView, WidgetType } from "@codemirror/view";
import { CLASS } from "../classes.ts";
import type { DecorationRule } from "../livePreview/types.ts";
import type { EditorFeature } from "./types.ts";
import { hasControlChars, mediaParts, schemeOf } from "./media.ts";

/**
 * Images, rendered inline as `<img>`.
 *
 * The destination in a document is a path relative to the note, which a webview
 * cannot resolve on its own its origin is the application, not the folder
 * holding the note. `imageResolver` is where a host maps that path onto something
 * loadable, such as Tauri's `asset:` protocol or a blob URL.
 *
 * With no resolver the path is used verbatim, which is correct when a server
 * hands out the note and its images from the same directory.
 */
export type ImageResolver = (src: string) => string;

export const imageResolver = Facet.define<ImageResolver, ImageResolver | null>({
  combine: (values) => values[0] ?? null,
});

/** Schemes an `src` may carry. A path with no scheme is relative, and allowed. */
const SAFE_SCHEMES = new Set(["http:", "https:", "asset:", "blob:"]);

/**
 * Whether a destination may become an `src`.
 *
 * The destination is document text, so a note can contain anything at all.
 * `data:` is admitted only for images: an SVG loaded through `<img>` cannot run
 * script, but keeping the media type narrow costs nothing.
 */
const isSafeSrc = (src: string): boolean => {
  const trimmed = src.trim();
  if (trimmed.length === 0) return false;
  if (hasControlChars(trimmed)) return false;

  const scheme = schemeOf(trimmed);
  if (!scheme) return true;
  if (scheme === "data:") return /^data:image\//i.test(trimmed);
  return SAFE_SCHEMES.has(scheme);
};

class ImageWidget extends WidgetType {
  constructor(
    private readonly src: string,
    private readonly alt: string,
  ) {
    super();
  }

  eq(other: ImageWidget): boolean {
    return other.src === this.src && other.alt === this.alt;
  }

  /**
   * `ignoreEvent` is deliberately left at its default of `true`, so pressing an
   * image does not move the caret onto it. The caret moving there would reveal the
   * source and swap the image out from under the pointer.
   *
   * The resolver is read here rather than when the rule runs, because a widget is
   * the first place with the view in hand.
   */
  toDOM(view: EditorView): HTMLElement {
    const resolve = view.state.facet(imageResolver);
    const image = document.createElement("img");
    image.className = CLASS.image;
    image.src = resolve ? resolve(this.src) : this.src;
    image.alt = this.alt;
    // Otherwise a press-and-move starts a native drag of the image.
    image.draggable = false;
    return image;
  }
}

const imageRule: DecorationRule = {
  id: "image",
  nodes: ["Image"],

  build({ from, to, node, text, reveals, add }) {
    // Editing: show `![alt](src)` so it can be changed.
    if (reveals(from, to)) return;

    const parts = mediaParts(node.node, text);
    // An unsafe destination stays as source text rather than becoming a request.
    if (!parts || !isSafeSrc(parts.url)) return;

    add(
      Decoration.replace({ widget: new ImageWidget(parts.url, parts.label) }),
      from,
      to,
    );
  },
};

export const imagesFeature: EditorFeature = {
  id: "images",
  rules: [imageRule],
};
