/**
 * Framework-agnostic SVG utilities used by both the React shell and
 * any external host that needs to massage slide markup before mount
 * (e.g. printing, PDF export, in-page embedding).
 */

import type { MediaBlob, SlideSvg } from "./types.js";

/**
 * Pull the slide aspect ratio (width / height) from the SVG's
 * `viewBox`. Returns `null` when the SVG is empty or malformed; the
 * caller should fall back to 16:9 in that case.
 */
export function parseAspect(svg: string): number | null {
  const match = svg.match(/viewBox=["']([^"']+)["']/);
  if (!match) return null;
  const parts = match[1].split(/\s+/).map(Number);
  if (parts.length < 4) return null;
  const w = parts[2];
  const h = parts[3];
  if (!Number.isFinite(w) || !Number.isFinite(h) || w <= 0 || h <= 0) {
    return null;
  }
  return w / h;
}

/**
 * Strip the renderer's intrinsic `width="…"` / `height="…"` attributes
 * off the outer `<svg>` and inject `style="width:100%;height:100%;
 * display:block"` so the SVG fills its container regardless of CSS
 * resolution order. The renderer emits these attributes to keep the
 * deck's native pixel dimensions, but they fight the viewer's
 * fit-to-stage layout when rendered inline.
 */
export function prepareSvg(svg: string): string {
  return svg.replace(/<svg\b([^>]*)>/, (_match, rawAttrs: string) => {
    const stripped = rawAttrs
      .replace(/\swidth=(["'])[^"']*\1/g, "")
      .replace(/\sheight=(["'])[^"']*\1/g, "");
    return `<svg${stripped} style="width:100%;height:100%;display:block">`;
  });
}

/**
 * Rewrite every `id="…"` and matching `url(#…)` / `href="#…"` /
 * `xlink:href="#…"` reference inside an SVG so the IDs become unique
 * within the host document.
 *
 * Why this is needed: HTML id namespace is document-wide. The viewer
 * mounts the same slide SVG simultaneously in the main stage and the
 * thumbnail panel, and renders sibling slides for grid view — every
 * one of those SVGs ships the same internal IDs (`crop-0`, `crop-1`,
 * `tile-0`, …) emitted by `slideglance-renderer`. Browsers resolve
 * `url(#crop-1)` against the *first* matching element in the
 * document, which is not necessarily the one inside the SVG that
 * declares the reference. The visible symptom is clip / mask /
 * gradient bleed across slides — most dramatically, picture frames
 * being clipped to the bounding box of an unrelated slide's tiny
 * thumbnail icon.
 *
 * Pass a different `prefix` per mount site (e.g. `main-{slide}`,
 * `thumb-{slide}`) so siblings don't collide either.
 */
export function uniquifyIds(svg: string, prefix: string): string {
  // Escape any regex meta characters in `prefix` for safety, even
  // though current callers use only alphanumerics + dash.
  const safe = prefix.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  void safe;
  return svg
    .replace(/\bid="([^"]+)"/g, (_m, id: string) => `id="${prefix}-${id}"`)
    .replace(
      /\burl\(['"]?#([^'")]+)['"]?\)/g,
      (_m, id: string) => `url(#${prefix}-${id})`,
    )
    .replace(
      /\b(xlink:)?href="#([^"]+)"/g,
      (_m, ns: string | undefined, id: string) =>
        `${ns ?? ""}href="#${prefix}-${id}"`,
    );
}

/**
 * Pull the first family out of an SVG `font-family` attribute value,
 * stripping the surrounding quotes and trimming whitespace. The
 * renderer emits a chain such as `'Source Sans 3', "Open Sans", arial`
 * — the first entry is what the source PPTX authored, and the rest are
 * fallbacks. Returns `null` when the input is empty or only contains
 * fallback aliases the user didn't write themselves (`sans-serif`,
 * `serif`, `monospace`, etc.) — those are renderer noise, not authored
 * typefaces, and surfacing them in the status bar would mislead.
 */
export function parseFirstFontFamily(value: string): string | null {
  // Splitting on `,` is correct because OOXML typeface names cannot
  // contain commas; the renderer composes the chain with `, ` joiners.
  const head = value.split(",")[0]?.trim();
  if (!head) return null;
  const unquoted = head.replace(/^['"]|['"]$/g, "").trim();
  if (!unquoted) return null;
  const lower = unquoted.toLowerCase();
  if (
    lower === "serif" ||
    lower === "sans-serif" ||
    lower === "monospace" ||
    lower === "cursive" ||
    lower === "fantasy" ||
    lower === "system-ui"
  ) {
    return null;
  }
  return unquoted;
}

/**
 * Replace `pptx-media://{hash}` URLs in the SVG with browser-friendly
 * `blob:` URLs created from the supplied media map. Returns the
 * rewritten SVG plus the array of blob URLs the caller is responsible
 * for revoking when the slide unmounts.
 */
export function rewriteMediaRefs(
  svg: string,
  media: Map<string, MediaBlob>,
): { svg: string; blobUrls: string[] } {
  if (media.size === 0) return { svg, blobUrls: [] };
  const blobUrls: string[] = [];
  const rewritten = svg.replace(
    /pptx-media:\/\/([a-zA-Z0-9_-]+)/g,
    (match, hash: string) => {
      const blob = media.get(hash);
      if (!blob) return match;
      const url = URL.createObjectURL(
        new Blob([blob.bytes as BlobPart], { type: blob.mime }),
      );
      blobUrls.push(url);
      return url;
    },
  );
  return { svg: rewritten, blobUrls };
}

/**
 * Hoist the deck-wide `<style>...@font-face...</style>` block out of
 * the first slide's SVG so the viewer can mount it once at the shadow-
 * root level instead of duplicating tens of MB of base64 across every
 * slide. Returns the extracted CSS body, and rewrites `slides[0].svg`
 * in place to remove the hoisted block. Returns `""` when there is
 * nothing to hoist.
 */
export function extractAndStripFontStyle(slides: SlideSvg[]): string {
  if (slides.length === 0) return "";
  const first = slides[0];
  if (!first || typeof first.svg !== "string") return "";
  const re = /<style\b[^>]*>([\s\S]*?@font-face[\s\S]*?)<\/style>/i;
  const match = first.svg.match(re);
  if (!match) return "";
  const css = match[1] ?? "";
  if (css.trim().length === 0) return "";
  slides[0] = { ...first, svg: first.svg.replace(re, "") };
  return css;
}

/**
 * Extract the bare CSS rules (the `@font-face` declarations) from the
 * `<defs><style>…</style></defs>` block that
 * `slideglance-wasm`'s `fontDefs()` returns.
 *
 * Background: when the host renders slides with `includeFontDefs:false`
 * (the worker controller's path — fonts are inlined once, not per
 * slide), the returned `fontDefs` string still wraps the CSS in an
 * SVG `<defs>` shell so the same payload can be used by
 * `includeFontDefs:true` callers without conditional formatting. Hosts
 * that mount these declarations into `document.head` need just the
 * inner CSS without the SVG wrapper.
 *
 * Returns `""` when the input is empty or contains no `<style>` block.
 */
export function extractFontStyleCss(fontDefs: string): string {
  if (typeof fontDefs !== "string" || fontDefs.length === 0) return "";
  const re = /<style\b[^>]*>([\s\S]*?)<\/style>/i;
  const match = fontDefs.match(re);
  if (!match) return "";
  const css = match[1] ?? "";
  return css.trim();
}
