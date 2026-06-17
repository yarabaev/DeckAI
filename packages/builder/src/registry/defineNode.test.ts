import { describe, expect, it } from "vitest";
import { z } from "zod";
import { defineNode } from "./defineNode.ts";
import { defineMeta } from "./defineMeta.ts";

describe("defineNode", () => {
  it("returns a compiled definition with kind=node", () => {
    const def = defineNode({
      tag: "Text",
      type: "text",
      description: "A piece of styled text.",
      category: "leaf",
      schema: z.object({ type: z.literal("text"), text: z.string() }),
      attributes: {
        text: { coerce: "string", doc: "Text content", bodyAlias: true },
        fontSize: { coerce: "number" },
      },
    });

    expect(def.kind).toBe("node");
    expect(def.tag).toBe("Text");
    expect(def.type).toBe("text");
    expect(def.attributes.text!.bodyAlias).toBe(true);
    expect(def.attributes.fontSize!.coerce).toBe("number");
    expect(def.root).toBe(false);
    expect(def.children).toEqual({});
  });

  it("throws when bodyAlias is set on a non-string attribute", () => {
    expect(() =>
      defineNode({
        tag: "Bad",
        description: "x",
        attributes: { val: { coerce: "number", bodyAlias: true } },
      }),
    ).toThrow(/bodyAlias/);
  });

  it("requires a description", () => {
    expect(() =>
      // @ts-expect-error description omitted on purpose
      defineNode({ tag: "Bad" }),
    ).toThrow(/description/);
  });
});

describe("defineMeta", () => {
  it("returns a compiled meta definition", () => {
    const meta = defineMeta({
      tag: "Template",
      description: "Defines a reusable XML template.",
      contentModel: "any",
      allowedParents: ["Templates"],
      attributes: {
        name: { coerce: "string", required: true, doc: "Template name" },
      },
    });

    expect(meta.kind).toBe("meta");
    expect(meta.tag).toBe("Template");
    expect(meta.contentModel).toBe("any");
    expect(meta.allowedParents).toEqual(["Templates"]);
  });
});
