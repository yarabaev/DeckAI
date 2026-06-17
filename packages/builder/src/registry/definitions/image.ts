import type { BuilderNode } from "../../types.ts";
import type { NodeDefinition, Yoga } from "../types.ts";
import type { Node as YogaNode } from "yoga-layout";
import { measureImage, getImageData } from "../../shared/measureImage.ts";
import type { BuildContext } from "../../buildContext.ts";
import {
  renderImageNode,
  validateImageSrc,
} from "../../renderPptx/nodes/image.ts";

export const imageNodeDef: NodeDefinition = {
  type: "image",
  category: "leaf",
  applyYogaStyle(
    node: BuilderNode,
    yn: YogaNode,
    _yoga: Yoga,
    ctx: BuildContext,
  ) {
    const n = node as Extract<BuilderNode, { type: "image" }>;
    const src = n.src;
    const guard = ctx.security.imageSrcGuard;

    if (
      guard &&
      validateImageSrc(src, guard, ctx.diagnostics, { silent: true }) ===
        undefined
    ) {
      // Guard blocks this src — register no-op measure to prevent unguarded fs.readFileSync
      yn.setMeasureFunc(() => ({ width: 100, height: 100 }));
      return;
    }

    yn.setMeasureFunc(() => {
      const { widthPx, heightPx } = measureImage(
        src,
        ctx.imageSizeCache,
        ctx.diagnostics,
      );
      return { width: widthPx, height: heightPx };
    });
  },
  toPositioned(pom, absoluteX, absoluteY, layout, ctx) {
    const n = pom as Extract<BuilderNode, { type: "image" }>;
    const imageData = getImageData(n.src, ctx.imageDataCache);
    return {
      ...n,
      x: absoluteX,
      y: absoluteY,
      w: layout.width,
      h: layout.height,
      imageData,
    };
  },
  render(node, ctx) {
    renderImageNode(node as Extract<typeof node, { type: "image" }>, ctx);
  },
  collectImageSources(node) {
    const n = node as Extract<BuilderNode, { type: "image" }>;
    return [n.src];
  },
};
