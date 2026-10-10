import type { ImageResolver } from "./images.ts";
import { schemeOf } from "./media.ts";

/**
 * A ready-made `resolveImage` for a Tauri host.
 *
 * The editor cannot build an `asset:` URL on its own. It never sees the file
 * system, and `@tauri-apps/api` is deliberately not a dependency of this
 * package, so the piece that talks to Tauri stays in the application. What the
 * editor can do is own the composition, which is what this helper is:
 *
 * ```ts
 * import { convertFileSrc } from "@tauri-apps/api/core";
 * import { initEditor, tauriImageResolver } from "@chinuu/editor";
 *
 * initEditor(parent, markdown, {
 *   resolveImage: tauriImageResolver({
 *     // `noteDir` is known by the app; the join has to be synchronous, because a
 *     // widget builds its `src` during a render pass and cannot await.
 *     resolvePath: (src) => `${noteDir}/${src}`,
 *     convertFileSrc,
 *     version: (path) => mtimes.get(path),
 *   }),
 * });
 * ```
 *
 * `resolvePath` decides how a destination written in the note becomes an
 * absolute filesystem path. It returns `null` when it cannot, which leaves the
 * destination as written rather than inventing one.
 *
 * `version` is optional and exists for one Tauri behaviour: the asset protocol
 * caches responses by URL, so replacing a file on disk under the same name keeps
 * serving the old bytes. Returning a token that changes with the file (an mtime,
 * a content hash) appends `?v=…` and makes the webview re-read it.
 *
 * Note the shape the Tauri config still needs, since a helper cannot supply it:
 * `app.security.assetProtocol.enable` has to be on with a `scope` covering the
 * vault, and an enabled CSP needs `asset:` and `http://asset.localhost` in
 * `img-src` (Windows resolves the protocol over HTTP).
 */
export interface TauriImageResolverOptions {
  /**
   * Maps the destination as written in the note onto an absolute path.
   *
   * Synchronous on purpose. A decoration widget builds its `src` inside a render
   * pass, so `@tauri-apps/api/path`'s promise-returning `join` cannot be used
   * here directly; resolve the note's directory once when the note opens and
   * concatenate.
   */
  readonly resolvePath: (src: string) => string | null;
  /** Tauri's `convertFileSrc`, injected so this package needs no Tauri import. */
  readonly convertFileSrc: (path: string) => string;
  /** A token that changes when the file does, for cache-busting. `null` skips it. */
  readonly version?: (
    absolutePath: string,
  ) => string | number | null | undefined;
}

/**
 * Schemes a webview can already fetch by itself, so they never need the asset
 * protocol. `asset:` is included because a host may resolve some paths itself
 * and leave the result in the document.
 */
const WEBVIEW_SCHEMES = new Set([
  "http:",
  "https:",
  "data:",
  "blob:",
  "asset:",
]);

/** Appends a cache-busting token without clobbering an existing query. */
const withVersion = (url: string, token: string | number): string => {
  const value = String(token);
  try {
    const parsed = new URL(url);
    parsed.searchParams.set("v", value);
    return parsed.toString();
  } catch {
    // `asset://` parses in every engine this package targets, but a plain string
    // fallback costs nothing and keeps a malformed URL from throwing mid-render.
    return `${url}${url.includes("?") ? "&" : "?"}v=${encodeURIComponent(value)}`;
  }
};

export const tauriImageResolver = (
  options: TauriImageResolverOptions,
): ImageResolver => {
  const { resolvePath, convertFileSrc, version } = options;

  return (src) => {
    // A destination that already names something loadable is left verbatim, so a
    // note linking a remote image keeps working.
    const scheme = schemeOf(src);
    if (scheme && WEBVIEW_SCHEMES.has(scheme)) return src;

    const absolute = resolvePath(src);
    if (!absolute) return src;

    const url = convertFileSrc(absolute);
    const token = version?.(absolute);
    return token === null || token === undefined
      ? url
      : withVersion(url, token);
  };
};
