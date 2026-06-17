import type { RenderContext } from "../types.ts";

/**
 * Validates an href URL scheme against the allowlist.
 * Returns the href if allowed, or undefined if disallowed (emits INVALID_HREF_SCHEME diagnostic).
 */
export function validateHref(
  href: string,
  allowedSchemes: ReadonlySet<string>,
  ctx: RenderContext,
): string | undefined {
  const colonIdx = href.indexOf(":");
  const scheme = colonIdx >= 0 ? href.slice(0, colonIdx + 1).toLowerCase() : "";
  if (!scheme || !allowedSchemes.has(scheme)) {
    ctx.buildContext.diagnostics.add(
      "INVALID_HREF_SCHEME",
      `<A href="${href}">: scheme "${scheme || "(none)"}" is not in the allowed list`,
    );
    return undefined;
  }
  return href;
}
