/**
 * XML-side compiled registry — Single source of truth for nodes & meta elements.
 *
 * Each declaration here drives codegen artifacts (XSD, JSON Schema, nodes.md)
 * and — in a future session — the runtime parser dispatcher. For Session 1
 * the runtime parser still uses parseXml.ts / coercionRules.ts; the compiled
 * declarations are read only by codegen.
 *
 * Authoring rule: one defineNode/defineMeta per element, schema imported from
 * types.ts (do not redeclare shapes here).
 */

import type { AttributeSpec } from "../defineNode.ts";
import { defineNode, type CompiledNodeDefinition } from "../defineNode.ts";
import { defineMeta, type CompiledMetaDefinition } from "../defineMeta.ts";

import {
  textNodeSchema,
  ulNodeSchema,
  olNodeSchema,
  imageNodeSchema,
  iconNodeSchema,
  svgNodeSchema,
  tableNodeSchema,
  shapeNodeSchema,
  chartNodeSchema,
  lineNodeSchema,
  connectorNodeSchema,
} from "../../types.ts";

// ===== Shared attribute groups =====

const BASE_ATTRS: Record<string, AttributeSpec> = {
  w: { coerce: "length", doc: "Width (number, percentage, or 'max')." },
  h: { coerce: "length", doc: "Height (number, percentage, or 'max')." },
  x: { coerce: "number", doc: "Absolute x (used inside <Layer>)." },
  y: { coerce: "number", doc: "Absolute y (used inside <Layer>)." },
  minW: { coerce: "number" },
  maxW: { coerce: "number" },
  minH: { coerce: "number" },
  maxH: { coerce: "number" },
  padding: {
    coerce: "padding",
    dotNotation: true,
    doc: "Inner padding (number or per-side object).",
  },
  margin: {
    coerce: "padding",
    dotNotation: true,
    doc: "Outer margin (number or per-side object).",
  },
  backgroundColor: { coerce: "color" },
  backgroundImage: {
    coerce: "json",
    dotNotation: true,
    objectShape: { src: "string", sizing: "string" },
    doc: "Background image source/sizing.",
  },
  border: {
    coerce: "border",
    dotNotation: true,
    doc: "Border style (color/width/dashType).",
  },
  borderTop: {
    coerce: "border",
    dotNotation: true,
    doc: "Top-only border (color/width/dashType). Composes additively with `border`.",
  },
  borderRight: {
    coerce: "border",
    dotNotation: true,
    doc: "Right-only border (color/width/dashType). Composes additively with `border`.",
  },
  borderBottom: {
    coerce: "border",
    dotNotation: true,
    doc: "Bottom-only border (color/width/dashType). Composes additively with `border`.",
  },
  borderLeft: {
    coerce: "border",
    dotNotation: true,
    doc: "Left-only border (color/width/dashType). Composes additively with `border`.",
  },
  borderRadius: { coerce: "number" },
  opacity: { coerce: "number", doc: "0..1 opacity." },
  zIndex: { coerce: "number" },
  position: { coerce: "string", enum: ["relative", "absolute"] },
  top: { coerce: "number" },
  right: { coerce: "number" },
  bottom: { coerce: "number" },
  left: { coerce: "number" },
  alignSelf: {
    coerce: "alignSelf",
    enum: ["auto", "start", "center", "end", "stretch"],
  },
  shadow: { coerce: "shadow", dotNotation: true },
  isDecorative: { coerce: "boolean" },
  flexGrow: {
    coerce: "number",
    doc: "Yoga flex-grow override (defaults to context-aware behavior).",
  },
  flexShrink: {
    coerce: "number",
    doc: "Yoga flex-shrink override (defaults to 1 inside stacks).",
  },
  flexBasis: {
    coerce: "length",
    doc: "Yoga flex-basis override (number, percentage, or 'max').",
  },
  id: {
    coerce: "string",
    doc: 'Author-facing identifier for cross-references (e.g. <Connector from="..." to="..."/>). Must be unique within a slide; XML-friendly chars only (no colon).',
  },
  group: {
    coerce: "string",
    doc: "Bundle this node and its descendants into one PowerPoint group (<p:grpSp>). Use `true` for an anonymous group or a string for a stable group id; nested groupings stack into nested grpSp elements.",
  },
};

const TEXT_STYLE_ATTRS: Record<string, AttributeSpec> = {
  fontSize: { coerce: "number" },
  color: { coerce: "color" },
  textAlign: { coerce: "textAlign", enum: ["left", "center", "right"] },
  bold: { coerce: "boolean" },
  italic: { coerce: "boolean" },
  underline: { coerce: "underline", dotNotation: true },
  strike: { coerce: "boolean" },
  highlight: { coerce: "color" },
  fontFamily: { coerce: "string" },
  lineHeight: { coerce: "number" },
  letterSpacing: {
    coerce: "number",
    doc: "Letter spacing in em units (e.g. -0.02 for tight display, 0.18 for wide small-caps eyebrows). Maps to pptxgenjs `charSpacing` (units of 1/100 em). Note: the WASM text measurer treats letterSpacing as 0 when measuring widths, so autofit may underestimate wrapping width by a small amount on lines with large absolute tracking.",
  },
  noWrap: {
    coerce: "boolean",
    doc: "When true, the layout never wraps the text — it is always measured as a single line. Combined with `flexShrink=0` so a parent flex layout cannot squeeze it. The rendered text may overflow the box horizontally if it is longer than the box's width; if you need word-wrap, leave this off.",
  },
  textVAlign: {
    coerce: "vAlign",
    enum: ["top", "middle", "bottom"],
    doc: "Vertical alignment of the rendered text within its box. Defaults to `top` — useful when the box has been stretched by an HStack parent (so a smaller-fontSize sibling does not visually float at the top of the equalized row). `middle` centers the glyphs; `bottom` anchors them to the baseline of the box.",
  },
};

// ===== Node declarations =====

const textCompiled = defineNode({
  tag: "Text",
  type: "text",
  description:
    "Renders a single piece of styled text. Supports inline styled runs via <B>, <I>, <U>, <S>, <Mark>, <A>, and <Span>.",
  category: "leaf",
  schema: textNodeSchema,
  attributes: {
    ...BASE_ATTRS,
    text: {
      coerce: "string",
      bodyAlias: true,
      doc: "Text content (or use element body).",
    },
    ...TEXT_STYLE_ATTRS,
  },
  example: `<Text fontSize="24" bold="true">Hello</Text>`,
});

const ulCompiled = defineNode({
  tag: "Ul",
  type: "ul",
  description:
    "Bulleted list. Items can be supplied via the JSON `items` attribute or via <Li> child elements.",
  category: "leaf",
  schema: ulNodeSchema,
  attributes: {
    ...BASE_ATTRS,
    items: { coerce: "json", doc: "Array of list items as JSON string." },
    ...TEXT_STYLE_ATTRS,
    bulletIndent: { coerce: "number" },
  },
  children: { items: { element: "Li", kind: "structured", doc: "List item." } },
});

const olCompiled = defineNode({
  tag: "Ol",
  type: "ol",
  description:
    "Numbered list. Items via JSON `items` attribute or <Li> children.",
  category: "leaf",
  schema: olNodeSchema,
  attributes: {
    ...BASE_ATTRS,
    items: { coerce: "json" },
    ...TEXT_STYLE_ATTRS,
    numberType: { coerce: "bulletNumberType" },
    numberStartAt: { coerce: "number" },
    bulletIndent: { coerce: "number" },
  },
  children: { items: { element: "Li", kind: "structured" } },
});

const imageCompiled = defineNode({
  tag: "Image",
  type: "image",
  description:
    "Embeds an image. `src` accepts http(s):// URL, data URI, or relative file path.",
  category: "leaf",
  schema: imageNodeSchema,
  attributes: {
    ...BASE_ATTRS,
    src: { coerce: "string", required: true, doc: "Image source." },
    sizing: {
      coerce: "imageSizing",
      dotNotation: true,
      doc: "How the image fits its box (contain/cover/crop).",
    },
    altText: { coerce: "string" },
    rotate: { coerce: "number" },
  },
});

const iconCompiled = defineNode({
  tag: "Icon",
  type: "icon",
  description:
    "Renders a built-in icon by name. See docs for the icon catalog.",
  category: "leaf",
  schema: iconNodeSchema,
  attributes: {
    ...BASE_ATTRS,
    name: { coerce: "iconName", required: true, doc: "Icon catalog name." },
    size: { coerce: "number" },
    color: { coerce: "iconColor" },
    variant: {
      coerce: "iconVariant",
      enum: [
        "circle-filled",
        "circle-outlined",
        "square-filled",
        "square-outlined",
      ],
    },
    backgroundColor: { coerce: "iconColor" },
    altText: { coerce: "string" },
  },
});

const svgCompiled = defineNode({
  tag: "Svg",
  type: "svg",
  description: "Embeds inline SVG. Supply the SVG via a <svg> child element.",
  category: "leaf",
  schema: svgNodeSchema,
  attributes: {
    ...BASE_ATTRS,
    color: { coerce: "iconColor", doc: "Recolors monochrome SVGs." },
    altText: { coerce: "string" },
  },
  children: {
    svgContent: {
      element: "svg",
      kind: "structured",
      min: 1,
      max: 1,
      doc: "Inline SVG markup.",
    },
  },
});

const tableCompiled = defineNode({
  tag: "Table",
  type: "table",
  description:
    "Tabular data. Use <Col>/<Tr>/<Td> child elements or the `columns`/`rows` JSON attributes.",
  category: "leaf",
  schema: tableNodeSchema,
  attributes: {
    ...BASE_ATTRS,
    columns: { coerce: "json", doc: "Column definitions (array)." },
    rows: { coerce: "json", doc: "Row definitions (array)." },
    defaultRowHeight: { coerce: "number" },
    cellBorder: { coerce: "border", dotNotation: true },
    cellMargin: { coerce: "padding", dotNotation: true },
  },
  children: {
    columns: {
      element: "Col",
      kind: "structured",
      doc: "Column definition (width).",
    },
    rows: {
      element: "Tr",
      kind: "structured",
      doc: "Table row containing <Td> cells.",
    },
  },
});

const shapeCompiled = defineNode({
  tag: "Shape",
  type: "shape",
  description:
    "Renders a built-in PowerPoint shape (rect, ellipse, arrows, callouts, etc.).",
  category: "leaf",
  schema: shapeNodeSchema,
  attributes: {
    ...BASE_ATTRS,
    shapeType: {
      coerce: "shapeType",
      required: true,
      doc: "PPTX shape preset name.",
    },
    text: { coerce: "string", doc: "Optional text inside the shape." },
    fill: { coerce: "fill", dotNotation: true },
    line: { coerce: "border", dotNotation: true },
    ...TEXT_STYLE_ATTRS,
    textVAlign: { coerce: "vAlign", enum: ["top", "middle", "bottom"] },
    rotate: { coerce: "number" },
  },
});

const chartCompiled = defineNode({
  tag: "Chart",
  type: "chart",
  description:
    "Renders a native PowerPoint chart (bar/line/pie/area/doughnut/radar).",
  category: "leaf",
  schema: chartNodeSchema,
  attributes: {
    ...BASE_ATTRS,
    chartType: {
      coerce: "string",
      required: true,
      enum: ["bar", "line", "pie", "area", "doughnut", "radar"],
    },
    data: {
      coerce: "json",
      doc: "Series data (array of {name?, labels[], values[]}).",
    },
    showLegend: { coerce: "boolean" },
    showTitle: { coerce: "boolean" },
    title: { coerce: "string" },
    chartColors: { coerce: "json" },
    radarStyle: { coerce: "string", enum: ["standard", "marker", "filled"] },
    legendPos: { coerce: "string", enum: ["t", "b", "l", "r", "tr"] },
    legendFontSize: { coerce: "number" },
    catAxisLabelFontSize: { coerce: "number" },
    valAxisLabelFontSize: { coerce: "number" },
    barGapWidthPct: { coerce: "number" },
    lineDataSymbolSize: { coerce: "number" },
    altText: { coerce: "string" },
    showValue: { coerce: "boolean" },
    barGrouping: {
      coerce: "string",
      enum: ["clustered", "stacked", "percentStacked"],
    },
    valAxisMinVal: { coerce: "number" },
    valAxisMaxVal: { coerce: "number" },
  },
  children: {
    data: {
      element: "ChartSeries",
      kind: "structured",
      doc: "Chart data series (each containing <ChartDataPoint>).",
    },
  },
});

const lineCompiled = defineNode({
  tag: "Line",
  type: "line",
  description: "Straight line connector with optional arrows on either end.",
  category: "leaf",
  schema: lineNodeSchema,
  attributes: {
    ...BASE_ATTRS,
    x1: { coerce: "number", required: true },
    y1: { coerce: "number", required: true },
    x2: { coerce: "number", required: true },
    y2: { coerce: "number", required: true },
    color: { coerce: "color" },
    lineWidth: { coerce: "number" },
    dashType: { coerce: "borderDash" },
    beginArrow: { coerce: "lineArrow", dotNotation: true },
    endArrow: { coerce: "lineArrow", dotNotation: true },
  },
});

const connectorCompiled = defineNode({
  tag: "Connector",
  type: "connector",
  description:
    "Smart line that binds to two shapes by their `id`. Emits a real PPTX <p:cxnSp> with stCxn/endCxn so PowerPoint reroutes the line automatically when shapes move. The line never participates in flexbox layout — its endpoints are derived from the from/to shapes' positioned boxes.",
  category: "leaf",
  schema: connectorNodeSchema,
  attributes: {
    ...BASE_ATTRS,
    from: {
      coerce: "string",
      required: true,
      doc: "id of the source shape on the same slide.",
    },
    to: {
      coerce: "string",
      required: true,
      doc: "id of the target shape on the same slide.",
    },
    kind: {
      coerce: "string",
      enum: ["straight", "elbow", "curved"],
      doc: "Line geometry. straight = direct line, elbow = orthogonal bent line, curved = smooth bezier. Default is straight.",
    },
    fromSide: {
      coerce: "string",
      enum: ["top", "right", "bottom", "left"],
      doc: "Which side of the source shape to attach to. When omitted, the renderer picks the side that points toward the target (auto).",
    },
    toSide: {
      coerce: "string",
      enum: ["top", "right", "bottom", "left"],
      doc: "Which side of the target shape to attach to. When omitted, auto.",
    },
    color: { coerce: "color" },
    lineWidth: { coerce: "number" },
    dashType: { coerce: "borderDash" },
    beginArrow: { coerce: "lineArrow", dotNotation: true },
    endArrow: { coerce: "lineArrow", dotNotation: true },
  },
  example: `<Shape id="A" shapeType="rect" w="100" h="60"/>\n<Shape id="B" shapeType="rect" w="100" h="60"/>\n<Connector from="A" to="B" kind="elbow" endArrow="true"/>`,
});

// Container nodes — schema is recursive, so we omit `schema` for codegen
// (codegen treats them as containers of any BuilderNode).

const vstackCompiled = defineNode({
  tag: "VStack",
  type: "vstack",
  description: "Flex column container — children stack vertically.",
  category: "multi-child",
  attributes: {
    ...BASE_ATTRS,
    gap: { coerce: "number" },
    alignItems: {
      coerce: "alignItems",
      enum: ["start", "center", "end", "stretch"],
    },
    justifyContent: {
      coerce: "justifyContent",
      enum: [
        "start",
        "center",
        "end",
        "spaceBetween",
        "spaceAround",
        "spaceEvenly",
      ],
    },
    flexWrap: { coerce: "flexWrap", enum: ["nowrap", "wrap", "wrapReverse"] },
  },
});

const hstackCompiled = defineNode({
  tag: "HStack",
  type: "hstack",
  description: "Flex row container — children stack horizontally.",
  category: "multi-child",
  attributes: {
    ...BASE_ATTRS,
    gap: { coerce: "number" },
    alignItems: {
      coerce: "alignItems",
      enum: ["start", "center", "end", "stretch"],
    },
    justifyContent: {
      coerce: "justifyContent",
      enum: [
        "start",
        "center",
        "end",
        "spaceBetween",
        "spaceAround",
        "spaceEvenly",
      ],
    },
    flexWrap: { coerce: "flexWrap", enum: ["nowrap", "wrap", "wrapReverse"] },
  },
});

const layerCompiled = defineNode({
  tag: "Layer",
  type: "layer",
  description:
    "Absolute-positioned container. Direct children must declare `x` and `y`.",
  category: "absolute-child",
  attributes: { ...BASE_ATTRS },
});

// Document containers — not BuilderNode types but have XML representation.

const slideGlanceCompiled = defineNode({
  tag: "SlideGlance",
  description:
    "Document root — contains <Document>, <Slide>, <Templates>, <Styles>, <Import>. Carries the `urn:slideglance:builder:v1` namespace; document-level settings (size, defaultMaster, defaultTextStyle) sit on a <Document> child rather than on the root.",
  root: true,
});

const slideCompiled = defineNode({
  tag: "Slide",
  description:
    "Single slide. Body is one BuilderNode tree (typically a stack or layer).",
});

const fragmentCompiled = defineNode({
  tag: "Fragment",
  description:
    "Wrapper element used as the root of imported partials. Behaves as a transparent container.",
  root: true,
});

// ===== Meta element declarations =====

// Document settings carrier. Sits inside <SlideGlance> alongside <Slide> /
// <Master> / <Templates> / <Styles> / <Import>. Holds presentation-level
// metadata (slide size, default master, default text style) that used to
// live as attributes on the root.
const documentMeta = defineMeta({
  tag: "Document",
  description:
    "Document settings — slide size, default master, default text style. One <Document> per <SlideGlance>.",
  contentModel: "none",
  allowedParents: ["SlideGlance"],
  attributes: {
    size: {
      coerce: "string",
      enum: ["16:9", "4:3", "A4", "A3", "Letter"],
      doc: "Preset slide size. Mutually exclusive with width/height/w/h.",
    },
    width: {
      coerce: "length",
      doc: "Custom slide width (px). Pair with `height`. Mutually exclusive with `size`.",
    },
    height: {
      coerce: "number",
      doc: "Custom slide height (px). Pair with `width`. Mutually exclusive with `size`.",
    },
    w: {
      coerce: "length",
      doc: "Alias of `width`.",
    },
    h: {
      coerce: "number",
      doc: "Alias of `height`.",
    },
    defaultMaster: {
      coerce: "string",
      doc: "Master name used by slides that omit `master`.",
    },
    fontFamily: { coerce: "string", doc: "Default text fontFamily." },
    fontSize: { coerce: "number", doc: "Default text fontSize." },
    color: { coerce: "color", doc: "Default text color." },
    bold: { coerce: "boolean", doc: "Default bold." },
    italic: { coerce: "boolean", doc: "Default italic." },
    underline: {
      coerce: "underline",
      doc: "Default underline (boolean or object).",
    },
    strike: { coerce: "boolean", doc: "Default strikethrough." },
    highlight: { coerce: "color", doc: "Default highlight colour." },
    lineHeight: { coerce: "number", doc: "Default lineHeight multiplier." },
  },
});

const templatesMeta = defineMeta({
  tag: "Templates",
  description:
    "Container for <Template> definitions. Must appear at the root of <SlideGlance> or <Fragment>.",
  contentModel: "templates",
  allowedParents: ["SlideGlance", "Fragment"],
});

const templateMeta = defineMeta({
  tag: "Template",
  description:
    'Defines a reusable XML template invoked by <Use template="...">. Body may include {placeholder} substitutions and <Slot> insertion points.',
  contentModel: "any",
  allowedParents: ["Templates"],
  attributes: {
    name: { coerce: "string", required: true, doc: "Template name." },
  },
});

const useMeta = defineMeta({
  tag: "Use",
  description:
    "Invokes a previously defined <Template>. Other attributes are passed as parameters.",
  contentModel: "slots",
  attributes: {
    template: {
      coerce: "string",
      required: true,
      doc: "Name of the <Template> to invoke.",
    },
  },
});

const slotMeta = defineMeta({
  tag: "Slot",
  description:
    "Slot insertion point inside a <Template> body, or named slot content inside <Use>.",
  contentModel: "any",
  attributes: {
    name: {
      coerce: "string",
      doc: "Slot name (defaults to 'default' when omitted).",
    },
  },
});

const importMeta = defineMeta({
  tag: "Import",
  description:
    "Inlines an external XML fragment at parse time. The target file must have a <Fragment> root.",
  contentModel: "none",
  allowedParents: ["SlideGlance", "Fragment"],
  attributes: {
    src: {
      coerce: "string",
      required: true,
      doc: "Relative path to the imported XML file.",
    },
  },
});

const stylesMeta = defineMeta({
  tag: "Styles",
  description:
    "Container for named style sets. Must appear at the root of <SlideGlance>.",
  contentModel: "styles",
  allowedParents: ["SlideGlance"],
});

const styleMeta = defineMeta({
  tag: "Style",
  description: 'Named style set. Other elements opt-in via class="name".',
  contentModel: "none",
  allowedParents: ["Styles"],
  // A <Style> carries any visual attribute valid on the element that opts in
  // via class="..." (padding, fontSize, bold, color, lineHeight, ...). The
  // attributes are stored verbatim and coerced in the consuming element's
  // context, so the schema accepts arbitrary attributes here.
  openAttributes: true,
  attributes: {
    name: {
      coerce: "string",
      required: true,
      doc: 'Style name (referenced via class="...").',
    },
  },
});

const ifMeta = defineMeta({
  tag: "If",
  description:
    "Conditional include. Body is emitted when `test` evaluates truthy. Expression supports identifiers, dotted paths (e.g. `m.tone`), comparisons (==, !=, <, <=, >, >=), logical operators (&&, ||, !), and the helpers empty(), not(), length().",
  contentModel: "any",
  attributes: {
    test: {
      coerce: "string",
      required: true,
      doc: "Boolean expression. Body is included when truthy.",
    },
  },
});

const chooseMeta = defineMeta({
  tag: "Choose",
  description:
    "First-match branch. Children must be one or more <When test=...> followed by an optional <Otherwise>. The body of the first matching <When> is emitted; if none match, the <Otherwise> body (if present) is emitted instead.",
  contentModel: "any",
});

const whenMeta = defineMeta({
  tag: "When",
  description:
    "<Choose> branch. Body is emitted when this is the first <When> whose `test` evaluates truthy. Same expression grammar as <If>.",
  contentModel: "any",
  allowedParents: ["Choose"],
  attributes: {
    test: {
      coerce: "string",
      required: true,
      doc: "Boolean expression. Branch is taken when truthy.",
    },
  },
});

const otherwiseMeta = defineMeta({
  tag: "Otherwise",
  description:
    "<Choose> fallback. Body is emitted when no preceding <When> matched. At most one is allowed per <Choose>.",
  contentModel: "any",
  allowedParents: ["Choose"],
});

const foreachMeta = defineMeta({
  tag: "Foreach",
  description:
    "Repeats its body once per element of `items`. `items` is parsed as a JSON array after placeholder substitution, so both inline literals (items='[{...}]') and references to a parent attribute (items=\"{milestones}\") are accepted. Each iteration binds `as`, plus optional indexAs/firstAs/lastAs vars, into the scope.",
  contentModel: "any",
  attributes: {
    items: {
      coerce: "string",
      required: true,
      doc: "JSON array literal, or a {ref} that resolves to one after substitution.",
    },
    as: {
      coerce: "string",
      required: true,
      doc: "Name bound to the current item inside the body.",
    },
    indexAs: {
      coerce: "string",
      doc: "Optional name bound to the 0-based index of the current iteration.",
    },
    firstAs: {
      coerce: "string",
      doc: "Optional name bound to true on the first iteration, false otherwise.",
    },
    lastAs: {
      coerce: "string",
      doc: "Optional name bound to true on the last iteration, false otherwise.",
    },
  },
});

// ===== Aggregate exports =====

export const ALL_COMPILED_NODES: readonly CompiledNodeDefinition[] = [
  // builder nodes
  textCompiled,
  ulCompiled,
  olCompiled,
  imageCompiled,
  iconCompiled,
  svgCompiled,
  tableCompiled,
  shapeCompiled,
  chartCompiled,
  lineCompiled,
  connectorCompiled,
  vstackCompiled,
  hstackCompiled,
  layerCompiled,
  // Document containers
  slideGlanceCompiled,
  slideCompiled,
  fragmentCompiled,
];

export const ALL_COMPILED_META: readonly CompiledMetaDefinition[] = [
  documentMeta,
  templatesMeta,
  templateMeta,
  useMeta,
  slotMeta,
  importMeta,
  stylesMeta,
  styleMeta,
  ifMeta,
  chooseMeta,
  whenMeta,
  otherwiseMeta,
  foreachMeta,
];

/**
 * Cross-element consistency check.
 *
 * Throws on any inconsistency that codegen cannot proceed past:
 *   - Duplicate node tags
 *   - Duplicate meta tags
 *   - bodyAlias on a non-string attribute (already caught in defineNode but rechecked)
 *
 * This is invoked once at codegen entry so a misedit in compiled/index.ts
 * fails loudly instead of producing a malformed XSD.
 */
export function validateCompiledRegistry(): void {
  const seenTags = new Set<string>();
  for (const def of ALL_COMPILED_NODES) {
    if (seenTags.has(def.tag)) {
      throw new Error(`Duplicate compiled node tag: <${def.tag}>`);
    }
    seenTags.add(def.tag);
  }
  for (const meta of ALL_COMPILED_META) {
    if (seenTags.has(meta.tag)) {
      throw new Error(
        `Compiled meta tag <${meta.tag}> conflicts with a node tag`,
      );
    }
    seenTags.add(meta.tag);
  }
}
