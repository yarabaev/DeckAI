import type { PositionedNode } from "../../types.ts";
import type { RenderContext } from "../types.ts";
import type { ImageSrcGuardOptions } from "../../options.ts";
import type { DiagnosticCollector } from "../../diagnostics.ts";
import { pxToIn } from "../units.ts";
import { getContentArea } from "../utils/contentArea.ts";
import { renderObjectName } from "../utils/objectName.ts";

type ImagePositionedNode = Extract<PositionedNode, { type: "image" }>;

/**
 * Validates an image src against the imageSrcGuard policy.
 * Returns the src if allowed, or undefined if disallowed (emits diagnostic).
 * Pass `silent: true` to suppress diagnostic emission (e.g. render-time
 * defense-in-depth after prefetch already emitted the diagnostic).
 */
function hasSchemeLike(src: string): boolean {
  // Detect "scheme:" appearing before the first "/" — distinguishes URLs from paths.
  const colonIdx = src.indexOf(":");
  const slashIdx = src.indexOf("/");
  return colonIdx >= 0 && (slashIdx === -1 || colonIdx < slashIdx);
}

export function validateImageSrc(
  src: string,
  guard: ImageSrcGuardOptions,
  diagnostics: DiagnosticCollector,
  options?: { silent?: boolean },
): string | undefined {
  const silent = options?.silent ?? false;
  const { allowSchemes } = guard;
  const srcIsUrl = hasSchemeLike(src);

  // Step 1: scheme validation (when allowSchemes is configured)
  if (allowSchemes && allowSchemes.length > 0) {
    if (srcIsUrl) {
      const colonIdx = src.indexOf(":");
      const scheme = src.slice(0, colonIdx + 1).toLowerCase();
      if (!allowSchemes.includes(scheme)) {
        if (!silent)
          diagnostics.add(
            "INVALID_IMAGE_SRC",
            `<Image src="${src}">: scheme "${scheme}" is not in the allowed list`,
          );
        return undefined;
      }
    } else {
      // Relative/bare path — block when allowSchemes only lists URL schemes
      const hasSchemeOnly = allowSchemes.every((s) => s.endsWith(":"));
      if (hasSchemeOnly) {
        if (!silent)
          diagnostics.add(
            "INVALID_IMAGE_SRC",
            `<Image src="${src}">: relative/bare paths are not allowed when allowSchemes is set`,
          );
        return undefined;
      }
    }
  }

  // Step 2: path containment (when allowBaseDir is configured)
  if (guard.allowBaseDir) {
    // URL-schemed srcs are incompatible with allowBaseDir — reject regardless of
    // whether allowSchemes also passed (HIGH-1 fix: file:///etc/passwd bypass;
    // HIGH-2 fix: https://evil.com SSRF via path concatenation).
    if (srcIsUrl) {
      if (!silent)
        diagnostics.add(
          "INVALID_IMAGE_SRC",
          `<Image src="${src}">: URL-schemed src is not allowed when allowBaseDir is configured`,
        );
      return undefined;
    }

    const resolved = src.startsWith("/") ? src : `${guard.allowBaseDir}/${src}`;
    // Normalise to detect traversal
    const normalised = resolved
      .split("/")
      .reduce<string[]>((acc, part) => {
        if (part === "..") acc.pop();
        else if (part !== ".") acc.push(part);
        return acc;
      }, [])
      .join("/");
    const base = guard.allowBaseDir.endsWith("/")
      ? guard.allowBaseDir
      : guard.allowBaseDir + "/";
    if (!normalised.startsWith(base) && normalised !== guard.allowBaseDir) {
      if (!silent)
        diagnostics.add(
          "INVALID_IMAGE_SRC",
          `<Image src="${src}">: path escapes the allowed base directory`,
        );
      return undefined;
    }
  }

  return src;
}

export function renderImageNode(
  node: ImagePositionedNode,
  ctx: RenderContext,
): void {
  const content = getContentArea(node);
  const objectName = renderObjectName(node, ctx);
  const imageOptions: Record<string, unknown> = {
    x: pxToIn(content.x),
    y: pxToIn(content.y),
    w: pxToIn(content.w),
    h: pxToIn(content.h),
    ...(objectName ? { objectName } : {}),
    ...(node.isDecorative
      ? { altText: "" }
      : node.altText !== undefined
        ? { altText: node.altText }
        : {}),
    ...(node.rotate !== undefined ? { rotate: node.rotate } : {}),
    shadow: node.shadow
      ? {
          type: node.shadow.type ?? ("outer" as const),
          opacity: node.shadow.opacity,
          blur: node.shadow.blur,
          angle: node.shadow.angle,
          offset: node.shadow.offset,
          color: node.shadow.color,
        }
      : undefined,
  };

  if (node.sizing) {
    imageOptions.sizing = {
      type: node.sizing.type,
      w: pxToIn(node.sizing.w ?? content.w),
      h: pxToIn(node.sizing.h ?? content.h),
      ...(node.sizing.x !== undefined && { x: pxToIn(node.sizing.x) }),
      ...(node.sizing.y !== undefined && { y: pxToIn(node.sizing.y) }),
    };
  }

  const guard = ctx.buildContext.security.imageSrcGuard;

  if (node.imageData) {
    // Base64 data — skip src validation (data is already embedded)
    ctx.slide.addImage({ ...imageOptions, data: node.imageData });
  } else {
    // silent: prefetch already emitted the diagnostic for blocked srcs
    const src = guard
      ? validateImageSrc(node.src, guard, ctx.buildContext.diagnostics, {
          silent: true,
        })
      : node.src;
    if (src !== undefined) {
      ctx.slide.addImage({ ...imageOptions, path: src });
    }
  }
}
