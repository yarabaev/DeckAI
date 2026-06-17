/**
 * Lint rule registry. Each rule is pure, runs at a single phase, emits
 * Diagnostics. The set is curated — every code listed here corresponds
 * to a real failure mode observed in slideglance/builder use.
 *
 * Adding a new rule? Append to ALL_RULES at the bottom of this file.
 */

import type { Diagnostic, DiagnosticCode } from "../../diagnostics.ts";
import type { LintRule, LintContext } from "../types.ts";
import type { PositionedNode } from "../../types.ts";
import {
  measureText,
  TEXT_MEASURE_WIDTH_SAFETY_MARGIN_PX,
} from "../../calcYogaLayout/measureText.ts";
import {
  measureTextWidth as measureTextWidthRaw,
  pickBundledFontForText,
} from "../../calcYogaLayout/fontLoader.ts";
import {
  getChildren,
  isTextBearing,
  nodeIdOf,
  nodeSourcePos,
  walk,
} from "../walker.ts";
import { scanRawXmlForBareLtGt } from "./rawXml.ts";

const TINY_FONT_PT_THRESHOLD = 8;
const LARGE_IMAGE_BYTES_THRESHOLD = 1_000_000; // 1 MB
const EXCESS_NODES_PER_SLIDE = 200;
const MAX_FONTS_PER_SLIDE = 5;
// Sub-pixel overflow from yoga text measurement is common — only flag
// overshoots greater than this to avoid measurement-noise false positives.
const OVERFLOW_TOLERANCE_PX = 3;
// Both TEXT_OVERFLOW_* rules re-measure with the same opentype path the
// layout pass uses (`mode="opentype"` + the build's TextMeasurer). When
// the re-measurement disagrees with the positioned bbox by less than
// this factor of the natural line height, treat it as measurement noise
// rather than a genuine overflow.
const TEXT_OVERFLOW_FACTOR = 1.2;

function diag(
  code: DiagnosticCode,
  severity: "error" | "warn" | "info",
  ctx: LintContext,
  node: PositionedNode,
  path: readonly PositionedNode[],
  message: string,
  extra: Partial<Diagnostic> = {},
): Diagnostic {
  const sp = nodeSourcePos(node);
  return {
    code,
    severity,
    message: `Slide ${ctx.slideIndex}: ${message}`,
    nodeId: nodeIdOf(node, path),
    nodeType: node.type,
    ...(sp ? { sourcePos: sp } : {}),
    ...extra,
  };
}

// ============================================================================
// PHASE 0 — raw-XML parse-phase rules (pre-positioning, source-level)
// ============================================================================

/**
 * Detects bare `<` / `>` inside XML attribute values. fast-xml-parser is
 * lenient here so the violation would otherwise slip through unnoticed
 * and produce a document strict consumers (PowerPoint, XSD validators)
 * reject. Operates on the original source strings — the BuilderNode
 * tree no longer has the unescaped characters by parse-time.
 */
const rawLtGtInAttr: LintRule = {
  code: "RAW_LT_GT_IN_ATTR",
  severity: "warn",
  phase: "parse",
  check(ctx) {
    if (!ctx.rawXmlSources || ctx.rawXmlSources.length === 0) return [];
    const out: Diagnostic[] = [];
    for (const src of ctx.rawXmlSources) {
      out.push(...scanRawXmlForBareLtGt(src.content, src.path));
    }
    return out;
  },
};

// ============================================================================
// PHASE A — overflow / dimension / wrap rules (post-layout)
// ============================================================================

const outOfPage: LintRule = {
  code: "OUT_OF_PAGE",
  severity: "error",
  phase: "post-layout",
  check(ctx) {
    const out: Diagnostic[] = [];
    for (const { node, path } of walk(ctx.tree)) {
      // Skip the slide root itself.
      if (path.length === 0) continue;
      const overshootX = node.x + node.w - ctx.slideSize.w;
      const overshootY = node.y + node.h - ctx.slideSize.h;
      const underX = -node.x;
      const underY = -node.y;
      const max = Math.max(overshootX, overshootY, underX, underY);
      if (max > OVERFLOW_TOLERANCE_PX) {
        out.push(
          diag(
            "OUT_OF_PAGE",
            "error",
            ctx,
            node,
            path,
            `<${node.type}> spills past the slide canvas by ${Math.round(max)}px ` +
              `(node at ${node.x.toFixed(0)},${node.y.toFixed(0)} size ${node.w.toFixed(0)}x${node.h.toFixed(0)}; ` +
              `slide is ${ctx.slideSize.w}x${ctx.slideSize.h}).`,
            {
              context: {
                node: { x: node.x, y: node.y, w: node.w, h: node.h },
                slide: ctx.slideSize,
                overshoot: { x: overshootX, y: overshootY, underX, underY },
              },
            },
          ),
        );
      }
    }
    return out;
  },
};

const outOfParent: LintRule = {
  code: "OUT_OF_PARENT",
  severity: "error",
  phase: "post-layout",
  check(ctx) {
    const out: Diagnostic[] = [];
    for (const { node, parent, path } of walk(ctx.tree)) {
      if (!parent) continue;
      // Layer's job is absolute positioning; overflow is a feature, not a bug.
      if (parent.type === "layer") continue;
      const rightSpill = node.x + node.w - (parent.x + parent.w);
      const bottomSpill = node.y + node.h - (parent.y + parent.h);
      const max = Math.max(rightSpill, bottomSpill);
      if (max > OVERFLOW_TOLERANCE_PX) {
        out.push(
          diag(
            "OUT_OF_PARENT",
            "error",
            ctx,
            node,
            path,
            `<${node.type}> overflows its ${parent.type} parent by ${Math.round(max)}px. ` +
              `Either widen the parent, shrink the child (lineHeight/fontSize), or wrap with Layer if absolute positioning is intentional.`,
            {
              context: {
                parent: { type: parent.type, w: parent.w, h: parent.h },
                child: { x: node.x, y: node.y, w: node.w, h: node.h },
                overshoot: { right: rightSpill, bottom: bottomSpill },
              },
            },
          ),
        );
      }
    }
    return out;
  },
};

const zeroDim: LintRule = {
  code: "ZERO_DIM",
  severity: "warn",
  phase: "post-layout",
  check(ctx) {
    const out: Diagnostic[] = [];
    for (const { node, path } of walk(ctx.tree)) {
      if (path.length === 0) continue;
      // Lines, by construction, have w=0 (vertical) or h=0 (horizontal):
      // the layout box only constrains where the line draws its endpoints.
      // Skip them so the rule only fires on genuinely-collapsed boxes.
      if (node.type === "line") continue;
      // Connectors carry a placeholder 0x0 bbox; their real geometry is
      // resolved render-side from the from/to shapes' bboxes, so a
      // zero-size connector is the expected steady state, not a layout
      // collapse worth warning about.
      if (node.type === "connector") continue;
      if (node.w === 0 || node.h === 0) {
        out.push(
          diag(
            "ZERO_DIM",
            "warn",
            ctx,
            node,
            path,
            `<${node.type}> resolved to ${node.w}x${node.h} — it renders but is invisible. ` +
              `Usually a sibling claimed all the slack via w="max". ` +
              `Set an explicit w/h, or check that the parent has room.`,
            {
              context: { w: node.w, h: node.h },
            },
          ),
        );
      }
    }
    return out;
  },
};

const negativeDim: LintRule = {
  code: "NEGATIVE_DIM",
  severity: "error",
  phase: "post-layout",
  check(ctx) {
    const out: Diagnostic[] = [];
    for (const { node, path } of walk(ctx.tree)) {
      if (node.w < 0 || node.h < 0) {
        out.push(
          diag(
            "NEGATIVE_DIM",
            "error",
            ctx,
            node,
            path,
            `<${node.type}> resolved to negative dimensions ${node.w}x${node.h}. ` +
              `Usually padding > parent size. Reduce padding or widen the parent.`,
            { context: { w: node.w, h: node.h } },
          ),
        );
      }
    }
    return out;
  },
};

/**
 * Re-measure a Text node with the same opentype path the layout pass
 * uses. Returns `null` for non-Text nodes, empty text, or when the
 * measurer throws (rules must never break the build).
 */
function reMeasure(
  ctx: LintContext,
  node: PositionedNode,
  maxWidthPx: number,
): {
  widthPx: number;
  heightPx: number;
  lineHeight: number;
  fontSize: number;
} | null {
  if (node.type !== "text") return null;
  const text = (node as { text?: string }).text ?? "";
  if (text.length === 0) return null;
  const fontSize = (node as { fontSize?: number }).fontSize ?? 13;
  const lineHeight = (node as { lineHeight?: number }).lineHeight ?? 1.0;
  const fontFamily = (node as { fontFamily?: string }).fontFamily ?? "Inter";
  const nodeBold = !!(node as { bold?: boolean }).bold;
  const hasBoldRun =
    (node as { runs?: { bold?: boolean }[] }).runs?.some((r) => r.bold) ??
    false;
  const fontWeight: "bold" | "normal" =
    nodeBold || hasBoldRun ? "bold" : "normal";
  try {
    const r = measureText(
      text,
      maxWidthPx,
      {
        fontFamily,
        fontSizePx: fontSize,
        lineHeight,
        fontWeight,
        measurer: ctx.measurer,
      },
      "opentype",
    );
    return { ...r, fontSize, lineHeight };
  } catch {
    return null;
  }
}

const textOverflowV: LintRule = {
  code: "TEXT_OVERFLOW_V",
  severity: "warn",
  phase: "post-layout",
  check(ctx) {
    const out: Diagnostic[] = [];
    for (const { node, path } of walk(ctx.tree)) {
      // `noWrap=true` is an explicit author opt-out from wrapping. The
      // text intentionally stays on a single line and lets glyphs spill
      // past the box. Re-measuring at the constrained width then
      // counting "lines needed" is meaningless — the text never wraps —
      // so the rule must respect the opt-out and skip these nodes.
      if ((node as { noWrap?: boolean }).noWrap === true) continue;
      const r = reMeasure(ctx, node, node.w);
      if (!r) continue;
      // Convert the re-measured pixel height back into a line count so
      // the user-facing message stays in the original "N lines" idiom.
      const estLines = Math.max(
        1,
        Math.round(r.heightPx / (r.fontSize * r.lineHeight)),
      );
      if (r.heightPx > node.h * TEXT_OVERFLOW_FACTOR) {
        out.push(
          diag(
            "TEXT_OVERFLOW_V",
            "warn",
            ctx,
            node,
            path,
            `<Text> opentype re-measure ${estLines} lines need ~${Math.round(r.heightPx)}px but ` +
              `the box is only ${Math.round(node.h)}px. Lower fontSize, raise the box, or use autoFit. ` +
              `If the layout used "fallback" mode but the renderer uses opentype, this gap is the cause.`,
            {
              context: {
                estLines,
                neededH: r.heightPx,
                boxH: node.h,
                fontSize: r.fontSize,
                lineHeight: r.lineHeight,
              },
            },
          ),
        );
      }
    }
    return out;
  },
};

const textOverflowH: LintRule = {
  code: "TEXT_OVERFLOW_H",
  severity: "warn",
  phase: "post-layout",
  check(ctx) {
    const out: Diagnostic[] = [];
    for (const { node, path } of walk(ctx.tree)) {
      // `noWrap=true` is an explicit author opt-out: the text is
      // expected to spill past the right edge by design (e.g. cover-
      // page meta strips that stay on a single line regardless of how
      // wide the strip's enclosing flex container is). The rule's
      // "natural width exceeds box width" diagnosis is exactly that
      // intended behaviour, so skip noWrap nodes to avoid a stream of
      // false positives on legitimate authoring.
      if ((node as { noWrap?: boolean }).noWrap === true) continue;
      // Measure the natural (un-wrapped) single-line width.
      const natural = reMeasure(ctx, node, Number.POSITIVE_INFINITY);
      if (!natural) continue;
      // Only fire when the box height *forces* a single line and the
      // natural width still overshoots that line. Multi-line boxes are
      // covered by TEXT_OVERFLOW_V.
      const singleLineH = natural.fontSize * natural.lineHeight;
      const boxFitsOneLine = node.h < singleLineH * 1.6;
      if (!boxFitsOneLine) continue;
      const overshootPx = natural.widthPx - node.w;
      const effectiveTolerancePx =
        OVERFLOW_TOLERANCE_PX + TEXT_MEASURE_WIDTH_SAFETY_MARGIN_PX;
      if (overshootPx > effectiveTolerancePx) {
        out.push(
          diag(
            "TEXT_OVERFLOW_H",
            "warn",
            ctx,
            node,
            path,
            `<Text> natural single-line width ~${Math.round(natural.widthPx)}px exceeds box width ${Math.round(node.w)}px by ${Math.round(overshootPx)}px. ` +
              `The box height (${Math.round(node.h)}px) only fits one line, so the glyphs will spill past the right edge. ` +
              `Widen the box, lower fontSize, or allow wrapping by raising the box height.`,
            {
              context: {
                naturalW: natural.widthPx,
                boxW: node.w,
                boxH: node.h,
                overshootPx,
                fontSize: natural.fontSize,
              },
            },
          ),
        );
      }
    }
    return out;
  },
};

const textWrapTo1Ch: LintRule = {
  code: "TEXT_WRAP_TO_1CH",
  severity: "error",
  phase: "post-layout",
  check(ctx) {
    const out: Diagnostic[] = [];
    for (const { node, path } of walk(ctx.tree)) {
      if (node.type !== "text") continue;
      const text = (node as { text?: string }).text ?? "";
      const trimmed = text.trim();
      if (trimmed.length < 2) continue;
      const fontSize = (node as { fontSize?: number }).fontSize ?? 13;
      const fontFamily =
        (node as { fontFamily?: string }).fontFamily ?? "Inter";
      const nodeBold = !!(node as { bold?: boolean }).bold;
      // Probe the node's own text raw — `measureTextWidth` returns the
      // unwrapped width with no calculateResult-side margin, so the
      // per-char average is clean and short texts ("02", "03"…) don't
      // inflate themselves into false positives. The 10-px safety
      // margin baked into `measureText` is appropriate for wrap
      // decisions but distorts per-glyph estimates at small char counts.
      let charWidthPx: number;
      try {
        const lookup = ctx.measurer
          ? fontFamily
          : pickBundledFontForText(fontFamily, trimmed);
        const raw = measureTextWidthRaw(
          trimmed,
          lookup,
          fontSize,
          nodeBold ? "bold" : "normal",
          ctx.measurer,
        );
        charWidthPx = Math.max(1, raw / trimmed.length);
      } catch {
        // Historic 0.45em estimate on measurer failure — keeps the rule
        // firing on broken-measurer paths the same way it always did.
        charWidthPx = fontSize * 0.45;
      }
      const charsPerLine = Math.max(1, Math.floor(node.w / charWidthPx));
      if (charsPerLine < 2) {
        out.push(
          diag(
            "TEXT_WRAP_TO_1CH",
            "error",
            ctx,
            node,
            path,
            `<Text> is wrapping to ${charsPerLine} character per line ` +
              `(text "${text.slice(0, 24)}…" w=${Math.round(node.w)}px, fontSize=${fontSize}). ` +
              `Most likely a sibling claimed slack via w="max" without explicit widths on this column. ` +
              `Give this Text an explicit w, or remove w="max" from the title column.`,
            {
              context: { charsPerLine, w: node.w, fontSize },
              suggestedFix: {
                kind: "attribute-set",
                target: "self",
                set: { w: Math.max(40, fontSize * 3) },
                notes:
                  "Pin the squeezed column to an intrinsic-friendly width so the w='max' sibling absorbs slack instead.",
              },
            },
          ),
        );
      }
    }
    return out;
  },
};

const lineOverParent: LintRule = {
  code: "LINE_OVER_PARENT",
  severity: "warn",
  phase: "post-layout",
  check(ctx) {
    const out: Diagnostic[] = [];
    for (const { node, parent, path } of walk(ctx.tree)) {
      if (node.type !== "line") continue;
      if (!parent || parent.type === "layer") continue;
      const lineNode = node as PositionedNode & {
        x1: number;
        y1: number;
        x2: number;
        y2: number;
      };
      const ends = [
        { x: node.x + lineNode.x1, y: node.y + lineNode.y1 },
        { x: node.x + lineNode.x2, y: node.y + lineNode.y2 },
      ];
      for (const e of ends) {
        if (
          e.x < parent.x - 1 ||
          e.x > parent.x + parent.w + 1 ||
          e.y < parent.y - 1 ||
          e.y > parent.y + parent.h + 1
        ) {
          out.push(
            diag(
              "LINE_OVER_PARENT",
              "warn",
              ctx,
              node,
              path,
              `<Line> endpoint (${e.x.toFixed(0)},${e.y.toFixed(0)}) sits outside its ${parent.type} parent. ` +
                `Either move the endpoint inside, or wrap the line with <Layer>.`,
              {
                context: {
                  endpoint: e,
                  parent: {
                    x: parent.x,
                    y: parent.y,
                    w: parent.w,
                    h: parent.h,
                  },
                },
              },
            ),
          );
          break;
        }
      }
    }
    return out;
  },
};

const imageMissing: LintRule = {
  code: "IMAGE_MISSING",
  severity: "error",
  phase: "post-layout",
  check(ctx) {
    const out: Diagnostic[] = [];
    for (const { node, path } of walk(ctx.tree)) {
      if (node.type !== "image") continue;
      const src = (node as { src?: string }).src ?? "";
      const data = (node as { imageData?: string }).imageData;
      if (src && !data) {
        out.push(
          diag(
            "IMAGE_MISSING",
            "error",
            ctx,
            node,
            path,
            `<Image src="${src}"> failed to resolve — no bytes were attached for rendering.`,
            { context: { src } },
          ),
        );
      }
    }
    return out;
  },
};

// ============================================================================
// PHASE B — visual-coherence rules (post-layout)
// ============================================================================

function rowChildren(node: PositionedNode): PositionedNode[] {
  if (node.type !== "hstack") return [];
  const kids = getChildren(node);
  return kids ? kids.filter(isTextBearing) : [];
}

const inflatedLineHeight: LintRule = {
  code: "INFLATED_LINE_HEIGHT_IN_ROW",
  severity: "warn",
  phase: "post-layout",
  check(ctx) {
    const out: Diagnostic[] = [];
    for (const { node, path } of walk(ctx.tree)) {
      if (node.type !== "hstack") continue;
      const texts = rowChildren(node);
      if (texts.length < 2) continue;
      // If the row mixes lineHeights with at least one inflated (≥ 1.3), flag.
      const lhs = texts.map(
        (t) => (t as { lineHeight?: number }).lineHeight ?? 1.0,
      );
      const minLh = Math.min(...lhs);
      const maxLh = Math.max(...lhs);
      if (maxLh >= 1.3 && maxLh - minLh > 0.1) {
        out.push(
          diag(
            "INFLATED_LINE_HEIGHT_IN_ROW",
            "warn",
            ctx,
            node,
            path,
            `Row mixes lineHeights ${lhs.map((n) => n.toFixed(2)).join(" / ")}. ` +
              `In a single-line label row, the larger sibling's leading inflates the box height past the glyph extent and the baseline reads as offset against neighbors. ` +
              `Override lineHeight="1.0" on the larger Text for row-label use.`,
            {
              context: { lineHeights: lhs },
              suggestedFix: {
                kind: "attribute-set",
                target: "all-children",
                set: { lineHeight: 1.0 },
              },
              docsAnchor: "mixed-size-text-rows",
            },
          ),
        );
      }
    }
    return out;
  },
};

const anchorInconsistent: LintRule = {
  code: "ANCHOR_INCONSISTENT",
  severity: "warn",
  phase: "post-layout",
  check(ctx) {
    const out: Diagnostic[] = [];
    for (const { node, path } of walk(ctx.tree)) {
      if (node.type !== "hstack") continue;
      const texts = rowChildren(node);
      if (texts.length < 2) continue;
      const anchors = texts.map(
        (t) =>
          (t as { textVAlign?: "top" | "middle" | "bottom" }).textVAlign ??
          "top",
      );
      const uniq = new Set(anchors);
      if (uniq.size > 1) {
        out.push(
          diag(
            "ANCHOR_INCONSISTENT",
            "warn",
            ctx,
            node,
            path,
            `Row siblings have mixed textVAlign anchors ${[...uniq].join(" / ")}. ` +
              `Pick one anchor for the whole row — typically "middle" for label rows — or PowerPoint will render glyphs at inconsistent y positions.`,
            {
              context: { anchors },
              suggestedFix: {
                kind: "attribute-set",
                target: "all-children",
                set: { textVAlign: "middle" },
              },
              docsAnchor: "mixed-size-text-rows",
            },
          ),
        );
      }
    }
    return out;
  },
};

const baselineMixInRow: LintRule = {
  code: "BASELINE_MIX_IN_ROW",
  severity: "warn",
  phase: "post-layout",
  check(ctx) {
    const out: Diagnostic[] = [];
    for (const { node, path } of walk(ctx.tree)) {
      if (node.type !== "hstack") continue;
      const texts = rowChildren(node);
      if (texts.length < 2) continue;
      const fontSizes = texts.map(
        (t) => (t as { fontSize?: number }).fontSize ?? 13,
      );
      const minFs = Math.min(...fontSizes);
      const maxFs = Math.max(...fontSizes);
      if (maxFs - minFs < 2) continue; // sizes similar enough, no concern
      const anchors = texts.map(
        (t) =>
          (t as { textVAlign?: "top" | "middle" | "bottom" }).textVAlign ??
          "top",
      );
      const allMiddle = anchors.every((a) => a === "middle");
      const lhs = texts.map(
        (t) => (t as { lineHeight?: number }).lineHeight ?? 1.0,
      );
      const lhTight = lhs.every((lh) => lh <= 1.1);
      if (!allMiddle || !lhTight) {
        out.push(
          diag(
            "BASELINE_MIX_IN_ROW",
            "warn",
            ctx,
            node,
            path,
            `Row mixes fontSize ${fontSizes.join(" / ")} but does not apply the "mixed-size text rows" idiom ` +
              `(textVAlign="middle" + lineHeight="1.0" on every sibling). Glyph baselines will read as offset.`,
            {
              context: { fontSizes, anchors, lineHeights: lhs },
              suggestedFix: {
                kind: "attribute-set",
                target: "all-children",
                set: { textVAlign: "middle", lineHeight: 1.0 },
              },
              docsAnchor: "mixed-size-text-rows",
            },
          ),
        );
      }
    }
    return out;
  },
};

function bboxIntersectionArea(a: PositionedNode, b: PositionedNode): number {
  const x1 = Math.max(a.x, b.x);
  const y1 = Math.max(a.y, b.y);
  const x2 = Math.min(a.x + a.w, b.x + b.w);
  const y2 = Math.min(a.y + a.h, b.y + b.h);
  if (x2 <= x1 || y2 <= y1) return 0;
  return (x2 - x1) * (y2 - y1);
}

const overlapLayer: LintRule = {
  code: "OVERLAP_LAYER",
  severity: "info",
  phase: "post-layout",
  check(ctx) {
    const out: Diagnostic[] = [];
    for (const { node, path } of walk(ctx.tree)) {
      if (node.type !== "layer") continue;
      const kids = getChildren(node) ?? [];
      // Skip thin decorative kids (rules/dividers) — they intersect by design.
      const candidates = kids.filter(
        (k) =>
          (k.type !== "shape" && k.type !== "line") || (k.w > 6 && k.h > 6),
      );
      for (let i = 0; i < candidates.length; i++) {
        for (let j = i + 1; j < candidates.length; j++) {
          const a = candidates[i]!;
          const b = candidates[j]!;
          const inter = bboxIntersectionArea(a, b);
          const smaller = Math.min(a.w * a.h, b.w * b.h);
          if (smaller > 0 && inter / smaller > 0.5) {
            out.push(
              diag(
                "OVERLAP_LAYER",
                "info",
                ctx,
                node,
                path,
                `<Layer> children <${a.type}> and <${b.type}> overlap by >50% of the smaller bbox. ` +
                  `Verify this is intentional — Layer is the only container that does not auto-arrange.`,
                {
                  context: {
                    overlap: inter,
                    aSize: { w: a.w, h: a.h },
                    bSize: { w: b.w, h: b.h },
                  },
                },
              ),
            );
            break;
          }
        }
      }
    }
    return out;
  },
};

// hex → relative luminance (WCAG)
function relLum(hex: string): number {
  const rgb = [0, 2, 4].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255);
  const lin = rgb.map((c) =>
    c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4),
  );
  return 0.2126 * lin[0]! + 0.7152 * lin[1]! + 0.0722 * lin[2]!;
}
function contrastRatio(a: string, b: string): number {
  const la = relLum(a);
  const lb = relLum(b);
  const [hi, lo] = la > lb ? [la, lb] : [lb, la];
  return (hi + 0.05) / (lo + 0.05);
}

const lowContrast: LintRule = {
  code: "LOW_CONTRAST",
  severity: "info",
  phase: "post-layout",
  check(ctx) {
    const out: Diagnostic[] = [];
    // Walk + track nearest ancestor backgroundColor.
    function visit(
      node: PositionedNode,
      bg: string | undefined,
      path: PositionedNode[],
    ) {
      const ownBg = (node as { backgroundColor?: string }).backgroundColor;
      const effBg = ownBg ?? bg;
      if (
        (node.type === "text" || node.type === "ul" || node.type === "ol") &&
        effBg
      ) {
        const color = (node as { color?: string }).color;
        if (
          color &&
          /^[0-9a-fA-F]{6}$/.test(color) &&
          /^[0-9a-fA-F]{6}$/.test(effBg)
        ) {
          const ratio = contrastRatio(color, effBg);
          if (ratio < 4.5) {
            out.push(
              diag(
                "LOW_CONTRAST",
                "info",
                ctx,
                node,
                path,
                `<${node.type}> text color #${color} against background #${effBg} has contrast ratio ${ratio.toFixed(2)} — below WCAG AA (4.5).`,
                {
                  context: {
                    color,
                    bg: effBg,
                    ratio: Number(ratio.toFixed(2)),
                  },
                },
              ),
            );
          }
        }
      }
      const kids = getChildren(node);
      if (kids) {
        const np = [...path, node];
        for (const k of kids) visit(k, effBg, np);
      }
    }
    visit(ctx.tree, undefined, []);
    return out;
  },
};

// ============================================================================
// PHASE C — design-system rules (parse-tracked + post-layout)
// ============================================================================

const unusedStyle: LintRule = {
  code: "UNUSED_STYLE",
  severity: "info",
  phase: "post-layout",
  check(ctx) {
    if (!ctx.declaredStyles || !ctx.referencedStyles) return [];
    if (ctx.slideIndex !== 1) return []; // emit only once per deck
    const out: Diagnostic[] = [];
    for (const name of ctx.declaredStyles) {
      if (!ctx.referencedStyles.has(name)) {
        out.push({
          code: "UNUSED_STYLE",
          severity: "info",
          message: `Deck declares <Style name="${name}"/> but no element references it via class=`,
          nodeType: "Style",
          context: { name },
        });
      }
    }
    return out;
  },
};

const unusedTemplate: LintRule = {
  code: "UNUSED_TEMPLATE",
  severity: "info",
  phase: "post-layout",
  check(ctx) {
    if (!ctx.declaredTemplates || !ctx.referencedTemplates) return [];
    if (ctx.slideIndex !== 1) return [];
    const out: Diagnostic[] = [];
    for (const name of ctx.declaredTemplates) {
      if (!ctx.referencedTemplates.has(name)) {
        out.push({
          code: "UNUSED_TEMPLATE",
          severity: "info",
          message: `Deck declares <Template name="${name}"/> but no <Use> calls it`,
          nodeType: "Template",
          context: { name },
        });
      }
    }
    return out;
  },
};

const hardcodedColor: LintRule = {
  code: "HARDCODED_COLOR",
  severity: "info",
  phase: "post-layout",
  check(ctx) {
    // Heuristic: count hex literals appearing directly on attributes vs. via Styles.
    // We approximate via post-layout walk — every node with color / backgroundColor
    // gets a count. The signal: if many nodes share the same hex, suggest a Style.
    const counts = new Map<string, number>();
    for (const { node } of walk(ctx.tree)) {
      const color = (node as { color?: string }).color;
      const bg = (node as { backgroundColor?: string }).backgroundColor;
      for (const c of [color, bg]) {
        if (c && /^[0-9a-fA-F]{6}$/.test(c)) {
          counts.set(c, (counts.get(c) ?? 0) + 1);
        }
      }
    }
    const out: Diagnostic[] = [];
    if (ctx.slideIndex !== 1) return []; // emit once per deck
    for (const [hex, n] of counts) {
      if (n >= 4) {
        out.push({
          code: "HARDCODED_COLOR",
          severity: "info",
          message: `Color #${hex} appears ${n} times across nodes — promote to a <Style> token (e.g., name="t-accent" color="${hex}").`,
          nodeType: "Slide",
          context: { hex, occurrences: n },
        });
      }
    }
    return out;
  },
};

const inconsistentFont: LintRule = {
  code: "INCONSISTENT_FONT",
  severity: "info",
  phase: "post-layout",
  check(ctx) {
    if (ctx.slideIndex !== 1) return [];
    const families = new Set<string>();
    for (const { node } of walk(ctx.tree)) {
      const ff = (node as { fontFamily?: string }).fontFamily;
      if (ff && ff.trim().length > 0) families.add(ff);
    }
    if (families.size <= 2) return [];
    return [
      {
        code: "INCONSISTENT_FONT",
        severity: "info",
        message: `Deck uses ${families.size} distinct font families (${[...families].join(", ")}). Two-family pairing (display + body, or sans + mono) is the editorial baseline.`,
        nodeType: "Slide",
        context: { families: [...families] },
      },
    ];
  },
};

const masterCollision: LintRule = {
  code: "MASTER_COLLISION",
  severity: "warn",
  phase: "post-layout",
  check(_ctx) {
    // The positioned tree does not surface master geometry, and adding that
    // wiring is a large detour. The post-layout phase records what would
    // collide via overflow checks (OUT_OF_PAGE for header strip overlap);
    // this rule is intentionally a placeholder for the structured detection
    // route. We leave it as a no-op until master geometry is exposed
    // through the build context — at which point the check becomes
    // a bbox-vs-master-reserved-strip intersection test.
    return [];
  },
};

// ============================================================================
// PHASE D — accessibility rules (post-layout + parse)
// ============================================================================

const imgNoAlt: LintRule = {
  code: "IMG_NO_ALT",
  severity: "warn",
  phase: "post-layout",
  check(ctx) {
    const out: Diagnostic[] = [];
    for (const { node, path } of walk(ctx.tree)) {
      if (node.type !== "image") continue;
      const decorative = (node as { decorative?: boolean }).decorative;
      const alt = (node as { altText?: string }).altText;
      if (decorative === true) continue;
      if (!alt || alt.trim().length === 0) {
        out.push(
          diag(
            "IMG_NO_ALT",
            "warn",
            ctx,
            node,
            path,
            `<Image> without altText. Add altText="…" for screen readers, or decorative="true" if purely visual.`,
            {
              suggestedFix: {
                kind: "attribute-set",
                target: "self",
                set: { decorative: true },
                notes:
                  'If the image is informative, replace this with `altText="<description>"` instead of decorative.',
              },
            },
          ),
        );
      }
    }
    return out;
  },
};

const readingOrderAmbiguous: LintRule = {
  code: "READING_ORDER_AMBIGUOUS",
  severity: "info",
  phase: "post-layout",
  check(ctx) {
    const out: Diagnostic[] = [];
    for (const { node, path } of walk(ctx.tree)) {
      if (node.type !== "layer") continue;
      const kids = getChildren(node) ?? [];
      // Compare source order vs. visual top-to-bottom-left-to-right.
      const expected = [...kids].sort((a, b) => {
        if (Math.abs(a.y - b.y) > 8) return a.y - b.y;
        return a.x - b.x;
      });
      const sourceOrder = kids.map((k) => k.type + "@" + k.x + "," + k.y);
      const expectedOrder = expected.map((k) => k.type + "@" + k.x + "," + k.y);
      let mismatches = 0;
      for (let i = 0; i < kids.length; i++) {
        if (sourceOrder[i] !== expectedOrder[i]) mismatches++;
      }
      if (mismatches >= 2) {
        out.push(
          diag(
            "READING_ORDER_AMBIGUOUS",
            "info",
            ctx,
            node,
            path,
            `<Layer> source order differs from top-to-bottom visual order at ${mismatches} positions. ` +
              `Screen readers follow source order — reorder children if the visual flow should be read literally.`,
            { context: { mismatches } },
          ),
        );
      }
    }
    return out;
  },
};

const iconNoLabel: LintRule = {
  code: "ICON_NO_LABEL",
  severity: "info",
  phase: "post-layout",
  check(ctx) {
    const out: Diagnostic[] = [];
    for (const { node, parent, path } of walk(ctx.tree)) {
      if (node.type !== "icon") continue;
      const decorative = (node as { decorative?: boolean }).decorative;
      if (decorative === true) continue;
      const siblings = parent ? (getChildren(parent) ?? []) : [];
      const hasNeighborText = siblings.some(
        (s) => s !== node && isTextBearing(s),
      );
      if (!hasNeighborText) {
        out.push(
          diag(
            "ICON_NO_LABEL",
            "info",
            ctx,
            node,
            path,
            `<Icon> appears without an adjacent text label and is not decorative. ` +
              `Add a sibling Text or mark decorative="true".`,
          ),
        );
      }
    }
    return out;
  },
};

const tinyFont: LintRule = {
  code: "TINY_FONT",
  severity: "info",
  phase: "post-layout",
  check(ctx) {
    const out: Diagnostic[] = [];
    for (const { node, path } of walk(ctx.tree)) {
      if (!isTextBearing(node)) continue;
      const fsPx = (node as { fontSize?: number }).fontSize ?? 13;
      // px to pt: pt = px * 3/4
      const fsPt = fsPx * 0.75;
      if (fsPt < TINY_FONT_PT_THRESHOLD) {
        out.push(
          diag(
            "TINY_FONT",
            "info",
            ctx,
            node,
            path,
            `<${node.type}> fontSize=${fsPx}px (~${fsPt.toFixed(1)}pt) — below the 8pt readability floor for projected slides.`,
            { context: { fontSizePx: fsPx, fontSizePt: fsPt } },
          ),
        );
      }
    }
    return out;
  },
};

// ============================================================================
// PHASE E — performance rules (post-layout)
// ============================================================================

const largeImageInlined: LintRule = {
  code: "LARGE_IMAGE_INLINED",
  severity: "info",
  phase: "post-layout",
  check(ctx) {
    const out: Diagnostic[] = [];
    for (const { node, path } of walk(ctx.tree)) {
      if (node.type !== "image") continue;
      const data = (node as { imageData?: string }).imageData;
      if (!data) continue;
      // base64 data length is ~4/3 of bytes
      const bytes = Math.floor((data.length * 3) / 4);
      if (bytes > LARGE_IMAGE_BYTES_THRESHOLD) {
        const displayArea = node.w * node.h;
        // Heuristic: if displayed area is small (<200k px²), the inlined image is wasteful.
        if (displayArea < 200_000) {
          out.push(
            diag(
              "LARGE_IMAGE_INLINED",
              "info",
              ctx,
              node,
              path,
              `<Image> inlines ${(bytes / 1024).toFixed(0)} KB but renders at only ${Math.round(node.w)}x${Math.round(node.h)}px. ` +
                `Pre-resize the source — every consumer downloads the full payload.`,
              { context: { bytes, w: node.w, h: node.h } },
            ),
          );
        }
      }
    }
    return out;
  },
};

const excessNodes: LintRule = {
  code: "EXCESS_NODES",
  severity: "info",
  phase: "post-layout",
  check(ctx) {
    let count = 0;
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    for (const _entry of walk(ctx.tree)) count++;
    if (count > EXCESS_NODES_PER_SLIDE) {
      return [
        {
          code: "EXCESS_NODES",
          severity: "info",
          message: `Slide ${ctx.slideIndex}: ${count} nodes — PowerPoint's editing experience degrades past ~${EXCESS_NODES_PER_SLIDE} shapes per slide. Split into multiple slides or move repeating units into a master.`,
          nodeType: "Slide",
          context: { count, threshold: EXCESS_NODES_PER_SLIDE },
        },
      ];
    }
    return [];
  },
};

const slideFontCount: LintRule = {
  code: "SLIDE_FONT_COUNT",
  severity: "info",
  phase: "post-layout",
  check(ctx) {
    const families = new Set<string>();
    for (const { node } of walk(ctx.tree)) {
      const ff = (node as { fontFamily?: string }).fontFamily;
      if (ff && ff.trim().length > 0) families.add(ff);
    }
    if (families.size > MAX_FONTS_PER_SLIDE) {
      return [
        {
          code: "SLIDE_FONT_COUNT",
          severity: "info",
          message: `Slide ${ctx.slideIndex}: ${families.size} font families (${[...families].join(", ")}). Consolidate to ≤ ${MAX_FONTS_PER_SLIDE} for typographic consistency.`,
          nodeType: "Slide",
          context: { count: families.size, families: [...families] },
        },
      ];
    }
    return [];
  },
};

// ============================================================================

export const ALL_RULES: readonly LintRule[] = [
  // Phase 0 — raw-XML parse-phase
  rawLtGtInAttr,
  // Phase A — overflow / dimension
  outOfPage,
  outOfParent,
  zeroDim,
  negativeDim,
  textOverflowV,
  textOverflowH,
  textWrapTo1Ch,
  lineOverParent,
  imageMissing,
  // Phase B — visual coherence
  inflatedLineHeight,
  anchorInconsistent,
  baselineMixInRow,
  overlapLayer,
  lowContrast,
  // Phase C — design system
  unusedStyle,
  unusedTemplate,
  hardcodedColor,
  inconsistentFont,
  masterCollision,
  // Phase D — accessibility
  imgNoAlt,
  readingOrderAmbiguous,
  iconNoLabel,
  tinyFont,
  // Phase E — performance
  largeImageInlined,
  excessNodes,
  slideFontCount,
];
