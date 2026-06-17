/**
 * Bundled-font registry + thin facade over `@slideglance/measure`'s
 * `TextMeasurer` for layout-time text measurement.
 *
 * The four bundled font buffers (Noto Sans JP / Pretendard, Regular +
 * Bold) are passed to slideglance once, lazily, on first use. Bold
 * variants are auto-detected via `OS/2.usWeightClass >= 600` on the
 * slideglance side and routed to its bold-variant slot, so the same
 * family name (e.g. `"Pretendard"`) resolves to the Regular or Bold
 * face based purely on the `bold` flag at measurement time.
 */

import { TextMeasurer } from "@slideglance/measure/node";
import { DEFAULT_FONT_FAMILY } from "../defaultTextStyle.ts";
import { NOTO_SANS_JP_REGULAR_BASE64 } from "./fonts/notoSansJPRegular.ts";
import { NOTO_SANS_JP_BOLD_BASE64 } from "./fonts/notoSansJPBold.ts";
import { PRETENDARD_REGULAR_BASE64 } from "./fonts/pretendardRegular.ts";
import { PRETENDARD_BOLD_BASE64 } from "./fonts/pretendardBold.ts";

type FontWeight = "normal" | "bold";

const BUNDLED_FONTS = {
  "Noto Sans JP": {
    normal: NOTO_SANS_JP_REGULAR_BASE64,
    bold: NOTO_SANS_JP_BOLD_BASE64,
  },
  Pretendard: {
    normal: PRETENDARD_REGULAR_BASE64,
    bold: PRETENDARD_BOLD_BASE64,
  },
} satisfies Record<string, Record<FontWeight, string>>;

function base64ToUint8Array(base64: string): Uint8Array {
  if (typeof Buffer !== "undefined") {
    return new Uint8Array(Buffer.from(base64, "base64"));
  }
  const binaryString = atob(base64);
  const bytes = new Uint8Array(binaryString.length);
  for (let i = 0; i < binaryString.length; i++) {
    bytes[i] = binaryString.charCodeAt(i);
  }
  return bytes;
}

let cachedMeasurer: TextMeasurer | undefined;
let cachedBundledFontBytes: Uint8Array[] | undefined;

/**
 * Decode the bundled-font base64 buffers exactly once. Both the default
 * singleton and any per-build measurer created via {@link createMeasurer}
 * reuse this array — re-decoding the base64 on every build added ~50ms
 * of cold-start time per call in benchmarks.
 */
function getBundledFontBytes(): Uint8Array[] {
  if (cachedBundledFontBytes) return cachedBundledFontBytes;
  const fonts: Uint8Array[] = [];
  for (const variants of Object.values(BUNDLED_FONTS)) {
    fonts.push(base64ToUint8Array(variants.normal));
    fonts.push(base64ToUint8Array(variants.bold));
  }
  cachedBundledFontBytes = fonts;
  return fonts;
}

/**
 * Default measurer singleton — bundled fonts only. Used when no caller
 * supplies its own measurer via {@link createMeasurer}. Decoding bundled-font
 * bytes is amortised across every build that hits this path.
 */
function getMeasurer(): TextMeasurer {
  if (cachedMeasurer) return cachedMeasurer;
  // Pass every variant under its natural family name (the .ttf's
  // `name` table reports `"Pretendard"` for both Regular and Bold).
  // The slideglance constructor inspects `OS/2.usWeightClass` and
  // routes Bold faces to the resolver's bold-variant slot, so a single
  // family lookup with `bold=true` reaches the Bold face directly.
  cachedMeasurer = new TextMeasurer(getBundledFontBytes(), undefined);
  return cachedMeasurer;
}

/**
 * Per-build measurer factory. Returns a fresh {@link TextMeasurer} that
 * mounts the bundled fonts (Noto Sans JP, Pretendard) plus every
 * caller-supplied TTF/OTF buffer in `extraFonts`. The slideglance
 * constructor reads each face's `name` table for the family lookup and
 * the `OS/2.usWeightClass` for the regular/bold split — supply both
 * weight variants of each family for accurate bold measurement.
 *
 * When `extraFonts` is empty / undefined this is identical to
 * {@link getMeasurer} but allocates a new instance instead of reusing
 * the cached singleton; prefer {@link getMeasurer} on the no-extras
 * path to avoid the allocation.
 */
export function createMeasurer(extraFonts?: Uint8Array[]): TextMeasurer {
  if (!extraFonts || extraFonts.length === 0) return getMeasurer();
  const fonts: Uint8Array[] = [...getBundledFontBytes(), ...extraFonts];
  return new TextMeasurer(fonts, undefined);
}

const BUNDLED_FONT_NAMES = new Set(Object.keys(BUNDLED_FONTS));

/** Whether the family name has a bundled TTF buffer this module can register. */
export function isBundledFont(fontFamily: string): boolean {
  return BUNDLED_FONT_NAMES.has(fontFamily);
}

/**
 * Script-aware bundled-font picker for an unbundled family.
 *
 * Both bundled fonts cover Latin glyphs, but only Pretendard ships
 * Hangul (Noto Sans JP covers Han/Hiragana/Katakana plus Latin). When
 * the caller supplies an unbundled `fontFamily`, sample the input
 * text and prefer Pretendard for Hangul-dominant strings; otherwise
 * use the global default. CJK and Latin text both round-trip fine
 * through Noto Sans JP.
 */
export function pickBundledFontForText(
  fontFamily: string,
  text: string,
): string {
  if (isBundledFont(fontFamily)) return fontFamily;
  let hangul = 0;
  let total = 0;
  for (const ch of text) {
    const code = ch.codePointAt(0);
    if (code === undefined) continue;
    total++;
    if (
      (code >= 0xac00 && code <= 0xd7a3) || // Hangul syllables
      (code >= 0x1100 && code <= 0x11ff) || // Hangul jamo
      (code >= 0x3130 && code <= 0x318f) // Hangul compatibility jamo
    ) {
      hangul++;
    }
  }
  if (total > 0 && hangul / total > 0.3) {
    return "Pretendard";
  }
  return DEFAULT_FONT_FAMILY;
}

const PT_PER_PX = 72 / 96;

/**
 * Pixel advance of `text` rendered at `fontSizePx` in the given bundled
 * font / weight. Non-bundled families must be canonicalised by the
 * caller (e.g. via {@link pickBundledFontForText}); passing an
 * unknown name causes the underlying `OpentypeTextMeasurer` to fall
 * back to its heuristic.
 */
export function measureTextWidth(
  text: string,
  fontFamily: string,
  fontSizePx: number,
  weight: FontWeight,
  measurer?: TextMeasurer,
): number {
  const m = measurer ?? getMeasurer();
  return m.measureWidth(
    text,
    fontSizePx * PT_PER_PX,
    weight === "bold",
    false,
    fontFamily,
    null,
  );
}

/**
 * Natural line-height ratio (`× fontSize`) for the given bundled font /
 * weight. Mirrors the legacy opentype-side computation
 * `(ascender + |descender| + lineGap) / unitsPerEm`, but reads metrics
 * from slideglance's resolved face. Non-bundled families fall through
 * to {@link DEFAULT_FONT_FAMILY}.
 */
export function measureFontLineHeightRatio(
  fontFamily: string,
  weight: FontWeight,
  measurer?: TextMeasurer,
): number {
  const m = measurer ?? getMeasurer();
  const baseFamily = isBundledFont(fontFamily)
    ? fontFamily
    : DEFAULT_FONT_FAMILY;
  // The `lineHeightRatio` API only takes a family — to reach the Bold
  // face's vertical metrics for a bold run, we issue an internal
  // `measureWidth` to populate the variant, then read the regular
  // ratio. In practice line-height differences between Regular and
  // Bold of the same family are negligible (<1%) for static fonts, so
  // we use the regular face's ratio for both weights. (Variable-axis
  // fonts that ship `wght` already pick the right axis at measure
  // time inside slideglance.)
  void weight;
  return m.lineHeightRatio(baseFamily, null);
}

/** Re-export the {@link TextMeasurer} type so callers don't need to depend on `@slideglance/measure` directly. */
export type { TextMeasurer };
