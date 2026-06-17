/**
 * Inline `pptx-media://{hash}` references in an SVG document as
 * base64 data URLs.
 *
 * The viewer rewrites media URLs to short-lived `blob:` URLs that the
 * browser scopes to the creating document. The moment the SVG leaves
 * that document — copied into a print window, fed to a canvas for PDF
 * rasterization — those URLs stop resolving and the slide loses every
 * embedded image. Data URLs are larger but document-portable, which
 * matches the offline / no-server constraint the viewer ships under.
 */

import type { MediaBlob } from "../types.js";

const TRANSPARENT_GIF =
  "data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=";

/**
 * Replace every `pptx-media://{hash}` URL in `svg` with a fully-
 * inlined `data:` URL pulled from `media`. Hashes that aren't in the
 * map fall back to a 1×1 transparent GIF — same defensive behaviour
 * the live viewer uses.
 */
export function inlineMediaAsDataUrls(
  svg: string,
  media: Map<string, MediaBlob>,
): string {
  if (svg.indexOf("pptx-media://") < 0) return svg;
  const cache = new Map<string, string>();
  return svg.replace(/pptx-media:\/\/([0-9a-f]+)/g, (_match, hash: string) => {
    let url = cache.get(hash);
    if (url !== undefined) return url;
    const blob = media.get(hash);
    if (!blob) {
      cache.set(hash, TRANSPARENT_GIF);
      return TRANSPARENT_GIF;
    }
    url = `data:${blob.mime};base64,${bytesToBase64(blob.bytes)}`;
    cache.set(hash, url);
    return url;
  });
}

/**
 * Convert a `Uint8Array` to a base64 string without going through
 * `String.fromCharCode(...bytes)` (which blows the JS argument
 * stack at ~100 KB images on most engines).
 */
function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  const chunkSize = 0x8000;
  for (let i = 0; i < bytes.length; i += chunkSize) {
    const chunk = bytes.subarray(i, i + chunkSize);
    binary += String.fromCharCode.apply(null, Array.from(chunk));
  }
  return btoa(binary);
}
