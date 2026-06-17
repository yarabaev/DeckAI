/**
 * Internal types shared by the React shell and its sub-components.
 *
 * Distinct from `../types.ts` (the public package surface):
 * - `../types.ts` exports types every consumer of `@slideglance/viewer`
 *   sees (`SlideController`, `RenderedSlide`, `MediaBlob`, ...).
 * - this file is for module-private structures the shell shares with
 *   `Thumbnail` / `NotesPanel` / `GridView` and is not re-exported.
 */

import type { SlideMeta } from "../types.js";

/**
 * One slide's cached render output. The shell builds this lazily as
 * the user navigates and reuses the cached entry on every subsequent
 * visit so the second pass is paint-only — no parser / renderer round-
 * trip and no font-resolution cost.
 *
 * - `svg`: raw SVG string the worker produced.
 * - `preparedSvg`: the same SVG after `prepareSvg` runs (id-namespacing,
 *   media-ref rewriting, font-style stripping).
 * - `blobUrls`: every `blob:` URL the prepared SVG references; the
 *   shell `URL.revokeObjectURL`s them when the deck is closed so the
 *   page doesn't leak the in-memory media bytes.
 * - `meta`: per-slide metadata (notes / layout / section name) the
 *   status bar + notes panel render.
 */
export interface CachedSlide {
  svg: string;
  preparedSvg: string;
  blobUrls: string[];
  meta: SlideMeta;
}
