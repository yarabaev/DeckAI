// pptxgenjs type alias
type PptxGenJSInstance = import("pptxgenjs").default;

// pptxgenjs is a CJS package — load via dynamic import
async function loadPptxGenJS(): Promise<new () => PptxGenJSInstance> {
  const pptxModule = await import("pptxgenjs");
  // Resolve CJS default export: module.default.default (ESM wrapper) or module.default
  /* eslint-disable @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-return */
  const mod = pptxModule as any;
  return mod.default?.default ?? mod.default ?? mod;
  /* eslint-enable @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-return */
}
type SlideMasterProps = Parameters<PptxGenJSInstance["defineSlideMaster"]>[0];
type SlideMasterObject = NonNullable<SlideMasterProps["objects"]>[number];
type ShapeName =
  PptxGenJSInstance["ShapeType"][keyof PptxGenJSInstance["ShapeType"]];
type ImageProps = {
  x: number;
  y: number;
  w: number;
  h: number;
  path?: string;
  data?: string;
};
import type {
  PositionedNode,
  SlideMasterOptions,
  MasterObject,
} from "../types.ts";
import type { BuildContext } from "../buildContext.ts";
import {
  resolveFontFamily,
  resolveTextStyleValue,
} from "../defaultTextStyle.ts";
import type { RenderContext } from "./types.ts";
import { pxToIn, pxToPt } from "./units.ts";
import { convertUnderline, convertStrike } from "./textOptions.ts";
import { getImageData } from "../shared/measureImage.ts";
import {
  renderBackgroundAndBorder,
  shouldEmbedBackgroundInText,
} from "./utils/backgroundBorder.ts";
import { getNodeDef } from "../registry/index.ts";
import { builderObjectName } from "./utils/objectName.ts";

/**
 * Wrap ctx.slide so that every `add*` call transparently carries the current
 * node's pptxgenjs `objectName`. Enables the source-map → SVG click pipeline
 * without requiring every render function to thread the id manually.
 *
 * Returns `ctx` unchanged when the node has no `__nodeId` (trackSourcePos
 * disabled) so hot paths avoid the proxy overhead.
 */
function wrapCtxWithObjectName(
  ctx: RenderContext,
  node: { __nodeId?: number; id?: string },
): RenderContext {
  const objectName = builderObjectName({
    __nodeId: node.__nodeId,
    id: node.id,
    groupIds: ctx.groupStack,
  });
  if (!objectName) return ctx;
  const target = ctx.slide as unknown as Record<string, unknown>;
  const proxy = new Proxy(target, {
    get(t, prop) {
      const value: unknown = Reflect.get(t, prop);
      if (typeof value !== "function") return value;
      if (typeof prop !== "string" || !prop.startsWith("add")) return value;
      return function (this: unknown, ...args: unknown[]) {
        // Inject objectName into the last argument when it's an options object
        // and no explicit objectName was supplied. Works for addShape(name,
        // options), addText(text, options), addImage(options), addTable
        // (rows, options), addChart(type, data, options).
        const lastIdx = args.length - 1;
        if (lastIdx >= 0) {
          const last = args[lastIdx];
          if (
            last &&
            typeof last === "object" &&
            !Array.isArray(last) &&
            !(last as Record<string, unknown>).objectName
          ) {
            args[lastIdx] = {
              ...(last as Record<string, unknown>),
              objectName,
            };
          }
        }
        return (value as (...a: unknown[]) => unknown).apply(t, args);
      };
    },
  });
  return { ...ctx, slide: proxy as unknown as RenderContext["slide"] };
}

type SlidePx = { w: number; h: number };

const DEFAULT_MASTER_NAME = "SLIDEGLANCE_MASTER";

/**
 * Sorts children by zIndex to control paint order (stable sort).
 * Lower zIndex is painted first because PowerPoint layers in insertion order.
 */
function sortByZIndex<T extends { zIndex?: number }>(children: T[]): T[] {
  // Return as-is when no child has a zIndex set
  if (children.every((c) => c.zIndex === undefined)) return children;
  return [...children].sort((a, b) => (a.zIndex ?? 0) - (b.zIndex ?? 0));
}

/**
 * Converts a MasterObject to the pptxgenjs objects format.
 */
function convertMasterObject(
  obj: MasterObject,
  buildContext: BuildContext,
): SlideMasterObject {
  const defaultTextStyle = buildContext.defaultTextStyle;
  switch (obj.type) {
    case "text":
      return {
        text: {
          text: obj.text,
          options: {
            x: pxToIn(obj.x),
            y: pxToIn(obj.y),
            w: pxToIn(obj.w),
            h: pxToIn(obj.h),
            fontSize:
              obj.fontSize !== undefined ||
              defaultTextStyle.fontSize !== undefined
                ? pxToPt(
                    resolveTextStyleValue(
                      obj.fontSize,
                      defaultTextStyle.fontSize,
                      24,
                    ),
                  )
                : undefined,
            fontFace: resolveFontFamily(obj.fontFamily, defaultTextStyle),
            color: obj.color ?? defaultTextStyle.color,
            bold: obj.bold ?? defaultTextStyle.bold,
            italic: obj.italic ?? defaultTextStyle.italic,
            underline: convertUnderline(obj.underline),
            strike: convertStrike(obj.strike),
            highlight: obj.highlight,
            align: obj.textAlign,
            lineSpacingMultiple: obj.lineHeight,
            charSpacing:
              obj.letterSpacing !== undefined
                ? obj.letterSpacing * 100
                : undefined,
          },
        },
      };
    case "image": {
      const imageProps: ImageProps = {
        x: pxToIn(obj.x),
        y: pxToIn(obj.y),
        w: pxToIn(obj.w),
        h: pxToIn(obj.h),
      };
      // Detect whether src is a data URI or a file path
      if (obj.src.startsWith("data:")) {
        imageProps.data = obj.src;
      } else {
        imageProps.path = obj.src;
      }
      return { image: imageProps };
    }
    case "rect": {
      const opacityTransparency =
        obj.opacity !== undefined ? (1 - obj.opacity) * 100 : undefined;
      const explicitTransparency =
        obj.fill?.transparency !== undefined
          ? obj.fill.transparency * 100
          : undefined;
      const rectProps: Record<string, unknown> = {
        x: pxToIn(obj.x),
        y: pxToIn(obj.y),
        w: pxToIn(obj.w),
        h: pxToIn(obj.h),
        fill: obj.fill
          ? {
              color: obj.fill.color,
              transparency: explicitTransparency ?? opacityTransparency,
            }
          : opacityTransparency !== undefined
            ? { transparency: opacityTransparency }
            : undefined,
        line: obj.border
          ? {
              color: obj.border.color,
              width: obj.border.width,
              dashType: obj.border.dashType,
            }
          : undefined,
      };
      if (obj.borderRadius !== undefined) {
        rectProps.rectRadius = pxToIn(obj.borderRadius);
      }
      return { rect: rectProps };
    }
    case "line":
      return {
        line: {
          x: pxToIn(obj.x),
          y: pxToIn(obj.y),
          w: pxToIn(obj.w),
          h: pxToIn(obj.h),
          line: obj.line
            ? {
                color: obj.line.color,
                width: obj.line.width,
                dashType: obj.line.dashType,
              }
            : { color: "000000", width: 1 },
        },
      };
  }
}

type SlideRenderTarget = {
  addText: (
    text: string | Array<{ text: string; options?: Record<string, unknown> }>,
    options?: Record<string, unknown>,
  ) => unknown;
  addImage: (options: Record<string, unknown>) => unknown;
  addShape: (shapeName: string, options?: Record<string, unknown>) => unknown;
  addChart: (
    type: string,
    data: Array<Record<string, unknown>>,
    options?: Record<string, unknown>,
  ) => unknown;
  addTable: (rows: unknown, options?: Record<string, unknown>) => unknown;
  background?: Record<string, unknown>;
};

function getRootBackground(
  node: PositionedNode,
  buildContext: BuildContext,
): {
  rootBackgroundColor?: string;
  rootBackgroundImage?: Exclude<PositionedNode["backgroundImage"], undefined>;
  rootHasOpacity: boolean;
  background?: SlideMasterProps["background"];
} {
  const rootBackgroundColor =
    node.type !== "line" ? node.backgroundColor : undefined;
  const rootHasOpacity =
    node.type !== "line" && "opacity" in node && node.opacity !== undefined;
  const rootBackgroundImage =
    node.type !== "line" ? node.backgroundImage : undefined;

  let background: SlideMasterProps["background"] | undefined;
  if (rootBackgroundImage) {
    const cachedData = getImageData(
      rootBackgroundImage.src,
      buildContext.imageDataCache,
    );
    background = cachedData
      ? { data: cachedData }
      : { path: rootBackgroundImage.src };
  } else if (rootBackgroundColor && !rootHasOpacity) {
    background = { color: rootBackgroundColor };
  }

  return {
    rootBackgroundColor,
    rootBackgroundImage,
    rootHasOpacity,
    background,
  };
}

function renderPositionedTree(
  data: PositionedNode,
  ctx: RenderContext,
  applyRootBackground = true,
): void {
  const { rootBackgroundColor, rootBackgroundImage, rootHasOpacity } =
    getRootBackground(data, ctx.buildContext);

  // Synthetic group-id pool. `group="true"` is the canonical "auto"
  // form — each occurrence gets a fresh `auto-grp-N` id so two siblings
  // both writing it never accidentally merge. Any other string is the
  // author-chosen group id and is used verbatim.
  let nextAutoGroupId = 1;
  const resolveGroupId = (raw: string): string =>
    raw === "true" ? `auto-grp-${nextAutoGroupId++}` : raw;

  function renderNode(
    node: PositionedNode,
    groupStack: readonly string[],
    isRoot = false,
  ) {
    // Push this node's group ancestor (if any) so its own emitted
    // shapes — bg/border, hit area, leaf body — all carry the group
    // sigil, not just its descendants.
    const nextStack =
      typeof node.group === "string" && node.group.length > 0
        ? [...groupStack, resolveGroupId(node.group)]
        : groupStack;
    // Per-node ctx variant carries the current group stack so
    // wrapCtxWithObjectName + leaf renderers (notably Connector) can
    // read it without an extra parameter. We rebuild when nextStack
    // differs from whatever the outer ctx already carries — including
    // the case where an ancestor pushed a group but this node merely
    // inherits it.
    const groupedCtx: RenderContext =
      nextStack.length === (ctx.groupStack?.length ?? 0) &&
      nextStack.every((g, i) => g === ctx.groupStack?.[i])
        ? ctx
        : { ...ctx, groupStack: nextStack };
    // Wrap once so every shape this node emits — background, border, an
    // invisible click hit-area for empty containers, and the leaf body —
    // carries the node's `objectName=node#N`. The webview click delegate
    // resolves that id back to the source line, which is how clicking on a
    // VStack/HStack/Layer surface (or even an empty container hit-area)
    // navigates to its XML origin.
    const nodeCtx = wrapCtxWithObjectName(groupedCtx, node);
    // Tracks whether THIS node's render path emitted at least one PPTX
    // shape carrying its objectName. Used by the container branch to
    // decide whether the invisible hit-area is needed.
    let nodeSurfaceEmitted = false;

    if (node.type !== "line" && node.type !== "connector") {
      if (
        isRoot &&
        applyRootBackground &&
        (rootBackgroundImage || (rootBackgroundColor && !rootHasOpacity))
      ) {
        // Root node with a non-translucent background: bg goes to the
        // slide master (not a per-slide shape), so it carries no
        // objectName. Only the optional border survives as an addShape.
        const { border, borderRadius } = node;
        const hasBorder = Boolean(
          border &&
          (border.color !== undefined ||
            border.width !== undefined ||
            border.dashType !== undefined),
        );
        if (hasBorder) {
          const line = {
            color: border?.color ?? "000000",
            width:
              border?.width !== undefined ? pxToPt(border.width) : undefined,
            dashType: border?.dashType,
          };
          const shapeType = borderRadius
            ? ctx.pptx.ShapeType.roundRect
            : ctx.pptx.ShapeType.rect;
          const rectRadius = borderRadius
            ? Math.min((borderRadius / Math.min(node.w, node.h)) * 2, 1)
            : undefined;
          nodeCtx.slide.addShape(shapeType, {
            x: pxToIn(node.x),
            y: pxToIn(node.y),
            w: pxToIn(node.w),
            h: pxToIn(node.h),
            fill: { type: "none" },
            line,
            rectRadius,
          });
          nodeSurfaceEmitted = true;
        }
      } else if (!shouldEmbedBackgroundInText(node)) {
        renderBackgroundAndBorder(node, nodeCtx);
        if (shouldEmitContainerSurface(node)) {
          nodeSurfaceEmitted = true;
        }
      }
    }

    const def = getNodeDef(node.type);

    switch (def.category) {
      case "leaf":
        if (!def.render) {
          throw new Error(
            `No render function registered for leaf node: ${node.type}`,
          );
        }
        def.render(node, nodeCtx);
        break;

      case "multi-child":
      case "absolute-child": {
        const containerNode = node as Extract<
          PositionedNode,
          { type: "vstack" | "hstack" | "layer" }
        >;
        // Containers don't have a leaf render. When the bg/border path
        // above didn't emit an objectName-bearing shape — empty container,
        // or root container whose bg got delegated to the slide master —
        // clicking on the container's footprint would land on whatever
        // child shape sits under the cursor (or, for empty padding/gap
        // areas, on nothing). Add an invisible-but-hit-testable shape at
        // the bottom of the z-stack so the SVG renderer keeps a `<rect
        // data-object-name="node#N">` covering the full footprint.
        //
        // Why white@100%-transparency instead of `fill: none`: SVG `<rect
        // fill="none">` is hit-test transparent over its fill area (only
        // the stroke catches clicks), so the click would pass through to
        // whatever is behind the container. `fill: white; fill-opacity: 0`
        // keeps the rect pixel-clickable while staying visually invisible.
        if (
          node.__nodeId !== undefined &&
          !nodeSurfaceEmitted &&
          node.w > 0 &&
          node.h > 0
        ) {
          nodeCtx.slide.addShape(ctx.pptx.ShapeType.rect, {
            x: pxToIn(node.x),
            y: pxToIn(node.y),
            w: pxToIn(node.w),
            h: pxToIn(node.h),
            fill: { color: "FFFFFF", transparency: 100 },
            line: { type: "none" as const },
          });
        }
        for (const child of sortByZIndex(containerNode.children)) {
          renderNode(child, nextStack);
        }
        break;
      }
    }
  }

  renderNode(data, [], true);
}

/**
 * True when the container already emits a real shape (background colour,
 * border, image, shadow). In that case `renderBackgroundAndBorder` has
 * already produced a clickable PPTX object carrying the node's objectName,
 * so the invisible hit-area would just duplicate it.
 */
function shouldEmitContainerSurface(node: PositionedNode): boolean {
  if (node.type === "line") return false;
  if (node.backgroundColor) return true;
  if (node.backgroundImage) return true;
  if (
    node.border &&
    (node.border.color !== undefined ||
      node.border.width !== undefined ||
      node.border.dashType !== undefined)
  ) {
    return true;
  }
  if (node.shadow) return true;
  return false;
}

function collectMasterObjects(
  nodes: PositionedNode[],
  pptx: PptxGenJSInstance,
  buildContext: BuildContext,
  explicitBackground?: SlideMasterProps["background"],
): {
  objects: SlideMasterObject[];
  background?: SlideMasterProps["background"];
} {
  const objects: SlideMasterObject[] = [];
  let derivedBackground = explicitBackground;

  const collectorSlide: SlideRenderTarget = {
    addText(text, options = {}) {
      const flattenedText = Array.isArray(text)
        ? text.map((item) => item.text).join("")
        : text;
      objects.push({
        text: {
          text: flattenedText,
          options,
        },
      });
      return this;
    },
    addImage(options) {
      objects.push({ image: options });
      return this;
    },
    addShape(shapeName, options = {}) {
      const normalizedShapeName = String(shapeName);
      if (normalizedShapeName === "line") {
        objects.push({ line: options });
        return this;
      }
      if (normalizedShapeName === "rect") {
        objects.push({ rect: options });
        return this;
      }
      objects.push({
        text: {
          text: "",
          options: { ...options, shape: normalizedShapeName as ShapeName },
        },
      });
      return this;
    },
    addChart() {
      throw new Error("Chart nodes are not supported in <Master> yet");
    },
    addTable() {
      throw new Error("Table nodes are not supported in true slide masters");
    },
  };

  const ctx: RenderContext = {
    slide: collectorSlide as never,
    pptx,
    buildContext,
  };

  for (const node of nodes) {
    const { background } = getRootBackground(node, buildContext);
    const applyRootBackground = !derivedBackground && !!background;
    if (applyRootBackground) {
      derivedBackground = background;
    }
    renderPositionedTree(node, ctx, applyRootBackground);
  }

  return { objects, background: derivedBackground };
}

function toMasterBackgroundProps(
  background: SlideMasterOptions["background"] | SlideMasterProps["background"],
): SlideMasterProps["background"] | undefined {
  if (!background) return undefined;
  if ("color" in background) return { color: background.color };
  if ("path" in background) return { path: background.path };
  if ("data" in background) return { data: background.data };
  return background;
}

/**
 * Calls pptxgenjs defineSlideMaster from a SlideMasterOptions object.
 */
function defineSlideMasterFromOptions(
  pptx: PptxGenJSInstance,
  master: SlideMasterOptions,
  buildContext: BuildContext,
  renderedObjects?: SlideMasterObject[],
  backgroundOverride?: SlideMasterProps["background"],
): string {
  const masterName = master.title || DEFAULT_MASTER_NAME;

  const masterProps: SlideMasterProps = {
    title: masterName,
  };

  // Convert background
  masterProps.background = toMasterBackgroundProps(
    master.background ?? backgroundOverride,
  );

  // Convert margin (px -> inches)
  if (master.margin !== undefined) {
    if (typeof master.margin === "number") {
      masterProps.margin = pxToIn(master.margin);
    } else {
      masterProps.margin = [
        pxToIn(master.margin.top ?? 0),
        pxToIn(master.margin.right ?? 0),
        pxToIn(master.margin.bottom ?? 0),
        pxToIn(master.margin.left ?? 0),
      ];
    }
  }

  // Convert objects
  const masterObjects = master.objects?.map((obj) =>
    convertMasterObject(obj, buildContext),
  );
  const allObjects = [...(masterObjects ?? []), ...(renderedObjects ?? [])];
  if (allObjects.length > 0) {
    masterProps.objects = allObjects;
  }

  // Convert slideNumber
  if (master.slideNumber) {
    masterProps.slideNumber = {
      x: pxToIn(master.slideNumber.x),
      y: pxToIn(master.slideNumber.y),
      w: master.slideNumber.w ? pxToIn(master.slideNumber.w) : undefined,
      h: master.slideNumber.h ? pxToIn(master.slideNumber.h) : undefined,
      fontSize: master.slideNumber.fontSize
        ? pxToPt(master.slideNumber.fontSize)
        : buildContext.defaultTextStyle.fontSize !== undefined
          ? pxToPt(buildContext.defaultTextStyle.fontSize)
          : undefined,
      fontFace: resolveFontFamily(
        master.slideNumber.fontFamily,
        buildContext.defaultTextStyle,
      ),
      color: master.slideNumber.color ?? buildContext.defaultTextStyle.color,
      align: master.slideNumber.textAlign,
    };
  }

  pptx.defineSlideMaster(masterProps);
  return masterName;
}

/**
 * Converts PositionedNode trees into PptxGenJS slides.
 * @param pages Array of PositionedNode trees, one per slide
 * @param slidePx Full slide dimensions in px
 * @param masters Optional slide master options
 * @returns PptxGenJS instance
 */
export async function renderPptx(
  pages: PositionedNode[],
  slidePx: SlidePx,
  buildContext: BuildContext,
  masters?: SlideMasterOptions[],
  defaultMasterName?: string,
  masterContents?: Record<string, PositionedNode[]>,
  docProps?: {
    title?: string;
    author?: string;
    company?: string;
    subject?: string;
  },
) {
  const slideIn = { w: pxToIn(slidePx.w), h: pxToIn(slidePx.h) }; // Final conversion: layout (px) → PptxGenJS (inches)

  const PptxGenJS = await loadPptxGenJS();
  const pptx = new PptxGenJS();

  pptx.defineLayout({ name: "custom", width: slideIn.w, height: slideIn.h });
  pptx.layout = "custom";

  if (docProps) {
    if (docProps.title !== undefined) pptx.title = docProps.title;
    if (docProps.author !== undefined) pptx.author = docProps.author;
    if (docProps.company !== undefined) pptx.company = docProps.company;
    if (docProps.subject !== undefined) pptx.subject = docProps.subject;
  }

  if (masters) {
    for (const master of masters) {
      const contentNodes =
        master.title && masterContents
          ? masterContents[master.title]
          : undefined;
      const rendered = contentNodes
        ? collectMasterObjects(
            contentNodes,
            pptx,
            buildContext,
            toMasterBackgroundProps(master.background),
          )
        : undefined;
      defineSlideMasterFromOptions(
        pptx,
        master,
        buildContext,
        rendered?.objects,
        rendered?.background,
      );
    }
  }

  for (const data of pages) {
    const masterName = data.master ?? defaultMasterName;
    const slide = masterName ? pptx.addSlide({ masterName }) : pptx.addSlide();
    // Pre-walk this slide tree to gather author-id -> bbox so the
    // Connector renderer can resolve from / to without re-walking. The
    // index is rebuilt per slide; ids are slide-scoped.
    const idIndex = buildSlideIdIndex(data);
    const ctx: RenderContext = { slide, pptx, buildContext, idIndex };
    if (typeof data.notes === "string") {
      slide.addNotes(data.notes);
    }
    const rootBackground = getRootBackground(data, buildContext).background;
    if (rootBackground) {
      slide.background = rootBackground;
    }
    renderPositionedTree(data, ctx, true);
  }

  return pptx;
}

/**
 * Walk a positioned slide tree and collect `id -> bbox` entries. Skips
 * Connector nodes (no bbox) and resolves duplicates first-wins (the
 * parseXml pass already emitted a DUPLICATE_NODE_ID diagnostic in that
 * case).
 */
function buildSlideIdIndex(
  root: PositionedNode,
): Map<string, import("./types.ts").SlideBBox> {
  const map = new Map<string, import("./types.ts").SlideBBox>();
  const walk = (node: PositionedNode): void => {
    if (
      node.id !== undefined &&
      node.type !== "connector" &&
      !map.has(node.id)
    ) {
      const prst =
        node.type === "shape" ? node.shapeType : pseudoPrstFor(node.type);
      map.set(node.id, {
        x: node.x,
        y: node.y,
        w: node.w,
        h: node.h,
        prst,
      });
    }
    const children = (node as { children?: PositionedNode[] }).children;
    if (Array.isArray(children)) {
      for (const child of children) walk(child);
    }
  };
  walk(root);
  return map;
}

/**
 * Best-effort PPTX preset name for non-Shape positioned nodes. Used by
 * the post-process step to pick the correct stCxn/endCxn idx. Text /
 * Ul / Ol / Shape text bodies render as `<p:sp prstGeom prst="rect">`
 * in pptxgenjs, so rect is the right default. Image / Table / Chart /
 * Icon / Svg map to non-prstGeom carriers (`<p:pic>`, etc.) — return
 * undefined and let the post-process pass surface the unknown-idx
 * diagnostic and fall back to the default idx table.
 */
function pseudoPrstFor(type: PositionedNode["type"]): string | undefined {
  switch (type) {
    case "text":
    case "ul":
    case "ol":
      return "rect";
    case "line":
    case "connector":
      return undefined;
    default:
      return undefined;
  }
}
