import type { RenderContext } from "../types.ts";
import {
  SG_GRP_PREFIX,
  SG_ID_PREFIX,
  buildObjectName,
} from "../postProcess/sigils.ts";

/**
 * Build the pptxgenjs `objectName` string for a BuilderNode. The output
 * is serialised into OOXML `<p:cNvPr name="...">` and read back by two
 * orthogonal consumers:
 *
 *   1. The connector / group post-process passes inside the builder
 *      itself — they look for `sg-id:USER_ID`, `sg-grp:GROUP_ID`, and
 *      `sg-cxn:...` tokens to rewrite placeholder shapes into real
 *      `<p:cxnSp>` / `<p:grpSp>` elements before the bytes leave
 *      pptx.write().
 *
 *   2. The SVG renderer (e.g. @slideglance/core) — it extracts the
 *      `node#N` token to emit `data-node-id="N"` on rendered SVG so
 *      webviews can map a click back to the originating source line.
 *
 * Tokens are joined with `|`; consumers ignore tokens they do not
 * own. Returns undefined when the node carries nothing worth marking,
 * in which case callers should omit the option entirely so pptxgenjs
 * lets the default `cNvPr name="Object N"` ship.
 */
export function builderObjectName(node: {
  __nodeId?: number;
  id?: string;
  /**
   * Group memberships, outermost first. Each entry becomes one
   * `sg-grp:G` token. The renderer accumulates this list while
   * walking ancestor `group="..."` containers; node definitions
   * never set it directly.
   */
  groupIds?: readonly string[];
}): string | undefined {
  const tokens: (string | undefined)[] = [];
  if (node.id) tokens.push(`${SG_ID_PREFIX}${node.id}`);
  if (node.groupIds) {
    for (const g of node.groupIds) tokens.push(`${SG_GRP_PREFIX}${g}`);
  }
  if (node.__nodeId !== undefined) tokens.push(`node#${node.__nodeId}`);
  const joined = buildObjectName(tokens);
  return joined.length > 0 ? joined : undefined;
}

/**
 * Render-time convenience: builds the objectName from a positioned
 * node + its render context, automatically threading the current
 * group ancestor stack. Every individual node renderer should call
 * this rather than `builderObjectName(node)` directly so the
 * `sg-grp:` tokens reach the post-process pass even on shapes whose
 * renderer sets `objectName` explicitly (which bypasses the proxy
 * injection done by `wrapCtxWithObjectName`).
 */
export function renderObjectName(
  node: { __nodeId?: number; id?: string },
  ctx: RenderContext,
): string | undefined {
  return builderObjectName({
    __nodeId: node.__nodeId,
    id: node.id,
    groupIds: ctx.groupStack,
  });
}
