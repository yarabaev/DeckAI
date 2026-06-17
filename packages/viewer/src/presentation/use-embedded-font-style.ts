/**
 * `useEmbeddedFontStyle` — mount the deck's embedded `@font-face`
 * declarations into a document-scoped `<style>` so the browser's
 * font matcher can resolve family names like `"Noto Sans Bold"`
 * referenced by SVG `<text>` runs.
 *
 * The declarations come from PPTX `<p:embeddedFontLst>` faces
 * extracted by the WASM core (`fontDefs()`), already de-obfuscated
 * and base64-encoded. Without this mount the embedded fonts would
 * only live in the worker's FontFaceSet (worker-local, invisible
 * to `document`), and the SVG would silently fall back to a system
 * sans-serif — wider than the authored face on most hosts and
 * overflowing text frames into adjacent layout regions.
 *
 * The `<style>` element is identified by a single id so re-mounting
 * the component (or swapping decks) replaces the rules atomically;
 * empty CSS removes the element entirely. The `data-slideglance-fonts`
 * attribute is for co-located shells that need to detect or clean
 * up orphan blocks.
 *
 * Eager `document.fonts.load(...)` calls force every declared
 * family to start decoding *now* so the FontUsageIndicator's
 * initial state matches its post-render-everything state. CSS
 * `@font-face` URLs are otherwise lazily fetched only when a text
 * node first references the family, which made the indicator
 * report transient substitutions that disappeared the moment the
 * user switched to grid view.
 */

import { useEffect } from "react";

const STYLE_ELEMENT_ID = "slideglance-deck-fonts";

export function useEmbeddedFontStyle(fontDefsCss: string): void {
  useEffect(() => {
    if (typeof document === "undefined") return;
    const existing = document.getElementById(
      STYLE_ELEMENT_ID,
    ) as HTMLStyleElement | null;
    if (fontDefsCss.length === 0) {
      if (existing) existing.remove();
      return;
    }
    const styleEl: HTMLStyleElement =
      existing ?? document.createElement("style");
    if (!existing) {
      styleEl.id = STYLE_ELEMENT_ID;
      styleEl.dataset.slideglanceFonts = "true";
      document.head.appendChild(styleEl);
    }
    if (styleEl.textContent !== fontDefsCss) {
      styleEl.textContent = fontDefsCss;
    }

    // Eager-load every embedded face so the FontUsageIndicator's
    // first read of `document.fonts.check(...)` matches its
    // eventual steady state. Errors are swallowed — a partial load
    // shouldn't block the rest of the deck from reporting correctly.
    if (document.fonts) {
      const families = new Set<string>();
      const matches = fontDefsCss.matchAll(
        /font-family\s*:\s*['"]([^'"]+)['"]/gi,
      );
      for (const match of matches) {
        families.add(match[1]);
      }
      for (const family of families) {
        const escaped = family.replace(/"/g, '\\"');
        document.fonts.load(`12px "${escaped}"`).catch(() => {
          // Ignore — single-face decode failure is non-fatal.
        });
      }
    }

    return () => {
      // Component-unmount cleanup: remove the deck-scoped `<style>`
      // so we don't leak base64 font payloads when the host tears
      // down the viewer (e.g. SPA route change, modal close).
      styleEl.remove();
    };
  }, [fontDefsCss]);
}
