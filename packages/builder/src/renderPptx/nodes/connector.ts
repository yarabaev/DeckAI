import type { PositionedNode, LineArrow } from "../../types.ts";
import type { RenderContext, SlideBBox } from "../types.ts";
import { pxToIn, pxToPt } from "../units.ts";
import {
  SG_CXN_PREFIX,
  SG_GRP_PREFIX,
  buildObjectName,
} from "../postProcess/sigils.ts";

type ConnectorPositionedNode = Extract<PositionedNode, { type: "connector" }>;
type Side = "top" | "right" | "bottom" | "left";
type ConnectorKind = "straight" | "elbow" | "curved";

/**
 * Resolve a pptxgenjs arrow type from a `boolean | LineArrowOptions`
 * value. Mirrors the helper in `line.ts`; we keep a local copy so the
 * connector module is self-contained.
 */
function resolveArrowType(
  arrow: LineArrow | undefined,
): "none" | "arrow" | "diamond" | "oval" | "stealth" | "triangle" | undefined {
  if (arrow === undefined) return undefined;
  if (arrow === false) return "none";
  if (arrow === true) return "triangle";
  return arrow.type ?? "triangle";
}

const OPPOSITE: Record<Side, Side> = {
  top: "bottom",
  bottom: "top",
  left: "right",
  right: "left",
};

/**
 * Pick the side pair when the author omitted fromSide / toSide. Picks
 * the dominant axis (horizontal vs vertical) by comparing the absolute
 * distances between the bbox centers, then chooses the side that points
 * away from the source toward the target. PowerPoint will reroute the
 * preset path; this only seeds the initial side selection.
 */
export function autoSidePair(
  a: SlideBBox,
  b: SlideBBox,
): { fromSide: Side; toSide: Side } {
  const dx = b.x + b.w / 2 - (a.x + a.w / 2);
  const dy = b.y + b.h / 2 - (a.y + a.h / 2);
  if (Math.abs(dx) >= Math.abs(dy)) {
    return dx >= 0
      ? { fromSide: "right", toSide: "left" }
      : { fromSide: "left", toSide: "right" };
  }
  return dy >= 0
    ? { fromSide: "bottom", toSide: "top" }
    : { fromSide: "top", toSide: "bottom" };
}

/**
 * Pick the PPTX preset name for a (kind, fromSide, toSide) triple.
 *
 *   straight -> straightConnector1
 *   elbow   -> bentConnectorN  (N = segments)
 *   curved  -> curvedConnectorN
 *
 * The OOXML preset name convention uses "N" for the number of line
 * segments (one more than the number of bends):
 *
 *   facing sides (right<->left, top<->bottom)         -> N=3 (Z shape, 2 bends)
 *   perpendicular sides (right<->top, top<->left, ...) -> N=2 (L shape, 1 bend)
 *   same side (left<->left, top<->top, ...)            -> N=4 (U shape, 3 bends)
 *
 * straightConnector1 has no bends, so it ignores side choice.
 */
export function pickConnectorPreset(
  kind: ConnectorKind,
  fromSide: Side,
  toSide: Side,
): string {
  if (kind === "straight") return "straightConnector1";
  const facing = toSide === OPPOSITE[fromSide];
  const same = fromSide === toSide;
  const n = facing ? 3 : same ? 4 : 2;
  return kind === "elbow" ? `bentConnector${n}` : `curvedConnector${n}`;
}

/**
 * Locate the attachment point on a bbox for a given side. Returns the
 * midpoint of the chosen edge in slide-px coordinates.
 */
export function sideAnchor(
  bbox: SlideBBox,
  side: Side,
): { x: number; y: number } {
  switch (side) {
    case "top":
      return { x: bbox.x + bbox.w / 2, y: bbox.y };
    case "right":
      return { x: bbox.x + bbox.w, y: bbox.y + bbox.h / 2 };
    case "bottom":
      return { x: bbox.x + bbox.w / 2, y: bbox.y + bbox.h };
    case "left":
      return { x: bbox.x, y: bbox.y + bbox.h / 2 };
  }
}

/**
 * Build the cNvPr@name token chain the post-process pass reads. The
 * sg-cxn body is always present; sg-grp tokens stack outermost-first
 * when the connector itself sits inside one or more `group="..."`
 * ancestors, so the group rewriter wraps the cxnSp the same way it
 * wraps regular shapes.
 */
export function buildConnectorSigil(
  node: ConnectorPositionedNode,
  fromSide: Side,
  toSide: Side,
  preset: string,
  groupIds?: readonly string[],
): string {
  const kind: ConnectorKind = node.kind ?? "straight";
  const cxn = `${SG_CXN_PREFIX}${node.from}#${fromSide}>${node.to}#${toSide}:${kind}:${preset}`;
  const grpTokens = (groupIds ?? []).map((g) => `${SG_GRP_PREFIX}${g}`);
  // Source-position token is appended too: connectors with __nodeId let
  // editors jump from the cxn line back to the <Connector> source.
  const nodeIdToken =
    node.__nodeId !== undefined ? `node#${node.__nodeId}` : undefined;
  return buildObjectName([cxn, ...grpTokens, nodeIdToken]);
}

export function renderConnectorNode(
  node: ConnectorPositionedNode,
  ctx: RenderContext,
): void {
  const idIndex = ctx.idIndex;
  // parseXml drops invalid connectors before this point, so the lookup
  // should always succeed. Guard anyway: if a future refactor breaks
  // that invariant we want a silent no-op rather than a crash.
  if (!idIndex) return;
  const a = idIndex.get(node.from);
  const b = idIndex.get(node.to);
  if (!a || !b) return;

  const auto = autoSidePair(a, b);
  const fromSide: Side = node.fromSide ?? auto.fromSide;
  const toSide: Side = node.toSide ?? auto.toSide;
  const kind: ConnectorKind = node.kind ?? "straight";
  const preset = pickConnectorPreset(kind, fromSide, toSide);

  const p1 = sideAnchor(a, fromSide);
  const p2 = sideAnchor(b, toSide);
  const minX = Math.min(p1.x, p2.x);
  const minY = Math.min(p1.y, p2.y);
  const w = Math.abs(p2.x - p1.x);
  const h = Math.abs(p2.y - p1.y);
  // pptxgenjs uses flipH / flipV to encode line direction inside the
  // bounding box; we copy the convention from renderLineNode.
  const flipH = p2.x < p1.x;
  const flipV = p2.y < p1.y;

  ctx.slide.addShape(ctx.pptx.ShapeType.line, {
    x: pxToIn(minX),
    y: pxToIn(minY),
    w: pxToIn(w),
    h: pxToIn(h),
    flipH,
    flipV,
    // The sigil here is the entire channel the post-process pass uses
    // to find this shape and rewrite it as a real <p:cxnSp>. Do not
    // omit it even when trackSourcePos is off. Group ancestors are
    // threaded in so the group rewriter wraps connectors too.
    objectName: buildConnectorSigil(
      node,
      fromSide,
      toSide,
      preset,
      ctx.groupStack,
    ),
    line: {
      color: node.color ?? "000000",
      width: node.lineWidth !== undefined ? pxToPt(node.lineWidth) : 1,
      dashType: node.dashType,
      beginArrowType: resolveArrowType(node.beginArrow),
      endArrowType: resolveArrowType(node.endArrow),
    },
  });
}
