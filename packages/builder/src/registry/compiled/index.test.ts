import { describe, expect, it } from "vitest";
import {
  ALL_COMPILED_NODES,
  ALL_COMPILED_META,
  validateCompiledRegistry,
} from "./index.ts";

const textCompiled = ALL_COMPILED_NODES.find((n) => n.tag === "Text");

describe("compiled registry", () => {
  it("exposes every authored node + meta declaration", () => {
    // POM node count (14 incl. Connector) + 3 document containers
    // (SlideGlance/Slide/Fragment)
    expect(ALL_COMPILED_NODES.length).toBe(17);
    // 13 meta tags: Document + Templates/Template/Use/Slot/Import/Styles/Style
    // + If/Choose/When/Otherwise/Foreach
    expect(ALL_COMPILED_META.length).toBe(13);
  });

  it("has unique tags across nodes and meta", () => {
    expect(() => validateCompiledRegistry()).not.toThrow();
  });

  it("text node has expected attribute and schema wired up", () => {
    expect(textCompiled).toBeDefined();
    expect(textCompiled!.attributes.text!.bodyAlias).toBe(true);
    expect(textCompiled!.attributes.fontSize!.coerce).toBe("number");
    expect(textCompiled!.schema).toBeDefined();
  });

  it("every node carries a description and category (or root flag for containers)", () => {
    for (const def of ALL_COMPILED_NODES) {
      expect(def.description.length).toBeGreaterThan(0);
      // Document containers (SlideGlance/Slide/Fragment) have no category
      // but should have root or be a Slide.
      if (!def.category) {
        expect(["SlideGlance", "Slide", "Fragment"]).toContain(def.tag);
      }
    }
  });

  it("every meta has a contentModel and description", () => {
    for (const meta of ALL_COMPILED_META) {
      expect(meta.description.length).toBeGreaterThan(0);
      expect(meta.contentModel).toBeDefined();
    }
  });
});
