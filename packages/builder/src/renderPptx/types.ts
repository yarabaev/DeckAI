import type { BuildContext } from "../buildContext.ts";

type PptxGenJSClass = import("pptxgenjs").default;
export type SlideInstance = ReturnType<PptxGenJSClass["addSlide"]>;
export type PptxInstance = PptxGenJSClass;

/**
 * Bounding box for nodes that own a positioned region on the current
 * slide. Used by the Connector renderer to anchor onto another node by
 * its author-facing `id`.
 */
export type SlideBBox = {
  x: number;
  y: number;
  w: number;
  h: number;
  /**
   * The PPTX preset name of this shape (e.g. "rect", "ellipse"), or
   * undefined for nodes that are not preset shapes (text, image, ...).
   * Carried so the post-process step can pick the correct stCxn/endCxn
   * `idx` from the shape-type table.
   */
  prst?: string;
};

export type RenderContext = {
  slide: SlideInstance;
  pptx: PptxInstance;
  buildContext: BuildContext;
  /**
   * Author-id -> bounding box, populated by the slide renderer before
   * descending into the positioned tree so that Connector nodes can look
   * up their from/to endpoints. Reset per slide.
   */
  idIndex?: Map<string, SlideBBox>;
  /**
   * Active group ancestors, outermost first. The slide renderer pushes
   * a group id every time it descends into a node carrying `group=...`
   * and pops on exit; leaf renderers read this so every emitted shape
   * carries `sg-grp:G` tokens for every ancestor group, which the
   * post-process `<p:grpSp>` rewriter consumes.
   */
  groupStack?: readonly string[];
};
