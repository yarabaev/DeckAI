/**
 * PPTX preset shape -> 4-side connection-site idx map.
 *
 * Each entry maps the four cardinal sides (top / right / bottom /
 * left) to the `idx` value that goes into `<a:stCxn idx="..."/>` or
 * `<a:endCxn idx="..."/>`. The mapping is taken from the OOXML preset
 * shape `cxnLst` definitions (see ECMA-376 Part 1, §20.1.9 and the
 * generated drawingml/preset-shapes data in OOXML SDK).
 *
 * For most rectangular-ish presets PowerPoint uses the same order
 * (top, right, bottom, left -> 0, 1, 2, 3). A handful of shapes use a
 * different cxnLst (e.g. line/arc/connector presets themselves), but
 * we never look those up as connector endpoints — only Shape / Text
 * / Ul / Ol bodies that render as a shape with a 4-corner bbox can be
 * endpoints in our model.
 *
 * Shapes not listed here fall back to DEFAULT_SIDE_IDX. The caller is
 * expected to emit a CONNECTOR_UNKNOWN_SHAPE_IDX info diagnostic when
 * that happens.
 */

export type Side = "top" | "right" | "bottom" | "left";

export type SideIdxMap = Record<Side, number>;

/** Standard 4-side idx for the great majority of preset shapes. */
export const DEFAULT_SIDE_IDX: SideIdxMap = {
  top: 0,
  right: 1,
  bottom: 2,
  left: 3,
};

/**
 * Preset shapes whose cxnLst follows the default cardinal order. The
 * vast majority — listed explicitly for clarity rather than for
 * differentiation so future audits can spot which shapes have been
 * verified.
 */
const STANDARD_4_SIDE = new Set<string>([
  // Basic rectangles / boxes
  "rect",
  "roundRect",
  "snip1Rect",
  "snip2SameRect",
  "snip2DiagRect",
  "snipRoundRect",
  "round1Rect",
  "round2SameRect",
  "round2DiagRect",
  // Circles / ovals
  "ellipse",
  "donut",
  "noSmoking",
  // Polygons
  "diamond",
  "triangle",
  "rtTriangle",
  "parallelogram",
  "trapezoid",
  "pentagon",
  "hexagon",
  "heptagon",
  "octagon",
  "decagon",
  "dodecagon",
  // Stars
  "star4",
  "star5",
  "star6",
  "star7",
  "star8",
  "star10",
  "star12",
  "star16",
  "star24",
  "star32",
  // Arrows (the wedge-style ones)
  "rightArrow",
  "leftArrow",
  "upArrow",
  "downArrow",
  "leftRightArrow",
  "upDownArrow",
  // Misc box-shaped presets
  "plaque",
  "can",
  "cube",
  "bevel",
  "frame",
  "halfFrame",
  "cloud",
  "heart",
  "lightningBolt",
  "sun",
  "moon",
  "smileyFace",
  "irregularSeal1",
  "irregularSeal2",
]);

export function lookupSideIdx(prst: string | undefined): SideIdxMap {
  if (prst && STANDARD_4_SIDE.has(prst)) return DEFAULT_SIDE_IDX;
  return DEFAULT_SIDE_IDX;
}

/**
 * Returns true when the prst is known to follow the default 4-side
 * idx layout. Callers can branch on this to surface an info-level
 * diagnostic when binding to an untabulated shape.
 */
export function isKnownPrstForCxn(prst: string | undefined): boolean {
  return prst !== undefined && STANDARD_4_SIDE.has(prst);
}
