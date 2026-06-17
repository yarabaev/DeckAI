import { describe, expect, it } from "vitest";
import { buildReferenceModel } from "./walkRegistry.ts";

describe("buildReferenceModel", () => {
  const model = buildReferenceModel();

  it("exposes namespace + package version", () => {
    expect(model.namespace).toBe("urn:slideglance:builder:v1");
    expect(model.packageVersion).toMatch(/^\d+\.\d+\.\d+/);
    expect(model.generatedAt).toBe("");
  });

  it("preserves registry counts (17 nodes + 13 meta)", () => {
    expect(model.nodes).toHaveLength(17);
    expect(model.meta).toHaveLength(13);
  });
});

describe("ReferenceModel.usedBy", () => {
  const model = buildReferenceModel();

  it("Text is used by VStack, HStack, Layer", () => {
    const entry = model.usedBy.get("Text") ?? [];
    const parents = entry.map((e) => e.parent);
    expect(parents).toContain("VStack");
    expect(parents).toContain("HStack");
    expect(parents).toContain("Layer");
  });

  it("SlideGlance is a root — has no parents", () => {
    expect(model.usedBy.get("SlideGlance") ?? []).toEqual([]);
  });

  it("Li is used by Ul and Ol", () => {
    const parents = (model.usedBy.get("Li") ?? []).map((e) => e.parent);
    expect(parents).toContain("Ul");
    expect(parents).toContain("Ol");
  });

  it("Template is used by Templates (meta children via allowedParents)", () => {
    const parents = (model.usedBy.get("Template") ?? []).map((e) => e.parent);
    expect(parents).toContain("Templates");
  });

  it("When and Otherwise are used by Choose", () => {
    expect((model.usedBy.get("When") ?? []).map((e) => e.parent)).toContain(
      "Choose",
    );
    expect(
      (model.usedBy.get("Otherwise") ?? []).map((e) => e.parent),
    ).toContain("Choose");
  });
});

describe("ReferenceModel.sourceLocations", () => {
  const model = buildReferenceModel();

  it("maps every node + meta tag to a source line", () => {
    for (const n of [...model.nodes, ...model.meta]) {
      const loc = model.sourceLocations.get(n.tag);
      expect(loc, `missing location for ${n.tag}`).toBeDefined();
      expect(loc!.file).toBe("src/registry/compiled/index.ts");
      expect(loc!.line).toBeGreaterThan(0);
    }
  });

  it("line numbers are monotonically increasing within the registry file", () => {
    const tags = [...model.nodes, ...model.meta].map((n) => n.tag);
    const lines = tags.map((t) => model.sourceLocations.get(t)!.line);
    expect(lines.every((l, i, a) => i === 0 || l > a[i - 1])).toBe(true);
  });
});
