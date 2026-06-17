/**
 * Sigil channel used by the post-process passes to smuggle metadata
 * through pptxgenjs output and back to the rewriter via
 * `<p:cNvPr name="...">`. pptxgenjs has no notion of connectors or
 * groups, so we encode all post-process intent into the only stable
 * per-shape string slot the library exposes.
 *
 * Format: tokens joined by `|`. Each recognised token starts with one
 * of the sg-* prefixes below; tokens we don't own (notably `node#N`,
 * read by @slideglance/core for source-map jumps) pass through
 * untouched.
 *
 *   sg-id:USER_ID
 *     Author-facing `id` attribute, added by every node that opts in.
 *     The connector rewriter uses it to resolve `<Connector from="...">`
 *     to the OOXML spId.
 *
 *   sg-cxn:FROM#FS>TO#TS:K:P
 *     Marker on the placeholder line a `<Connector>` emits. FS/TS are
 *     `top|right|bottom|left`. K is the author kind (straight/elbow/
 *     curved). P is the picked PPTX preset name.
 *
 *   sg-grp:GROUP_ID
 *     "this shape belongs to GROUP_ID". Appears once per group on
 *     every leaf rendered inside a `group="..."` ancestor; nested
 *     groups stack as additional sg-grp tokens, outermost first.
 *
 * Tokens are stripped after rewriting so the final PPTX shows nothing
 * unusual to PowerPoint or accessibility tools. The `node#N` token, if
 * present, survives the strip — that contract is owned by the SVG
 * renderer side, not the builder.
 */

export const SG_ID_PREFIX = "sg-id:";
export const SG_CXN_PREFIX = "sg-cxn:";
export const SG_GRP_PREFIX = "sg-grp:";
export const TOKEN_DELIM = "|";

const SG_PREFIXES = [SG_ID_PREFIX, SG_CXN_PREFIX, SG_GRP_PREFIX];

function splitTokens(name: string | undefined): string[] {
  if (!name) return [];
  return name.split(TOKEN_DELIM).filter((t) => t.length > 0);
}

function findToken(
  tokens: readonly string[],
  prefix: string,
): string | undefined {
  return tokens.find((t) => t.startsWith(prefix));
}

function findAllTokens(tokens: readonly string[], prefix: string): string[] {
  return tokens.filter((t) => t.startsWith(prefix));
}

function isSgToken(token: string): boolean {
  return SG_PREFIXES.some((p) => token.startsWith(p));
}

/**
 * Join an arbitrary list of tokens into a cNvPr@name value. Filters
 * empties so callers can append optional pieces without guarding.
 */
export function buildObjectName(
  tokens: readonly (string | undefined)[],
): string {
  return tokens
    .filter((t): t is string => Boolean(t && t.length > 0))
    .join(TOKEN_DELIM);
}

export interface ParsedIdSigil {
  userId: string;
}

/**
 * Read the author id from a sg-id token. Returns null when no sg-id
 * token is present. The userId payload is the entire remainder of the
 * token after the prefix; the id regex enforced at parse time
 * guarantees no `|` characters.
 */
export function parseIdSigil(name: string | undefined): ParsedIdSigil | null {
  const tokens = splitTokens(name);
  const tok = findToken(tokens, SG_ID_PREFIX);
  if (!tok) return null;
  const userId = tok.slice(SG_ID_PREFIX.length);
  if (!userId) return null;
  return { userId };
}

export interface ParsedCxnSigil {
  from: string;
  fromSide: "top" | "right" | "bottom" | "left";
  to: string;
  toSide: "top" | "right" | "bottom" | "left";
  kind: "straight" | "elbow" | "curved";
  preset: string;
}

const SIDES = new Set(["top", "right", "bottom", "left"]);
const KINDS = new Set(["straight", "elbow", "curved"]);

/**
 * Parse a `sg-cxn:FROM#FS>TO#TS:K:P` token. Returns null on any
 * structural mismatch so callers can leave the unrecognised cNvPr in
 * place.
 */
export function parseCxnSigil(name: string | undefined): ParsedCxnSigil | null {
  const tokens = splitTokens(name);
  const tok = findToken(tokens, SG_CXN_PREFIX);
  if (!tok) return null;
  const body = tok.slice(SG_CXN_PREFIX.length);
  const arrowIdx = body.indexOf(">");
  if (arrowIdx === -1) return null;
  const left = body.slice(0, arrowIdx);
  const rightAndTail = body.slice(arrowIdx + 1);
  const leftHash = left.indexOf("#");
  if (leftHash === -1) return null;
  const from = left.slice(0, leftHash);
  const fromSide = left.slice(leftHash + 1);
  if (!SIDES.has(fromSide)) return null;

  const rightHash = rightAndTail.indexOf("#");
  if (rightHash === -1) return null;
  const to = rightAndTail.slice(0, rightHash);
  const tail = rightAndTail.slice(rightHash + 1);
  const tailParts = tail.split(":");
  if (tailParts.length !== 3) return null;
  const [toSide, kind, preset] = tailParts;
  if (!toSide || !kind || !preset) return null;
  if (!SIDES.has(toSide)) return null;
  if (!KINDS.has(kind)) return null;

  return {
    from,
    fromSide: fromSide as ParsedCxnSigil["fromSide"],
    to,
    toSide: toSide as ParsedCxnSigil["toSide"],
    kind: kind as ParsedCxnSigil["kind"],
    preset,
  };
}

/**
 * Extract every sg-grp token payload, preserving order. The renderer
 * pushes outermost groups first, so the resulting array is in
 * outer -> inner order — useful for the rewriter which needs to wrap
 * the innermost group first.
 */
export function parseGrpSigils(name: string | undefined): string[] {
  const tokens = splitTokens(name);
  return findAllTokens(tokens, SG_GRP_PREFIX).map((t) =>
    t.slice(SG_GRP_PREFIX.length),
  );
}

/**
 * Remove every sg-* token, returning the residual joined by the same
 * delimiter. Used after every rewriter has consumed its markers so the
 * final cNvPr@name carries only contract-bearing tokens (currently
 * just `node#N` for the SVG-side source-map jump). Empty result means
 * the caller should delete the attribute entirely.
 */
export function stripSigils(name: string | undefined): string {
  const tokens = splitTokens(name);
  const kept = tokens.filter((t) => !isSgToken(t));
  return kept.join(TOKEN_DELIM);
}

/**
 * Remove only the tokens with the given prefixes, leaving every other
 * token (including unrelated sg-* tokens) untouched. Used by a single
 * rewriter pass that wants to clear its own markers while preserving
 * markers a later pass still needs to read.
 */
export function stripSigilsByPrefix(
  name: string | undefined,
  prefixes: readonly string[],
): string {
  const tokens = splitTokens(name);
  const kept = tokens.filter((t) => !prefixes.some((p) => t.startsWith(p)));
  return kept.join(TOKEN_DELIM);
}
