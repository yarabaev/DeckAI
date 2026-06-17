import { describe, expect, it } from "vitest";
import { XMLBuilder, XMLParser } from "fast-xml-parser";
import { _rewriteSlideForTest } from "./cxnSp.ts";
import type { Diagnostic } from "../../diagnostics.ts";

const parser = new XMLParser({
  preserveOrder: true,
  ignoreAttributes: false,
  attributeNamePrefix: "@_",
  parseAttributeValue: false,
  parseTagValue: false,
  trimValues: false,
});
// Match the production builder used by cxnSp.ts so the
// expectation strings line up. fast-xml-parser emits empty elements
// as `<tag></tag>` when suppressEmptyNode is false; we read that
// shape rather than the self-closing one.
const builder = new XMLBuilder({
  preserveOrder: true,
  ignoreAttributes: false,
  attributeNamePrefix: "@_",
  suppressEmptyNode: true,
});

/**
 * Build a minimal slide XML with two preset shapes (sg-id sigils on
 * cNvPr) and a placeholder line (sg-cxn sigil). We feed the parser
 * via this helper so each test starts from a clean DOM.
 */
function makeSlideXml(opts: {
  fromUserId: string;
  fromPrst: string;
  fromSpId: string;
  toUserId: string;
  toPrst: string;
  toSpId: string;
  cxnSigil: string;
  cxnSpId: string;
}): string {
  return `<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr><a:xfrm/></p:grpSpPr>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="${opts.fromSpId}" name="sg-id:${opts.fromUserId}"/>
          <p:cNvSpPr/>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm>
          <a:prstGeom prst="${opts.fromPrst}"><a:avLst/></a:prstGeom>
        </p:spPr>
      </p:sp>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="${opts.toSpId}" name="sg-id:${opts.toUserId}"/>
          <p:cNvSpPr/>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="1828800" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm>
          <a:prstGeom prst="${opts.toPrst}"><a:avLst/></a:prstGeom>
        </p:spPr>
      </p:sp>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="${opts.cxnSpId}" name="${opts.cxnSigil}"/>
          <p:cNvSpPr/>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="914400" y="457200"/><a:ext cx="914400" cy="0"/></a:xfrm>
          <a:prstGeom prst="line"><a:avLst/></a:prstGeom>
        </p:spPr>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>`;
}

describe("rewriteSlide (single slide round-trip)", () => {
  it("rewrites <p:sp prst=line> into <p:cxnSp> with the chosen preset and stCxn / endCxn", () => {
    const xml = makeSlideXml({
      fromUserId: "A",
      fromPrst: "rect",
      fromSpId: "10",
      toUserId: "B",
      toPrst: "rect",
      toSpId: "11",
      cxnSigil: "sg-cxn:A#right>B#left:elbow:bentConnector3",
      cxnSpId: "12",
    });
    const parsed = parser.parse(xml) as Parameters<
      typeof _rewriteSlideForTest
    >[0];
    const diagnostics: Diagnostic[] = [];
    const changed = _rewriteSlideForTest(parsed, diagnostics);
    expect(changed).toBe(true);
    const out = builder.build(parsed) as string;

    // The connector placeholder became cxnSp.
    expect(out).toContain("<p:cxnSp>");
    // ...with the picked preset.
    expect(out).toContain('prst="bentConnector3"');
    // ...and stCxn / endCxn bound to the sg-id spIds.
    expect(out).toContain('<a:stCxn id="10" idx="1"/>');
    expect(out).toContain('<a:endCxn id="11" idx="3"/>');
    // The two endpoint shapes lost their sg-id sigils.
    expect(out).not.toContain("sg-id:A");
    expect(out).not.toContain("sg-id:B");
    // The placeholder lost its sg-cxn sigil.
    expect(out).not.toContain("sg-cxn:");
    // The original line prstGeom is gone (the connector got the new prst).
    expect(out).not.toContain('prst="line"');
    // The nvSpPr group around the cxn became nvCxnSpPr / cNvCxnSpPr.
    expect(out).toContain("<p:nvCxnSpPr>");
    expect(out).toContain("<p:cNvCxnSpPr>");
    // No CONNECTOR_UNKNOWN_SHAPE_IDX for the well-known rect/rect pair.
    expect(diagnostics).toHaveLength(0);
  });

  it("emits CONNECTOR_UNKNOWN_SHAPE_IDX when an endpoint prst is not in the side-idx table", () => {
    const xml = makeSlideXml({
      fromUserId: "A",
      fromPrst: "rect",
      fromSpId: "10",
      toUserId: "B",
      // Unfamiliar preset name -> falls back to DEFAULT_SIDE_IDX but
      // surfaces a non-fatal diagnostic.
      toPrst: "mysteryShape42",
      toSpId: "11",
      cxnSigil: "sg-cxn:A#right>B#left:straight:straightConnector1",
      cxnSpId: "12",
    });
    const parsed = parser.parse(xml) as Parameters<
      typeof _rewriteSlideForTest
    >[0];
    const diagnostics: Diagnostic[] = [];
    _rewriteSlideForTest(parsed, diagnostics);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0]?.code).toBe("CONNECTOR_UNKNOWN_SHAPE_IDX");
  });

  it("does nothing when the slide has no sg-cxn placeholders", () => {
    const xml = `<?xml version="1.0"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld><p:spTree>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Picture 1"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr><a:xfrm/><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
</p:sld>`;
    const parsed = parser.parse(xml) as Parameters<
      typeof _rewriteSlideForTest
    >[0];
    const diagnostics: Diagnostic[] = [];
    const changed = _rewriteSlideForTest(parsed, diagnostics);
    expect(changed).toBe(false);
    expect(diagnostics).toHaveLength(0);
  });
});
