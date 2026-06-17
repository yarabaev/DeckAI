import { describe, expect, it } from "vitest";
import JSZip from "jszip";
import { buildPptx } from "./buildPptx.ts";

const slideSize = { w: 1280, h: 720 };

async function readSlideXml(
  bytes: Uint8Array,
  slideNumber = 1,
): Promise<string> {
  const zip = await JSZip.loadAsync(bytes);
  const path = `ppt/slides/slide${slideNumber}.xml`;
  const file = zip.file(path);
  if (!file) throw new Error(`missing ${path}`);
  return file.async("string");
}

describe("buildPptx — groups", () => {
  it("wraps a grouped container's descendants in a single <p:grpSp>", async () => {
    const xml = `
      <HStack group="diagram" gap="40">
        <Shape id="A" shapeType="rect" w="100" h="60" fill.color="#dddddd"/>
        <Shape id="B" shapeType="rect" w="100" h="60" fill.color="#dddddd"/>
      </HStack>
    `;
    const { pptx, diagnostics } = await buildPptx(xml, slideSize, {
      autoFit: false,
    });
    expect(diagnostics).toEqual([]);
    const bytes = (await pptx.write({
      outputType: "uint8array",
    })) as Uint8Array;
    const slide = await readSlideXml(bytes);

    // Exactly one grpSp, named after the author-supplied group id.
    const grpSpMatches = slide.match(/<p:grpSp>/g) ?? [];
    expect(grpSpMatches.length).toBe(1);
    expect(slide).toContain('name="diagram"');
    // Both shapes still emit (sg-id markers are stripped after the
    // connector pass; the group pass strips remaining sg-* tokens
    // including sg-grp).
    expect(slide).not.toContain("sg-id:");
    expect(slide).not.toContain("sg-grp:");
  });

  it('emits independent groups for two siblings both writing group="true"', async () => {
    const xml = `
      <HStack gap="40">
        <HStack group="true" gap="8">
          <Shape id="A1" shapeType="rect" w="60" h="40" fill.color="#dddddd"/>
          <Shape id="A2" shapeType="rect" w="60" h="40" fill.color="#dddddd"/>
        </HStack>
        <HStack group="true" gap="8">
          <Shape id="B1" shapeType="rect" w="60" h="40" fill.color="#dddddd"/>
          <Shape id="B2" shapeType="rect" w="60" h="40" fill.color="#dddddd"/>
        </HStack>
      </HStack>
    `;
    const { pptx } = await buildPptx(xml, slideSize, { autoFit: false });
    const bytes = (await pptx.write({
      outputType: "uint8array",
    })) as Uint8Array;
    const slide = await readSlideXml(bytes);

    const grpSpMatches = slide.match(/<p:grpSp>/g) ?? [];
    expect(grpSpMatches.length).toBe(2);
    // Auto ids are unique per occurrence.
    expect(slide).toContain('name="auto-grp-1"');
    expect(slide).toContain('name="auto-grp-2"');
  });

  it("nests <p:grpSp> when a grouped container sits inside another grouped container", async () => {
    const xml = `
      <HStack group="outer" gap="40">
        <HStack group="inner" gap="8">
          <Shape id="A" shapeType="rect" w="60" h="40" fill.color="#dddddd"/>
          <Shape id="B" shapeType="rect" w="60" h="40" fill.color="#dddddd"/>
        </HStack>
      </HStack>
    `;
    const { pptx } = await buildPptx(xml, slideSize, { autoFit: false });
    const bytes = (await pptx.write({
      outputType: "uint8array",
    })) as Uint8Array;
    const slide = await readSlideXml(bytes);

    // The outer grpSp must enclose the inner grpSp (string order =
    // document order in serialised XML).
    const outerStart = slide.indexOf('name="outer"');
    const innerStart = slide.indexOf('name="inner"');
    expect(outerStart).toBeGreaterThanOrEqual(0);
    expect(innerStart).toBeGreaterThan(outerStart);
    // The inner grpSp closes before the outer.
    const allGrpSpClose = [...slide.matchAll(/<\/p:grpSp>/g)].map(
      (m) => m.index ?? -1,
    );
    expect(allGrpSpClose.length).toBe(2);
    // Both closes appear after outerStart; the first close belongs to inner.
    expect(allGrpSpClose[0]).toBeLessThan(allGrpSpClose[1]!);
  });

  it("pulls Connector lines into the same <p:grpSp> as their endpoints", async () => {
    const xml = `
      <HStack group="diagram" gap="40">
        <Shape id="A" shapeType="rect" w="100" h="60" fill.color="#dddddd"/>
        <Shape id="B" shapeType="rect" w="100" h="60" fill.color="#dddddd"/>
        <Connector from="A" to="B" kind="straight" fromSide="right" toSide="left"/>
      </HStack>
    `;
    const { pptx } = await buildPptx(xml, slideSize, { autoFit: false });
    const bytes = (await pptx.write({
      outputType: "uint8array",
    })) as Uint8Array;
    const slide = await readSlideXml(bytes);

    // Single grpSp containing one cxnSp and two sp elements (besides
    // the outer spTree-level nvGrpSpPr / grpSpPr).
    expect((slide.match(/<p:grpSp>/g) ?? []).length).toBe(1);
    expect(slide).toContain("<p:cxnSp>");
    // The cxnSp sits between the grpSp open / close (string-order
    // check: cxnSp index is after grpSp open and before its close).
    const grpOpen = slide.indexOf("<p:grpSp>");
    const grpClose = slide.indexOf("</p:grpSp>");
    const cxnIdx = slide.indexOf("<p:cxnSp>");
    expect(cxnIdx).toBeGreaterThan(grpOpen);
    expect(cxnIdx).toBeLessThan(grpClose);
  });

  it("emits no <p:grpSp> when no node carries group=", async () => {
    const xml = `
      <HStack gap="40">
        <Shape id="A" shapeType="rect" w="100" h="60" fill.color="#dddddd"/>
        <Shape id="B" shapeType="rect" w="100" h="60" fill.color="#dddddd"/>
      </HStack>
    `;
    const { pptx } = await buildPptx(xml, slideSize, { autoFit: false });
    const bytes = (await pptx.write({
      outputType: "uint8array",
    })) as Uint8Array;
    const slide = await readSlideXml(bytes);
    expect(slide).not.toContain("<p:grpSp>");
  });
});
