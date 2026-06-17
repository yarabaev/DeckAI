import { describe, expect, it } from "vitest";
import JSZip from "jszip";
import { buildPptx } from "./buildPptx.ts";

const slideSize = { w: 1280, h: 720 };

/**
 * Pull a slide{N}.xml out of a built PPTX so individual tests can grep
 * for the connector markup without re-implementing the unzip dance.
 */
async function readSlideXml(
  bytes: Uint8Array,
  slideNumber = 1,
): Promise<string> {
  const zip = await JSZip.loadAsync(bytes);
  const path = `ppt/slides/slide${slideNumber}.xml`;
  const file = zip.file(path);
  if (!file) throw new Error(`missing ${path} in built PPTX`);
  return file.async("string");
}

describe("buildPptx — connectors", () => {
  it("emits <p:cxnSp> with stCxn / endCxn when a Connector binds two shapes", async () => {
    // Tests use HStack (not Layer) so Shape children don't need x/y, and
    // Connector — which doesn't carry x/y — can sit alongside them.
    const xml = `
      <HStack gap="40">
        <Shape id="A" shapeType="rect" w="200" h="100" fill.color="#dddddd"/>
        <Shape id="B" shapeType="rect" w="200" h="100" fill.color="#dddddd"/>
        <Connector from="A" to="B" kind="elbow" fromSide="right" toSide="left" endArrow="true"/>
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

    expect(slide).toContain("<p:cxnSp>");
    expect(slide).toContain('prst="bentConnector3"');
    expect(slide).toMatch(/<a:stCxn id="\d+" idx="1"\/>/);
    expect(slide).toMatch(/<a:endCxn id="\d+" idx="3"\/>/);
    // Sigils are stripped from author-visible cNvPr@name.
    expect(slide).not.toContain("sg-id:");
    expect(slide).not.toContain("sg-cxn:");
  });

  it("auto-picks fromSide / toSide from the bbox geometry when omitted", async () => {
    const xml = `
      <VStack gap="200">
        <Shape id="A" shapeType="rect" w="120" h="80" fill.color="#dddddd"/>
        <Shape id="B" shapeType="rect" w="120" h="80" fill.color="#dddddd"/>
        <Connector from="A" to="B" kind="straight"/>
      </VStack>
    `;
    const { pptx } = await buildPptx(xml, slideSize, { autoFit: false });
    const bytes = (await pptx.write({
      outputType: "uint8array",
    })) as Uint8Array;
    const slide = await readSlideXml(bytes);

    expect(slide).toContain('prst="straightConnector1"');
    // bottom side -> idx 2, top side -> idx 0 for rect.
    expect(slide).toMatch(/<a:stCxn id="\d+" idx="2"\/>/);
    expect(slide).toMatch(/<a:endCxn id="\d+" idx="0"\/>/);
  });

  it("supports the three kinds with their preset families", async () => {
    const cases: Array<[string, string]> = [
      ["straight", "straightConnector1"],
      ["elbow", "bentConnector3"],
      ["curved", "curvedConnector3"],
    ];
    for (const [kind, preset] of cases) {
      const xml = `
        <HStack gap="40">
          <Shape id="A" shapeType="rect" w="100" h="60" fill.color="#dddddd"/>
          <Shape id="B" shapeType="rect" w="100" h="60" fill.color="#dddddd"/>
          <Connector from="A" to="B" kind="${kind}" fromSide="right" toSide="left"/>
        </HStack>
      `;
      const { pptx } = await buildPptx(xml, slideSize, { autoFit: false });
      const bytes = (await pptx.write({
        outputType: "uint8array",
      })) as Uint8Array;
      const slide = await readSlideXml(bytes);
      expect(slide).toContain(`prst="${preset}"`);
    }
  });

  it("drops <Connector> with a missing endpoint and emits UNKNOWN_CONNECTOR_ENDPOINT", async () => {
    const xml = `
      <HStack>
        <Shape id="A" shapeType="rect" w="100" h="60" fill.color="#dddddd"/>
        <Connector from="A" to="GHOST"/>
      </HStack>
    `;
    const { pptx, diagnostics } = await buildPptx(xml, slideSize, {
      autoFit: false,
    });
    expect(diagnostics.map((d) => d.code)).toContain(
      "UNKNOWN_CONNECTOR_ENDPOINT",
    );
    const bytes = (await pptx.write({
      outputType: "uint8array",
    })) as Uint8Array;
    const slide = await readSlideXml(bytes);
    expect(slide).not.toContain("<p:cxnSp>");
  });

  it("drops self-referencing <Connector> and emits INVALID_CONNECTOR_SELF_REF", async () => {
    const xml = `
      <HStack>
        <Shape id="A" shapeType="rect" w="100" h="60" fill.color="#dddddd"/>
        <Connector from="A" to="A"/>
      </HStack>
    `;
    const { pptx, diagnostics } = await buildPptx(xml, slideSize, {
      autoFit: false,
    });
    expect(diagnostics.map((d) => d.code)).toContain(
      "INVALID_CONNECTOR_SELF_REF",
    );
    const bytes = (await pptx.write({
      outputType: "uint8array",
    })) as Uint8Array;
    const slide = await readSlideXml(bytes);
    expect(slide).not.toContain("<p:cxnSp>");
  });

  it("emits DUPLICATE_NODE_ID when the same id appears twice on a slide", async () => {
    const xml = `
      <HStack gap="40">
        <Shape id="A" shapeType="rect" w="100" h="60" fill.color="#dddddd"/>
        <Shape id="A" shapeType="rect" w="100" h="60" fill.color="#dddddd"/>
      </HStack>
    `;
    const { diagnostics } = await buildPptx(xml, slideSize, { autoFit: false });
    expect(diagnostics.map((d) => d.code)).toContain("DUPLICATE_NODE_ID");
  });
});
