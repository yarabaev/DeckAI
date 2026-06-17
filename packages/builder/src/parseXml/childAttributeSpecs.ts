/**
 * Per-child-element attribute specs.
 *
 * These describe child elements that are *not* full builder nodes (Master/Slide
 * objects, table Col/Tr/Td, list Li, inline format A/B/I/U/S/Mark/Span). The
 * schema for each child element is a flat `Record<attrName, AttributeSpec>`;
 * the dispatcher and child converters consume them via `coerceChildAttrs`.
 *
 * A future cleanup may roll these into the compiled registry alongside
 * `defineMeta` / `defineNode`. For now they live in this dedicated map so
 * that runtime coercion can be driven entirely by `AttributeSpec` and
 * `coerceByType` — without depending on the legacy `coerceWithRule`.
 */

import type { AttributeSpec } from "../registry/defineNode.ts";

export const CHILD_ATTRIBUTE_SPECS: Record<
  string,
  Record<string, AttributeSpec>
> = {
  Master: {
    name: { coerce: "string" },
    margin: { coerce: "padding", dotNotation: true },
    backgroundColor: { coerce: "color" },
    backgroundPath: { coerce: "string" },
    backgroundData: { coerce: "string" },
  },
  MasterText: {
    text: { coerce: "string" },
    x: { coerce: "number" },
    y: { coerce: "number" },
    w: { coerce: "number" },
    h: { coerce: "number" },
    fontSize: { coerce: "number" },
    fontFamily: { coerce: "string" },
    color: { coerce: "color" },
    bold: { coerce: "boolean" },
    italic: { coerce: "boolean" },
    underline: { coerce: "underline", dotNotation: true },
    strike: { coerce: "boolean" },
    highlight: { coerce: "color" },
    textAlign: { coerce: "string" },
    lineHeight: { coerce: "number" },
    letterSpacing: { coerce: "number" },
  },
  MasterImage: {
    src: { coerce: "string" },
    x: { coerce: "number" },
    y: { coerce: "number" },
    w: { coerce: "number" },
    h: { coerce: "number" },
  },
  MasterRect: {
    x: { coerce: "number" },
    y: { coerce: "number" },
    w: { coerce: "number" },
    h: { coerce: "number" },
    fill: { coerce: "fill", dotNotation: true },
    border: { coerce: "border", dotNotation: true },
    borderRadius: { coerce: "number" },
    opacity: { coerce: "number" },
  },
  MasterLine: {
    x: { coerce: "number" },
    y: { coerce: "number" },
    w: { coerce: "number" },
    h: { coerce: "number" },
    // Endpoint-pair alternative to (x, y, w, h). When all four are
    // provided the dispatcher folds them into the positioned-rect form
    // (x = x1, y = y1, w = x2 - x1, h = y2 - y1).
    x1: { coerce: "number" },
    y1: { coerce: "number" },
    x2: { coerce: "number" },
    y2: { coerce: "number" },
    line: { coerce: "border", dotNotation: true },
  },
  SlideNumber: {
    x: { coerce: "number" },
    y: { coerce: "number" },
    w: { coerce: "number" },
    h: { coerce: "number" },
    fontSize: { coerce: "number" },
    fontFamily: { coerce: "string" },
    color: { coerce: "color" },
    textAlign: { coerce: "string" },
  },
  Col: {
    w: { coerce: "length" },
  },
  Td: {
    text: { coerce: "string" },
    fontSize: { coerce: "number" },
    fontFamily: { coerce: "string" },
    color: { coerce: "color" },
    bold: { coerce: "boolean" },
    italic: { coerce: "boolean" },
    underline: { coerce: "underline", dotNotation: true },
    strike: { coerce: "boolean" },
    highlight: { coerce: "color" },
    textAlign: { coerce: "string" },
    verticalAlign: { coerce: "string" },
    backgroundColor: { coerce: "color" },
    colspan: { coerce: "number" },
    rowspan: { coerce: "number" },
    letterSpacing: { coerce: "number" },
    margin: { coerce: "padding", dotNotation: true },
    padding: { coerce: "padding", dotNotation: true },
  },
  Li: {
    text: { coerce: "string" },
    bold: { coerce: "boolean" },
    italic: { coerce: "boolean" },
    underline: { coerce: "underline", dotNotation: true },
    strike: { coerce: "boolean" },
    highlight: { coerce: "color" },
    color: { coerce: "color" },
    fontSize: { coerce: "number" },
    fontFamily: { coerce: "string" },
    letterSpacing: { coerce: "number" },
  },
  B: {},
  I: {},
  Span: { color: { coerce: "color" } },
  Tr: {
    h: { coerce: "length" },
  },
  Mark: {
    color: { coerce: "color" },
  },
  A: {
    href: { coerce: "string" },
  },
  U: {},
  S: {},
};
