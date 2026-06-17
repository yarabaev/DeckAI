/**
 * Post-process step that rewrites pptxgenjs' `<p:sp>` placeholders into
 * real PPTX `<p:cxnSp>` elements with stCxn / endCxn bindings.
 *
 * Why post-process? pptxgenjs has no API for connector shapes — every
 * `addShape("line", ...)` writes a regular `<p:sp prstGeom="line">`.
 * The Connector renderer (`renderPptx/nodes/connector.ts`) tags its
 * placeholder via `<p:cNvPr name="sg-cxn:...">` so we can find it here
 * after the slide XML is otherwise complete, and rewrite the element
 * in-place. Author ids that should be reachable from connectors are
 * tagged with `sg-id:USER_ID` on the same channel.
 *
 * The rewrite covers every `ppt/slides/slide*.xml` in the zip; other
 * parts (theme, master, rels) are left untouched.
 *
 * Invariants this pass relies on (set up by the renderer):
 *   - Every sg-cxn placeholder has its from / to user ids present on
 *     the same slide (parseXml drops the connector otherwise).
 *   - Both endpoints have `sg-id:` markers in their cNvPr@name.
 *   - The cxn placeholder is rendered as a line shape, so its spPr
 *     already carries a valid xfrm bounding box.
 */

import type { Diagnostic } from "../../diagnostics.ts";
import {
  SG_CXN_PREFIX,
  SG_ID_PREFIX,
  parseCxnSigil,
  parseIdSigil,
  stripSigilsByPrefix,
} from "./sigils.ts";
import { lookupSideIdx, isKnownPrstForCxn } from "./idxTable.ts";
import { rewriteSlideGroups } from "./groupSp.ts";

// JSZip and fast-xml-parser are CJS — load via dynamic import to handle
// default export differences across runtimes (Node vs browser bundlers).
async function loadJSZip(): Promise<typeof import("jszip")> {
  const mod = await import("jszip");
  /* eslint-disable @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-return */
  return (mod as any).default ?? mod;
  /* eslint-enable @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-return */
}

async function loadXmlClasses(): Promise<{
  XMLParser: typeof import("fast-xml-parser").XMLParser;
  XMLBuilder: typeof import("fast-xml-parser").XMLBuilder;
}> {
  const mod = await import("fast-xml-parser");
  return { XMLParser: mod.XMLParser, XMLBuilder: mod.XMLBuilder };
}

/**
 * fast-xml-parser preserveOrder output: each element is `{ TAG: NODE[],
 * ":@"?: { "@_attr": "value" } }`. We work with this representation
 * directly so element order survives the round-trip.
 */
type Attrs = Record<string, string>;
type ElementNode = {
  [tag: string]: NodeArray | Attrs | undefined;
  ":@"?: Attrs;
};
type TextNode = { "#text": string };
type AnyNode = ElementNode | TextNode;
type NodeArray = AnyNode[];

function isElementNode(node: AnyNode): node is ElementNode {
  return !("#text" in node);
}

function getTag(node: ElementNode): string {
  for (const key of Object.keys(node)) {
    if (key !== ":@") return key;
  }
  return "";
}

function getChildren(node: ElementNode): NodeArray {
  const tag = getTag(node);
  const value = node[tag];
  return Array.isArray(value) ? value : [];
}

function getAttrs(node: ElementNode): Attrs | undefined {
  return node[":@"];
}

function setAttr(node: ElementNode, key: string, value: string): void {
  let attrs = node[":@"];
  if (!attrs) {
    attrs = {};
    node[":@"] = attrs;
  }
  attrs[`@_${key}`] = value;
}

function removeAttr(node: ElementNode, key: string): void {
  const attrs = node[":@"];
  if (attrs) delete attrs[`@_${key}`];
}

function getAttr(node: ElementNode, key: string): string | undefined {
  return node[":@"]?.[`@_${key}`];
}

/**
 * Locate the first descendant with the given tag (depth-first). Used
 * for spot-lookups inside an sp subtree where positions are stable but
 * we don't want to hard-code paths.
 */
function findDescendant(
  nodes: NodeArray,
  tag: string,
): ElementNode | undefined {
  for (const n of nodes) {
    if (!isElementNode(n)) continue;
    const nt = getTag(n);
    if (nt === tag) return n;
    const children = getChildren(n);
    if (children.length > 0) {
      const inner = findDescendant(children, tag);
      if (inner) return inner;
    }
  }
  return undefined;
}

/**
 * Walk the spTree children of a slide. The spTree itself lives at
 * p:sld > p:cSld > p:spTree.
 */
function findSpTreeChildren(root: NodeArray): NodeArray | null {
  for (const n of root) {
    if (!isElementNode(n)) continue;
    if (getTag(n) !== "p:sld") continue;
    for (const c of getChildren(n)) {
      if (!isElementNode(c) || getTag(c) !== "p:cSld") continue;
      for (const g of getChildren(c)) {
        if (!isElementNode(g) || getTag(g) !== "p:spTree") continue;
        return getChildren(g);
      }
    }
  }
  return null;
}

interface ShapeMeta {
  spId: string;
  prst: string | undefined;
}

interface ConnectorJob {
  element: ElementNode;
  parsed: ReturnType<typeof parseCxnSigil>;
}

/**
 * Rewrite a single slide's parsed XML tree in place. Returns whether
 * any changes were made (callers skip the re-serialize step when
 * nothing changed).
 */
function rewriteSlide(parsed: NodeArray, diagnostics: Diagnostic[]): boolean {
  const spTreeChildren = findSpTreeChildren(parsed);
  if (!spTreeChildren) return false;

  // Pass 1: walk every sp directly under spTree, build userId -> meta
  // and queue cxn placeholders.
  const idMap = new Map<string, ShapeMeta>();
  const jobs: ConnectorJob[] = [];
  let changed = false;

  for (const child of spTreeChildren) {
    if (!isElementNode(child)) continue;
    const tag = getTag(child);
    if (tag !== "p:sp") continue;

    // Find p:nvSpPr > p:cNvPr to read the sigil chain.
    const nvSpPr = findDescendant([child], "p:nvSpPr");
    const cNvPr = nvSpPr ? findDescendant([nvSpPr], "p:cNvPr") : undefined;
    if (!cNvPr) continue;

    const spId = getAttr(cNvPr, "id");
    const name = getAttr(cNvPr, "name");
    if (!spId || !name) continue;

    const idSig = parseIdSigil(name);
    if (idSig) {
      // The prst lives at p:spPr > a:prstGeom@prst.
      const prstGeom = findDescendant([child], "a:prstGeom");
      const prst = prstGeom ? getAttr(prstGeom, "prst") : undefined;
      idMap.set(idSig.userId, { spId, prst });
    }

    const cxnSig = parseCxnSigil(name);
    if (cxnSig) {
      // Connector placeholders go into the rewrite queue. The sigil
      // strip happens inside rewriteSpToCxnSp so the cNvPr@name is
      // consistent with the rewritten element.
      jobs.push({ element: child, parsed: cxnSig });
      continue;
    }

    // For non-connector elements: only sg-id consumed here. Strip the
    // sg-id token if it was present so subsequent passes / final
    // output don't see it. sg-grp tokens are intentionally preserved —
    // the group rewriter consumes them in the next pass.
    if (idSig) {
      const stripped = stripSigilsByPrefix(name, [SG_ID_PREFIX]);
      if (stripped !== name) {
        if (stripped.length > 0) {
          setAttr(cNvPr, "name", stripped);
        } else {
          removeAttr(cNvPr, "name");
        }
        changed = true;
      }
    }
  }

  // Pass 2: rewrite each cxn placeholder. We do this after pass 1 has
  // populated idMap so order of declaration on the slide is irrelevant.
  for (const job of jobs) {
    const sig = job.parsed;
    if (!sig) continue;
    const fromMeta = idMap.get(sig.from);
    const toMeta = idMap.get(sig.to);
    // parseXml already drops connectors with unknown endpoints; reaching
    // here with a missing meta means renderPptx emitted a placeholder
    // whose target id was never tagged. Safest is to skip the rewrite
    // and leave the placeholder as a plain line.
    if (!fromMeta || !toMeta) continue;

    if (!isKnownPrstForCxn(fromMeta.prst)) {
      diagnostics.push({
        code: "CONNECTOR_UNKNOWN_SHAPE_IDX",
        message: `Connector binds to shape id="${sig.from}" with unfamiliar preset prst="${fromMeta.prst ?? "<none>"}"; using default 4-side idx table.`,
      });
    }
    if (!isKnownPrstForCxn(toMeta.prst)) {
      diagnostics.push({
        code: "CONNECTOR_UNKNOWN_SHAPE_IDX",
        message: `Connector binds to shape id="${sig.to}" with unfamiliar preset prst="${toMeta.prst ?? "<none>"}"; using default 4-side idx table.`,
      });
    }

    rewriteSpToCxnSp(job.element, sig, fromMeta, toMeta);
    changed = true;
  }

  return changed;
}

/**
 * Mutate a placeholder `<p:sp>` into a `<p:cxnSp>` with the resolved
 * stCxn / endCxn bindings and the chosen preset. The element keeps
 * the same identity (same JS object) so the surrounding spTree array
 * does not need reshuffling.
 */
function rewriteSpToCxnSp(
  element: ElementNode,
  sig: NonNullable<ReturnType<typeof parseCxnSigil>>,
  fromMeta: ShapeMeta,
  toMeta: ShapeMeta,
): void {
  const spChildren = getChildren(element);
  // Rename the outer tag p:sp -> p:cxnSp by swapping the key.
  delete element["p:sp"];
  element["p:cxnSp"] = spChildren;

  // p:nvSpPr -> p:nvCxnSpPr (and inner p:cNvSpPr -> p:cNvCxnSpPr).
  for (const child of spChildren) {
    if (!isElementNode(child)) continue;
    const tag = getTag(child);
    if (tag === "p:nvSpPr") {
      const inner = getChildren(child);
      delete child["p:nvSpPr"];
      child["p:nvCxnSpPr"] = inner;

      // Strip the sg-cxn sigil from the cNvPr@name. sg-grp tokens
      // stay so the group rewriter can wrap this connector along with
      // its sibling shapes when both share a group ancestor.
      const cNvPr = findDescendant(inner, "p:cNvPr");
      if (cNvPr) {
        const name = getAttr(cNvPr, "name");
        const stripped = stripSigilsByPrefix(name, [SG_CXN_PREFIX]);
        if (stripped) {
          setAttr(cNvPr, "name", stripped);
        } else {
          removeAttr(cNvPr, "name");
        }
      }

      // Locate p:cNvSpPr inside the renamed group and rename it. The
      // OOXML schema lists p:cNvCxnSpPr as the cxnSp counterpart; it
      // carries the stCxn / endCxn children that bind the line to two
      // spIds with their connection-site indices.
      for (let i = 0; i < inner.length; i++) {
        const c = inner[i];
        if (!c || !isElementNode(c)) continue;
        const ctag = getTag(c);
        if (ctag !== "p:cNvSpPr") continue;
        const cSpChildren = getChildren(c);
        // Inject stCxn / endCxn at the head of cNvCxnSpPr's children.
        const fromIdx = lookupSideIdx(fromMeta.prst)[sig.fromSide];
        const toIdx = lookupSideIdx(toMeta.prst)[sig.toSide];
        const stCxn: ElementNode = {
          "a:stCxn": [],
          ":@": { "@_id": fromMeta.spId, "@_idx": String(fromIdx) },
        };
        const endCxn: ElementNode = {
          "a:endCxn": [],
          ":@": { "@_id": toMeta.spId, "@_idx": String(toIdx) },
        };
        const newInner: NodeArray = [stCxn, endCxn, ...cSpChildren];
        const replacement: ElementNode = { "p:cNvCxnSpPr": newInner };
        const attrs = getAttrs(c);
        if (attrs) replacement[":@"] = attrs;
        inner[i] = replacement;
        break;
      }
    } else if (tag === "p:spPr") {
      // Swap prstGeom@prst from "line" to the picked connector preset.
      const prstGeom = findDescendant(getChildren(child), "a:prstGeom");
      if (prstGeom) {
        setAttr(prstGeom, "prst", sig.preset);
      }
    }
  }
}

/**
 * Apply the connector rewrite over every ppt/slides/slide*.xml in a
 * PPTX byte stream. Returns the rewritten bytes plus any diagnostics
 * surfaced during the pass.
 *
 * When the input contains no connector placeholders, the function
 * still does the unzip / re-zip round-trip — JSZip's compression is
 * fast enough that we don't optimise the no-op case. If you need to
 * skip rewriting for performance, gate the call site on the presence
 * of a Connector node at parse time.
 */
export async function postProcessConnectors(
  bytes: Uint8Array,
): Promise<{ bytes: Uint8Array; diagnostics: Diagnostic[] }> {
  const JSZip = await loadJSZip();
  const { XMLParser, XMLBuilder } = await loadXmlClasses();
  const diagnostics: Diagnostic[] = [];

  const parser = new XMLParser({
    preserveOrder: true,
    ignoreAttributes: false,
    attributeNamePrefix: "@_",
    parseAttributeValue: false,
    parseTagValue: false,
    trimValues: false,
    processEntities: true,
  });
  const builder = new XMLBuilder({
    preserveOrder: true,
    ignoreAttributes: false,
    attributeNamePrefix: "@_",
    // Match pptxgenjs' own self-closing style for empty elements so
    // the round-trip touches only the connector subtree, leaving
    // every other shape's serialization unchanged. PowerPoint accepts
    // both forms, but minimising the diff makes review and any future
    // visual regression check cleaner.
    suppressEmptyNode: true,
    suppressBooleanAttributes: false,
    processEntities: true,
  });

  const zip = await JSZip.loadAsync(bytes);
  const slideFiles = Object.keys(zip.files).filter((p) =>
    /^ppt\/slides\/slide\d+\.xml$/.test(p),
  );
  for (const path of slideFiles) {
    const file = zip.file(path);
    if (!file) continue;
    const xml = await file.async("string");
    const parsed = parser.parse(xml) as NodeArray;
    // Order matters: connectors first (so cxnSp elements exist when
    // groups wrap them), then groups (which strip all remaining sg-*
    // tokens as a final cleanup).
    const cxnChanged = rewriteSlide(parsed, diagnostics);
    const grpChanged = rewriteSlideGroups(parsed, diagnostics);
    if (!cxnChanged && !grpChanged) continue;
    const rebuilt = builder.build(parsed);
    zip.file(path, rebuilt);
  }

  const out = await zip.generateAsync({
    type: "uint8array",
    compression: "DEFLATE",
    compressionOptions: { level: 6 },
  });
  return { bytes: out, diagnostics };
}

export { rewriteSlide as _rewriteSlideForTest };
