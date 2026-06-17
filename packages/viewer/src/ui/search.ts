import type { SlideSvg } from "../types.js";

/** One search hit. */
export interface SearchHit {
  slide_number: number;
  /** Surrounding excerpt with the match underlined by `[match]…[/match]`. */
  excerpt: string;
}

const MATCH_OPEN = "[match]";
const MATCH_CLOSE = "[/match]";

/**
 * Plain-text search across text-mode SVG output. Each slide's `<text>`
 * descendant text is concatenated and searched case-insensitively.
 *
 * Path-mode SVG has no searchable text (glyphs are paths) and
 * therefore yields no hits. The function is pure so consumers can run
 * it inside Web Workers if needed.
 */
export function searchSlides(slides: SlideSvg[], query: string): SearchHit[] {
  const trimmed = query.trim();
  if (!trimmed) return [];
  const needle = trimmed.toLowerCase();
  // Lightweight extractor: pull every `<text…>…</text>` and `<tspan…>…</tspan>`
  // body via regex. This sidesteps DOM-implementation differences for
  // namespaced SVG documents — the renderer is the sole producer of
  // these strings, so we know they're well-formed and balanced.
  const TEXT_RE = /<(?:text|tspan)\b[^>]*>([\s\S]*?)<\/(?:text|tspan)>/g;
  const decodeEntities = (s: string): string =>
    s
      .replaceAll("&amp;", "&")
      .replaceAll("&lt;", "<")
      .replaceAll("&gt;", ">")
      .replaceAll("&quot;", '"')
      .replaceAll("&apos;", "'");
  const hits: SearchHit[] = [];
  for (const slide of slides) {
    const fragments: string[] = [];
    for (const match of slide.svg.matchAll(TEXT_RE)) {
      const inner = match[1].replace(/<[^>]*>/g, " ");
      const text = decodeEntities(inner).trim();
      if (text) fragments.push(text);
    }
    const haystack = fragments.join(" ");
    const idx = haystack.toLowerCase().indexOf(needle);
    if (idx === -1) continue;
    const start = Math.max(0, idx - 24);
    const end = Math.min(haystack.length, idx + trimmed.length + 24);
    const before = haystack.slice(start, idx);
    const match = haystack.slice(idx, idx + trimmed.length);
    const after = haystack.slice(idx + trimmed.length, end);
    hits.push({
      slide_number: slide.slide_number,
      excerpt: `${start > 0 ? "…" : ""}${before}${MATCH_OPEN}${match}${MATCH_CLOSE}${after}${end < haystack.length ? "…" : ""}`,
    });
  }
  return hits;
}

/** Strip the `[match]` markers — useful for plain-text rendering. */
export function stripMatchMarkers(excerpt: string): string {
  return excerpt.split(MATCH_OPEN).join("").split(MATCH_CLOSE).join("");
}

/**
 * Split an excerpt into ordered `(text, isMatch)` runs so renderers
 * can wrap the highlighted span without parsing markers themselves.
 */
export function splitMatches(
  excerpt: string,
): Array<{ text: string; isMatch: boolean }> {
  const out: Array<{ text: string; isMatch: boolean }> = [];
  let cursor = 0;
  while (cursor < excerpt.length) {
    const open = excerpt.indexOf(MATCH_OPEN, cursor);
    if (open === -1) {
      out.push({ text: excerpt.slice(cursor), isMatch: false });
      break;
    }
    if (open > cursor)
      out.push({ text: excerpt.slice(cursor, open), isMatch: false });
    const close = excerpt.indexOf(MATCH_CLOSE, open + MATCH_OPEN.length);
    if (close === -1) {
      out.push({
        text: excerpt.slice(open + MATCH_OPEN.length),
        isMatch: true,
      });
      break;
    }
    out.push({
      text: excerpt.slice(open + MATCH_OPEN.length, close),
      isMatch: true,
    });
    cursor = close + MATCH_CLOSE.length;
  }
  return out;
}
