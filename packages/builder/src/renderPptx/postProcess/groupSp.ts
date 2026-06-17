/**
 * Post-process step that wraps shapes carrying a `sg-grp:G` sigil into
 * real PPTX `<p:grpSp>` group elements. Authors opt in via the
 * `group="..."` attribute on any container (or any node); the renderer
 * threads the active group stack through every leaf and emits one
 * `sg-grp:G` token per ancestor group on each shape's cNvPr@name.
 *
 * The rewriter consumes those tokens after the connector pass has
 * finalised cxnSp elements. Order of operations matters: cxn elements
 * still need group wrapping when an author placed the connector
 * inside a grouped container.
 *
 * Nested groups: built top-down. At each depth we scan the current
 * level's siblings for contiguous runs sharing the same group id,
 * recurse into deeper depths for those runs, then wrap the recursion
 * result in a grpSp. This produces properly nested `<p:grpSp>`
 * elements without the brittle "find members in the working array
 * after earlier wraps moved them" bookkeeping a bottom-up pass would
 * require.
 *
 * Non-contiguous groups: members sharing a group id but not adjacent
 * in the spTree open a fresh grpSp for each run. PPTX `<p:grpSp>`
 * requires its members to be siblings in z-order so we never reorder
 * to merge non-contiguous spans.
 */

import type { Diagnostic } from "../../diagnostics.ts";
import {
  SG_GRP_PREFIX,
  SG_ID_PREFIX,
  parseGrpSigils,
  stripSigils,
} from "./sigils.ts";

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

function getAttr(node: ElementNode, key: string): string | undefined {
  return node[":@"]?.[`@_${key}`];
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

function findSpTreeChildren(
  root: NodeArray,
): { parent: ElementNode; children: NodeArray } | null {
  for (const n of root) {
    if (!isElementNode(n)) continue;
    if (getTag(n) !== "p:sld") continue;
    for (const c of getChildren(n)) {
      if (!isElementNode(c) || getTag(c) !== "p:cSld") continue;
      for (const g of getChildren(c)) {
        if (!isElementNode(g) || getTag(g) !== "p:spTree") continue;
        return { parent: g, children: getChildren(g) };
      }
    }
  }
  return null;
}

/**
 * Read the cNvPr@name from a top-level spTree child. Members are
 * `<p:sp>` / `<p:cxnSp>` / `<p:pic>` / `<p:graphicFrame>` — every
 * type carries an `nv*Pr > cNvPr` ancestor in the OOXML schema.
 */
function readCNvPrName(element: ElementNode): {
  cNvPr: ElementNode | undefined;
  name: string | undefined;
} {
  const cNvPr = findDescendant([element], "p:cNvPr");
  if (!cNvPr) return { cNvPr: undefined, name: undefined };
  return { cNvPr, name: getAttr(cNvPr, "name") };
}

interface BBoxEmu {
  x: number;
  y: number;
  cx: number;
  cy: number;
}

function readBBox(element: ElementNode): BBoxEmu | null {
  const spPr =
    findDescendant([element], "p:spPr") ??
    findDescendant([element], "p:grpSpPr");
  if (!spPr) return null;
  const xfrm = findDescendant([spPr], "a:xfrm");
  if (!xfrm) return null;
  const off = findDescendant([xfrm], "a:off");
  const ext = findDescendant([xfrm], "a:ext");
  if (!off || !ext) return null;
  const x = Number(getAttr(off, "x"));
  const y = Number(getAttr(off, "y"));
  const cx = Number(getAttr(ext, "cx"));
  const cy = Number(getAttr(ext, "cy"));
  if (!Number.isFinite(x) || !Number.isFinite(y)) return null;
  if (!Number.isFinite(cx) || !Number.isFinite(cy)) return null;
  return { x, y, cx, cy };
}

function unionBBox(boxes: readonly BBoxEmu[]): BBoxEmu | null {
  if (boxes.length === 0) return null;
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const b of boxes) {
    if (b.x < minX) minX = b.x;
    if (b.y < minY) minY = b.y;
    const r = b.x + b.cx;
    const btm = b.y + b.cy;
    if (r > maxX) maxX = r;
    if (btm > maxY) maxY = btm;
  }
  return { x: minX, y: minY, cx: maxX - minX, cy: maxY - minY };
}

/**
 * Read the group id at a given depth from an element's sigil chain.
 * Returns undefined when the element does not belong to a group at
 * that depth (either no group at all, or shallower-only nesting).
 */
function groupAtDepth(element: ElementNode, depth: number): string | undefined {
  const { name } = readCNvPrName(element);
  const groups = parseGrpSigils(name);
  return groups[depth];
}

/**
 * Build the `<p:grpSp>` element that wraps the supplied members. The
 * group's xfrm uses the union bbox of its members; chOff/chExt mirror
 * off/ext (no nested coordinate transform), matching what PowerPoint
 * writes when a user groups shapes via the UI.
 */
function buildGrpSpElement(
  groupId: string,
  members: NodeArray,
  spIdCounter: { next: number },
): ElementNode {
  const elementMembers = members.filter(isElementNode);
  const bbox = unionBBox(
    elementMembers
      .map((m) => readBBox(m))
      .filter((b): b is BBoxEmu => b !== null),
  );
  const x = String(bbox?.x ?? 0);
  const y = String(bbox?.y ?? 0);
  const cx = String(bbox?.cx ?? 0);
  const cy = String(bbox?.cy ?? 0);
  // OOXML requires unique spIds across a slide. We pick fresh ones at
  // the high end of the existing range to avoid collisions with the
  // ids pptxgenjs assigned to real shapes.
  const id = String(spIdCounter.next++);
  // cNvPr@name carries the group id verbatim so PowerPoint's
  // "selection pane" lists the group with a human-readable label.
  return {
    "p:grpSp": [
      {
        "p:nvGrpSpPr": [
          { "p:cNvPr": [], ":@": { "@_id": id, "@_name": groupId } },
          { "p:cNvGrpSpPr": [] },
          { "p:nvPr": [] },
        ],
      },
      {
        "p:grpSpPr": [
          {
            "a:xfrm": [
              { "a:off": [], ":@": { "@_x": x, "@_y": y } },
              { "a:ext": [], ":@": { "@_cx": cx, "@_cy": cy } },
              { "a:chOff": [], ":@": { "@_x": x, "@_y": y } },
              { "a:chExt": [], ":@": { "@_cx": cx, "@_cy": cy } },
            ],
          },
        ],
      },
      ...members,
    ],
  };
}

/**
 * Build the nested grpSp structure top-down. At each call, scans the
 * supplied list of siblings for contiguous runs sharing the same
 * group id at `depth`, recurses into deeper depths for those runs,
 * then wraps the recursion result in a grpSp. Elements that carry no
 * group at this depth pass through unchanged. The result is a fresh
 * array; callers replace the parent's children wholesale.
 *
 * `flag.changed` becomes true the first time the algorithm produces
 * any grpSp wrap so the slide-level pass can skip the re-serialise
 * step on a no-op slide.
 */
function buildGroupedChildren(
  siblings: NodeArray,
  depth: number,
  spIdCounter: { next: number },
  flag: { changed: boolean },
): NodeArray {
  const result: NodeArray = [];
  let i = 0;
  while (i < siblings.length) {
    const head = siblings[i];
    if (!head || !isElementNode(head)) {
      if (head) result.push(head);
      i += 1;
      continue;
    }
    const groupId = groupAtDepth(head, depth);
    if (groupId === undefined) {
      result.push(head);
      i += 1;
      continue;
    }
    // Collect contiguous siblings sharing this group id at this depth.
    let j = i;
    while (j < siblings.length) {
      const next = siblings[j];
      if (!next || !isElementNode(next)) break;
      if (groupAtDepth(next, depth) !== groupId) break;
      j += 1;
    }
    const span = siblings.slice(i, j);
    // Recurse one level deeper so nested groups inside this run get
    // their own grpSp before we wrap the outer one.
    const nested = buildGroupedChildren(span, depth + 1, spIdCounter, flag);
    result.push(buildGrpSpElement(groupId, nested, spIdCounter));
    flag.changed = true;
    i = j;
  }
  return result;
}

/**
 * Compute the next spId to use for synthetic group elements. Scans
 * every cNvPr@id on the slide and returns max + 1.
 */
function nextSpIdFromSlide(parsed: NodeArray): number {
  let max = 0;
  const walk = (nodes: NodeArray): void => {
    for (const n of nodes) {
      if (!isElementNode(n)) continue;
      if (getTag(n) === "p:cNvPr") {
        const idStr = getAttr(n, "id");
        if (idStr) {
          const v = Number(idStr);
          if (Number.isFinite(v) && v > max) max = v;
        }
      }
      const kids = getChildren(n);
      if (kids.length > 0) walk(kids);
    }
  };
  walk(parsed);
  return max + 1;
}

/**
 * Final cNvPr@name cleanup over the spTree subtree. Removes every
 * remaining sg-* token (sg-id should already be gone, sg-grp tokens
 * are gone now that wrapping is done) so the final PPTX never leaks
 * builder-internal markers.
 */
function stripAllRemainingSigils(parent: ElementNode): void {
  const walk = (nodes: NodeArray): void => {
    for (const n of nodes) {
      if (!isElementNode(n)) continue;
      if (getTag(n) === "p:cNvPr") {
        const name = getAttr(n, "name");
        if (name) {
          const cleaned = stripSigils(name);
          if (cleaned !== name) {
            if (cleaned.length > 0) {
              setAttr(n, "name", cleaned);
            } else {
              removeAttr(n, "name");
            }
          }
        }
      }
      const kids = getChildren(n);
      if (kids.length > 0) walk(kids);
    }
  };
  walk(getChildren(parent));
}

/**
 * Rewrite a single slide's parsed XML tree in place. Replaces the
 * spTree's children with the recursively-grouped variant produced by
 * `buildGroupedChildren`, then runs the final sigil cleanup. Returns
 * whether anything actually changed so the slide-level pass can skip
 * re-serialising untouched slides.
 */
export function rewriteSlideGroups(
  parsed: NodeArray,
  _diagnostics: Diagnostic[],
): boolean {
  const located = findSpTreeChildren(parsed);
  if (!located) return false;

  const spIdCounter = { next: nextSpIdFromSlide(parsed) };
  const flag = { changed: false };
  const grouped = buildGroupedChildren(located.children, 0, spIdCounter, flag);
  if (flag.changed) {
    located.parent["p:spTree"] = grouped;
  }

  // Even when no group spans were produced (no sg-grp tokens at all),
  // we still need to strip any leftover sg-id markers the connector
  // pass missed for shapes the rewriter skipped.
  const before = JSON.stringify(located.children);
  stripAllRemainingSigils(located.parent);
  const after = JSON.stringify(getChildren(located.parent));
  return flag.changed || before !== after;
}

export { SG_GRP_PREFIX, SG_ID_PREFIX };
