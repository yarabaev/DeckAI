import { describe, expect, it } from "vitest";
import { isBundledFont } from "./fontLoader.ts";

describe("isBundledFont", () => {
  it("Noto Sans JP はバンドル済みフォントと判定される", () => {
    expect(isBundledFont("Noto Sans JP")).toBe(true);
  });

  it("Pretendard はバンドル済みフォントと判定される", () => {
    expect(isBundledFont("Pretendard")).toBe(true);
  });

  it("Arial はバンドル外フォントと判定される", () => {
    expect(isBundledFont("Arial")).toBe(false);
  });

  it("空文字はバンドル外フォントと判定される", () => {
    expect(isBundledFont("")).toBe(false);
  });
});
