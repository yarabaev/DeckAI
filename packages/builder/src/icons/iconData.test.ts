import { describe, expect, it } from "vitest";
import { ICON_DATA } from "./iconData.ts";

describe("ICON_DATA", () => {
  it("1900 個以上のアイコンを含む", () => {
    expect(Object.keys(ICON_DATA).length).toBeGreaterThanOrEqual(1900);
  });

  it("代表的なアイコン名が存在する", () => {
    const expected = [
      "cpu",
      "database",
      "cloud",
      "server",
      "user",
      "heart",
      "star",
      "search",
      "settings",
      "arrow-right",
    ];
    for (const name of expected) {
      expect(ICON_DATA).toHaveProperty(name);
    }
  });

  it("全エントリが空でない SVG パスデータを持つ", () => {
    for (const [name, data] of Object.entries(ICON_DATA)) {
      expect(data, `${name} のパスデータが空`).toBeTruthy();
      expect(data, `${name} に <path> や <circle> 等の SVG 要素がない`).toMatch(
        /<(path|circle|rect|line|ellipse|polygon|polyline)\b/,
      );
    }
  });
});
