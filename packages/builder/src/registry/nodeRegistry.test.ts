import { describe, it, expect } from "vitest";
import type { BuilderNode } from "../types.ts";
import { getNodeDef } from "./index.ts";

/**
 * Full list of BuilderNode type literals.
 * When a new node is added to the BuilderNode union in types.ts, add it here too.
 */
const ALL_NODE_TYPES: BuilderNode["type"][] = [
  "text",
  "ul",
  "ol",
  "image",
  "table",
  "vstack",
  "hstack",
  "shape",
  "chart",
  "line",
  "layer",
  "icon",
  "svg",
];

describe("NodeRegistry", () => {
  it("全ノードタイプが登録されていること", () => {
    for (const type of ALL_NODE_TYPES) {
      expect(() => getNodeDef(type)).not.toThrow();
    }
  });

  it("leaf ノードは render 関数を持つこと", () => {
    for (const type of ALL_NODE_TYPES) {
      const def = getNodeDef(type);
      if (def.category === "leaf") {
        expect(def.render).toBeDefined();
      }
    }
  });

  it("absolute-child ノードは toPositioned 関数を持つこと", () => {
    for (const type of ALL_NODE_TYPES) {
      const def = getNodeDef(type);
      if (def.category === "absolute-child") {
        expect(def.toPositioned).toBeDefined();
      }
    }
  });

  it("カテゴリが正しく設定されていること", () => {
    expect(getNodeDef("vstack").category).toBe("multi-child");
    expect(getNodeDef("hstack").category).toBe("multi-child");
    expect(getNodeDef("layer").category).toBe("absolute-child");

    const leafTypes: BuilderNode["type"][] = [
      "text",
      "ul",
      "ol",
      "image",
      "table",
      "shape",
      "chart",
      "line",
      "icon",
      "svg",
    ];
    for (const type of leafTypes) {
      expect(getNodeDef(type).category).toBe("leaf");
    }
  });
});
