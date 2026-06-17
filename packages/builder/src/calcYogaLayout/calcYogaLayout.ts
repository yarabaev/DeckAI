import type { BuilderNode, AlignItems } from "../types.ts";
import type { BuildContext } from "../buildContext.ts";
import type { YogaNodeMap } from "./types.ts";
import { Node as YogaNode } from "yoga-layout";
import { loadYoga } from "yoga-layout/load";
import { prefetchImageSize } from "../shared/measureImage.ts";
import { freeYogaTree } from "../shared/freeYogaTree.ts";
import { getNodeDef } from "../registry/index.ts";
import { validateImageSrc } from "../renderPptx/nodes/image.ts";

/**
 * Compute the Yoga layout for a BuilderNode tree.
 * Returns the BuilderNode-to-YogaNode mapping as a YogaNodeMap.
 *
 * @param root Root of the input BuilderNode tree.
 * @param slideSize Overall slide size (px).
 * @param ctx BuildContext
 * @returns YogaNodeMap — BuilderNode -> YogaNode.
 */
export async function calcYogaLayout(
  root: BuilderNode,
  slideSize: { w: number; h: number },
  ctx: BuildContext,
): Promise<YogaNodeMap> {
  const Yoga = await getYoga();

  // Prefetch every image size up front (for HTTPS support).
  await prefetchAllImageSizes(root, ctx);

  const map: YogaNodeMap = new Map();

  try {
    const rootYoga = Yoga.Node.create();
    map.set(root, rootYoga);

    await buildPomWithYogaTree(root, rootYoga, ctx, map);

    // Pass the overall slide size.
    rootYoga.setWidth(slideSize.w);
    rootYoga.setHeight(slideSize.h);

    rootYoga.calculateLayout(slideSize.w, slideSize.h, Yoga.DIRECTION_LTR);
  } catch (e) {
    // On a mid-pass failure, free the created YogaNodes before re-throwing.
    freeYogaTree(map);
    throw e;
  }

  return map;
}

/**
 * Prefetches sizes for all images in the BuilderNode tree.
 * Applies imageSrcGuard before issuing any network/fs calls to prevent SSRF
 * and unauthorized file reads at the prefetch phase.
 */
async function prefetchAllImageSizes(
  node: BuilderNode,
  ctx: BuildContext,
): Promise<void> {
  const guard = ctx.security.imageSrcGuard;
  const imageSources = collectImageSources(node);
  const allowedSources = guard
    ? imageSources.filter(
        (src) => validateImageSrc(src, guard, ctx.diagnostics) !== undefined,
      )
    : imageSources;
  await Promise.all(
    allowedSources.map((src) =>
      prefetchImageSize(
        src,
        ctx.imageSizeCache,
        ctx.imageDataCache,
        ctx.diagnostics,
      ),
    ),
  );
}

/**
 * Collect every image `src` referenced anywhere in a BuilderNode tree.
 */
function collectImageSources(node: BuilderNode): string[] {
  const sources: string[] = [];

  function traverse(n: BuilderNode) {
    // Collect backgroundImage src (shared across every node).
    if (n.backgroundImage) {
      sources.push(n.backgroundImage.src);
    }

    const def = getNodeDef(n.type);

    // Node-specific image-source collection.
    if (def.collectImageSources) {
      sources.push(...def.collectImageSources(n));
    }

    // Recurse into child elements.
    switch (def.category) {
      case "multi-child":
      case "absolute-child": {
        const containerNode = n as Extract<
          BuilderNode,
          { type: "vstack" | "hstack" | "layer" }
        >;
        for (const child of containerNode.children) {
          traverse(child);
        }
        break;
      }
    }
  }

  traverse(node);
  return sources;
}

/**
 * Yoga singleton.
 */
let yogaP: Promise<Yoga> | null = null;
type Yoga = Awaited<ReturnType<typeof loadYoga>>;
async function getYoga(): Promise<Yoga> {
  if (yogaP === null) yogaP = loadYoga();
  return yogaP;
}

/**
 * Whether the node has a determinate cross-axis size (width).
 * - Determinate when an explicit w is provided.
 * - Indeterminate when alignSelf is specified as something other than stretch.
 * - Indeterminate when the parent's alignItems is specified as something other than stretch (the default).
 */
function nodeHasDefiniteWidth(
  node: BuilderNode,
  parentNode?: BuilderNode,
): boolean {
  // Has an explicit width.
  if (node.w !== undefined) return true;

  // Indeterminate when alignSelf is explicitly something other than stretch.
  if (
    node.alignSelf !== undefined &&
    node.alignSelf !== "stretch" &&
    node.alignSelf !== "auto"
  ) {
    return false;
  }

  // Root nodes (no parent) are determinate.
  if (!parentNode) return true;

  // Read the parent's alignItems (only VStack/HStack carry it).
  let parentAlignItems: AlignItems | undefined;
  if (parentNode.type === "vstack") {
    parentAlignItems = parentNode.alignItems;
  } else if (parentNode.type === "hstack") {
    parentAlignItems = parentNode.alignItems;
  }

  // For a VStack (column-direction) child, the cross-axis is horizontal.
  // With alignItems stretch (the default), children are stretched to the parent's width.
  if (parentNode.type === "vstack" || parentNode.type === "layer") {
    return parentAlignItems === undefined || parentAlignItems === "stretch";
  }

  // For an HStack child, the width is decided by main-axis flex, so it's treated as determinate.
  if (parentNode.type === "hstack") {
    return true;
  }

  return true;
}

/**
 * Recursively walk the BuilderNode tree and build a matching YogaNode tree.
 */
async function buildPomWithYogaTree(
  node: BuilderNode,
  parentYoga: YogaNode,
  ctx: BuildContext,
  map: YogaNodeMap,
  parentNode?: BuilderNode,
  grandparentNode?: BuilderNode,
) {
  const yoga = await getYoga();

  const yn = yoga.Node.create();
  map.set(node, yn); // support YogaNode map register

  await applyStyleToYogaNode(node, yn, ctx);

  // Detect whether any sibling has explicitly nominated itself to absorb
  // main-axis slack via `w="max"` (HStack) or `h="max"` (VStack). When such
  // a "growing sibling" exists, the other siblings stop participating in
  // the equal-distribution heuristic below and pin to their intrinsic
  // size: the growing sibling alone takes the slack and absorbs any
  // overflow (its content wraps), preventing the regression where
  // a long-title sibling collapsed short ones (e.g. `<Text>01</Text>`)
  // down to single-character columns.
  const hasGrowingSibling =
    (parentNode?.type === "hstack" &&
      (parentNode as { children: BuilderNode[] }).children.some(
        (c) => c !== node && c.w === "max",
      )) ||
    (parentNode?.type === "vstack" &&
      (parentNode as { children: BuilderNode[] }).children.some(
        (c) => c !== node && c.h === "max",
      ));
  const isThisTheGrowingSibling =
    (parentNode?.type === "hstack" && node.w === "max") ||
    (parentNode?.type === "vstack" && node.h === "max");

  // Default HStack/VStack children to flexShrink=1 (matching CSS Flexbox).
  // Prevent overflow when main-axis %-size combines with gap.
  // Don't shrink an icon-only fixed-size content. Also pin children whose
  // main-axis size was set as an explicit pixel value: when an author writes
  // `<Text w="40">` they mean exactly 40px, so siblings with flex-grow or
  // intrinsic overflow must not steal space from the declared column.
  // Text-bearing nodes with `noWrap=true` likewise opt out: shrinking them
  // would re-trigger the wrap path the author opted out of.
  if (parentNode?.type === "hstack" || parentNode?.type === "vstack") {
    const mainAxisIsFixed =
      (parentNode.type === "hstack" && typeof node.w === "number") ||
      (parentNode.type === "vstack" && typeof node.h === "number");
    const isNoWrapText =
      (node.type === "text" ||
        node.type === "ul" ||
        node.type === "ol" ||
        node.type === "shape") &&
      node.noWrap === true;
    const noShrink =
      node.type === "icon" ||
      mainAxisIsFixed ||
      isNoWrapText ||
      (hasGrowingSibling && !isThisTheGrowingSibling);
    yn.setFlexShrink(noShrink ? 0 : 1);
  }

  // For HStack children with no explicit width, distribute the available width equally by default.
  // Skip table because setMeasureFunc returns the sum of column widths.
  // Skip icon-only fixed-size content.
  // Skip when a sibling claims the slack via `w="max"`: equal distribution
  // would re-stretch every column to share grow:1 with the max sibling,
  // collapsing intrinsic-sized siblings to zero basis.
  if (
    parentNode?.type === "hstack" &&
    node.w === undefined &&
    node.type !== "table" &&
    node.type !== "icon" &&
    !hasGrowingSibling
  ) {
    yn.setFlexGrow(1);
    // Only apply flexBasis=0 equal-distribution when the HStack has a determinate width.
    // When an HStack is auto-sized (e.g. parent alignItems center/start/end),
    // flexBasis=0 strips children of their natural width and breaks layout, so
    // keep flexBasis=auto (the default).
    if (nodeHasDefiniteWidth(parentNode, grandparentNode)) {
      yn.setFlexBasis(0);
    }
  }

  // Explicit flex-* overrides (always win over the context-aware defaults
  // above). Applied last so authors can pin a column with flexShrink=0 or
  // claim slack with flexGrow=2 even inside HStack/VStack defaults.
  if (node.flexGrow !== undefined) {
    yn.setFlexGrow(node.flexGrow);
  }
  if (node.flexShrink !== undefined) {
    yn.setFlexShrink(node.flexShrink);
  }
  if (node.flexBasis !== undefined) {
    if (typeof node.flexBasis === "number") {
      yn.setFlexBasis(node.flexBasis);
    } else if (node.flexBasis === "max") {
      // "max" on flexBasis is treated as flex-basis: auto so the node
      // sizes to its content first, then participates in grow.
      yn.setFlexBasisAuto();
    } else if (node.flexBasis.endsWith("%")) {
      yn.setFlexBasisPercent(parseFloat(node.flexBasis));
    }
  }

  parentYoga.insertChild(yn, parentYoga.getChildCount());

  const def = getNodeDef(node.type);

  switch (def.category) {
    case "multi-child":
    case "absolute-child": {
      const containerNode = node as Extract<
        BuilderNode,
        { type: "vstack" | "hstack" | "layer" }
      >;
      for (const child of containerNode.children) {
        await buildPomWithYogaTree(child, yn, ctx, map, node, parentNode);
      }
      break;
    }
    case "leaf":
      // No child elements.
      break;
  }
}

/**
 * Apply the node's style onto its YogaNode.
 */
async function applyStyleToYogaNode(
  node: BuilderNode,
  yn: YogaNode,
  ctx: BuildContext,
) {
  const yoga = await getYoga();

  // Default: vertical arrangement.
  yn.setFlexDirection(yoga.FLEX_DIRECTION_COLUMN);

  // width
  if (node.w !== undefined) {
    if (typeof node.w === "number") {
      yn.setWidth(node.w);
    } else if (node.w === "max") {
      yn.setFlexGrow(1);
    } else if (node.w.endsWith("%")) {
      const percent = parseFloat(node.w);
      yn.setWidthPercent(percent);
    }
  }

  // height
  if (node.h !== undefined) {
    if (typeof node.h === "number") {
      yn.setHeight(node.h);
    } else if (node.h === "max") {
      yn.setFlexGrow(1);
    } else if (node.h.endsWith("%")) {
      const percent = parseFloat(node.h);
      yn.setHeightPercent(percent);
    }
  }

  // min/max constraints
  if (node.minW !== undefined) {
    yn.setMinWidth(node.minW);
  }
  if (node.maxW !== undefined) {
    yn.setMaxWidth(node.maxW);
  }
  if (node.minH !== undefined) {
    yn.setMinHeight(node.minH);
  }
  if (node.maxH !== undefined) {
    yn.setMaxHeight(node.maxH);
  }

  // padding
  if (node.padding !== undefined) {
    if (typeof node.padding === "number") {
      yn.setPadding(yoga.EDGE_TOP, node.padding);
      yn.setPadding(yoga.EDGE_RIGHT, node.padding);
      yn.setPadding(yoga.EDGE_BOTTOM, node.padding);
      yn.setPadding(yoga.EDGE_LEFT, node.padding);
    } else {
      if (node.padding.top !== undefined) {
        yn.setPadding(yoga.EDGE_TOP, node.padding.top);
      }
      if (node.padding.right !== undefined) {
        yn.setPadding(yoga.EDGE_RIGHT, node.padding.right);
      }
      if (node.padding.bottom !== undefined) {
        yn.setPadding(yoga.EDGE_BOTTOM, node.padding.bottom);
      }
      if (node.padding.left !== undefined) {
        yn.setPadding(yoga.EDGE_LEFT, node.padding.left);
      }
    }
  }

  // margin
  if (node.margin !== undefined) {
    if (typeof node.margin === "number") {
      yn.setMargin(yoga.EDGE_TOP, node.margin);
      yn.setMargin(yoga.EDGE_RIGHT, node.margin);
      yn.setMargin(yoga.EDGE_BOTTOM, node.margin);
      yn.setMargin(yoga.EDGE_LEFT, node.margin);
    } else {
      if (node.margin.top !== undefined) {
        yn.setMargin(yoga.EDGE_TOP, node.margin.top);
      }
      if (node.margin.right !== undefined) {
        yn.setMargin(yoga.EDGE_RIGHT, node.margin.right);
      }
      if (node.margin.bottom !== undefined) {
        yn.setMargin(yoga.EDGE_BOTTOM, node.margin.bottom);
      }
      if (node.margin.left !== undefined) {
        yn.setMargin(yoga.EDGE_LEFT, node.margin.left);
      }
    }
  }

  // position
  if (node.position === "absolute") {
    yn.setPositionType(yoga.POSITION_TYPE_ABSOLUTE);
  }
  if (node.top !== undefined) {
    yn.setPosition(yoga.EDGE_TOP, node.top);
  }
  if (node.right !== undefined) {
    yn.setPosition(yoga.EDGE_RIGHT, node.right);
  }
  if (node.bottom !== undefined) {
    yn.setPosition(yoga.EDGE_BOTTOM, node.bottom);
  }
  if (node.left !== undefined) {
    yn.setPosition(yoga.EDGE_LEFT, node.left);
  }

  // alignSelf
  if (node.alignSelf !== undefined) {
    switch (node.alignSelf) {
      case "auto":
        yn.setAlignSelf(yoga.ALIGN_AUTO);
        break;
      case "start":
        yn.setAlignSelf(yoga.ALIGN_FLEX_START);
        break;
      case "center":
        yn.setAlignSelf(yoga.ALIGN_CENTER);
        break;
      case "end":
        yn.setAlignSelf(yoga.ALIGN_FLEX_END);
        break;
      case "stretch":
        yn.setAlignSelf(yoga.ALIGN_STRETCH);
        break;
    }
  }

  // Apply node-specific styles (e.g. measureFunc).
  const def = getNodeDef(node.type);
  if (def.applyYogaStyle) {
    await def.applyYogaStyle(node, yn, yoga, ctx);
  }
}
