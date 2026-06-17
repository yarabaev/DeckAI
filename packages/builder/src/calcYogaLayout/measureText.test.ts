import { describe, expect, it } from "vitest";
import { measureText } from "./measureText";

describe("measureText", () => {
  describe("フォントファミリーによる計測方法の切り替え", () => {
    const text = "Hello World";
    const maxWidth = 500;

    it("バンドル済みフォント（Noto Sans JP）では opentype 計測を使用する", () => {
      const opentypeResult = measureText(text, maxWidth, {
        fontFamily: "Noto Sans JP",
        fontSizePx: 24,
      });

      const fallbackResult = measureText(
        text,
        maxWidth,
        {
          fontFamily: "Noto Sans JP",
          fontSizePx: 24,
        },
        "fallback",
      );

      // Confirm opentype and fallback produce different results (=opentype is in effect).
      expect(opentypeResult.widthPx).not.toBe(fallbackResult.widthPx);
    });

    it("Pretendard でも opentype 計測を使用する", () => {
      const opentypeResult = measureText(text, maxWidth, {
        fontFamily: "Pretendard",
        fontSizePx: 24,
      });

      const fallbackResult = measureText(
        text,
        maxWidth,
        {
          fontFamily: "Pretendard",
          fontSizePx: 24,
        },
        "fallback",
      );

      expect(opentypeResult.widthPx).not.toBe(fallbackResult.widthPx);
    });

    it("auto mode routes non-bundled fonts through opentype substitution", () => {
      // Updated policy: `auto` no longer falls back to the per-char
      // heuristic for unknown families. It runs the opentype measurer
      // with the script-aware bundled substitute (`pickBundledFontForText`),
      // tracking the renderer's font-fallback chain. The heuristic-equal
      // assertion the legacy test made would now mask the very mismatch
      // this measurement-policy change exists to close.
      const arialAuto = measureText(text, maxWidth, {
        fontFamily: "Arial",
        fontSizePx: 24,
      });
      const arialFallback = measureText(
        text,
        maxWidth,
        { fontFamily: "Arial", fontSizePx: 24 },
        "fallback",
      );
      const arialOpentype = measureText(
        text,
        maxWidth,
        { fontFamily: "Arial", fontSizePx: 24 },
        "opentype",
      );

      // auto now matches explicit opentype, not the heuristic fallback.
      expect(arialAuto.widthPx).toBe(arialOpentype.widthPx);
      expect(arialAuto.heightPx).toBe(arialOpentype.heightPx);
      expect(arialAuto.widthPx).not.toBe(arialFallback.widthPx);
    });

    it("mode=fallback が明示された場合はフォントに関係なくフォールバックを使用する", () => {
      const result = measureText(
        text,
        maxWidth,
        {
          fontFamily: "Noto Sans JP",
          fontSizePx: 24,
        },
        "fallback",
      );

      const fallbackDirect = measureText(
        text,
        maxWidth,
        {
          fontFamily: "Arial",
          fontSizePx: 24,
        },
        "fallback",
      );

      // Same fallback computation, so the result is identical.
      expect(result.widthPx).toBe(fallbackDirect.widthPx);
    });

    it("mode=opentype が明示された場合はバンドル外フォントでも opentype 計測を使用する", () => {
      const opentypeForced = measureText(
        text,
        maxWidth,
        {
          fontFamily: "Arial",
          fontSizePx: 24,
        },
        "opentype",
      );

      const fallbackResult = measureText(
        text,
        maxWidth,
        {
          fontFamily: "Arial",
          fontSizePx: 24,
        },
        "fallback",
      );

      // Forcing opentype yields a result distinct from fallback.
      expect(opentypeForced.widthPx).not.toBe(fallbackResult.widthPx);
    });
  });

  describe("fallback per-character glyph width", () => {
    it("estimates narrow chars (i,l,.) at less than wide chars (m,M,W)", () => {
      const narrow = measureText(
        "iiiiiiiiii",
        Number.POSITIVE_INFINITY,
        { fontFamily: "Arial", fontSizePx: 20 },
        "fallback",
      );
      const wide = measureText(
        "mmmmmmmmmm",
        Number.POSITIVE_INFINITY,
        { fontFamily: "Arial", fontSizePx: 20 },
        "fallback",
      );
      expect(narrow.widthPx).toBeLessThan(wide.widthPx);
      // The gap is large — "i" is roughly a third of "m".
      expect(narrow.widthPx).toBeLessThan(wide.widthPx * 0.5);
    });

    it("estimates a long Latin string at less than the legacy 0.5em flat rate", () => {
      // The previous implementation returned fontSizePx * 0.5 for every
      // non-CJK character. The per-character table averages closer to
      // ~0.45em on real text — confirm a representative sentence now
      // measures lower than the old 0.5em ceiling. This is what lets
      // toc-style row layouts fit on one line in 50%-wide columns when
      // they used to over-wrap to two.
      const text = "Documentation, archaeology, and forgetting";
      const fontSizePx = 13;
      const result = measureText(
        text,
        Number.POSITIVE_INFINITY,
        { fontFamily: "Georgia", fontSizePx },
        "fallback",
      );
      const oldFlatWidth = text.length * fontSizePx * 0.5;
      expect(result.widthPx).toBeLessThan(oldFlatWidth);
    });

    it("CJK chars still measure ~1em each", () => {
      const result = measureText(
        "本日は晴天",
        Number.POSITIVE_INFINITY,
        { fontFamily: "Inter", fontSizePx: 20 },
        "fallback",
      );
      // 5 CJK chars × 1em × 20px = ~100. The wrap/split path adds a
      // small amount of inter-character padding so the value lands a
      // little above; verify it stays in the CJK-width ballpark and
      // is at least 5× the Latin per-char width (which is < 0.5em).
      expect(result.widthPx).toBeGreaterThanOrEqual(5 * 20);
      expect(result.widthPx).toBeLessThan(5 * 20 * 1.5);
    });
  });
});
