import { describe, expect, it } from "vitest";
import { extractFontStyleCss, parseFirstFontFamily } from "../src/svg-utils.js";

describe("extractFontStyleCss", () => {
  it("returns empty string for empty input", () => {
    expect(extractFontStyleCss("")).toBe("");
  });

  it("returns empty string when no <style> block exists", () => {
    expect(extractFontStyleCss("<defs></defs>")).toBe("");
  });

  it("strips the <defs><style>…</style></defs> wrapper", () => {
    const input =
      "<defs><style type=\"text/css\">@font-face{font-family:'Foo';src:url(data:font/ttf;base64,AAA);}</style></defs>";
    const css = extractFontStyleCss(input);
    expect(css).toBe(
      "@font-face{font-family:'Foo';src:url(data:font/ttf;base64,AAA);}",
    );
  });

  it("preserves multiple @font-face rules in order", () => {
    const input =
      "<defs><style type=\"text/css\">@font-face{font-family:'A';src:url(data:font/ttf;base64,A);}@font-face{font-family:'B';src:url(data:font/ttf;base64,B);}</style></defs>";
    const css = extractFontStyleCss(input);
    expect(css.indexOf("'A'")).toBeLessThan(css.indexOf("'B'"));
    expect((css.match(/@font-face/g) ?? []).length).toBe(2);
  });

  it("trims surrounding whitespace inside the <style> block", () => {
    const input =
      "<defs><style>\n  @font-face{font-family:'Foo';src:url(x);}\n</style></defs>";
    expect(extractFontStyleCss(input)).toBe(
      "@font-face{font-family:'Foo';src:url(x);}",
    );
  });
});

describe("parseFirstFontFamily", () => {
  it("returns the first family from a comma-separated chain", () => {
    expect(parseFirstFontFamily("'Source Sans 3', Arial, sans-serif")).toBe(
      "Source Sans 3",
    );
  });

  it("strips matching surrounding quotes", () => {
    expect(parseFirstFontFamily('"Open Sans", sans-serif')).toBe("Open Sans");
    expect(parseFirstFontFamily("'Noto Sans KR'")).toBe("Noto Sans KR");
  });

  it("returns the unquoted name for a single bareword", () => {
    expect(parseFirstFontFamily("Arial")).toBe("Arial");
  });

  it("returns null for empty input", () => {
    expect(parseFirstFontFamily("")).toBeNull();
    expect(parseFirstFontFamily("   ")).toBeNull();
  });

  it("rejects generic CSS fallback aliases as authored fonts", () => {
    // These would mislead a user who's trying to identify the authored
    // typeface — the renderer adds them automatically as a last-resort
    // fallback even when the source PPTX did not name them.
    expect(parseFirstFontFamily("sans-serif")).toBeNull();
    expect(parseFirstFontFamily("serif")).toBeNull();
    expect(parseFirstFontFamily("monospace")).toBeNull();
    expect(parseFirstFontFamily("system-ui")).toBeNull();
  });

  it("preserves family names that contain spaces and case", () => {
    expect(parseFirstFontFamily('"맑은 고딕", sans-serif')).toBe("맑은 고딕");
    expect(parseFirstFontFamily("'NoTo SaNs', serif")).toBe("NoTo SaNs");
  });
});
