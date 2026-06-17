/**
 * Self-contained coercion engine driven by registry `CoerceType`.
 *
 * Replaces the runtime use of `coercionRules.ts` for node-level attributes:
 * the dispatcher reads `AttributeSpec` (from compiled/index.ts) and calls
 * `coerceByType` to convert string XML attribute values to typed BuilderNode
 * fields. Sub-shape information for object-typed coerces lives in
 * `STRUCTURED_SHAPES` (implied by the type) or on the spec itself
 * (`AttributeSpec.objectShape` for `coerce: "json"` attrs that nonetheless
 * support dot-notation).
 */

import type { AttributeSpec, CoerceType } from "../registry/defineNode.ts";

export interface CoerceResult {
  value: unknown;
  error: string | null;
}

const HEX_RE = /^[0-9A-Fa-f]{6}$/;
const LENGTH_NUM_RE = /^-?\d+(\.\d+)?$/;
const PERCENT_RE = /^\d+%$/;

const ENUM_TYPES: ReadonlySet<CoerceType> = new Set([
  "alignSelf",
  "alignItems",
  "justifyContent",
  "flexWrap",
  "positionType",
  "shapeType",
  "bulletNumberType",
  "iconName",
  "iconVariant",
  "textAlign",
  "vAlign",
  "borderDash",
]);

/**
 * Sub-field shape for object-typed CoerceTypes whose structure is implied by
 * the type itself. Together with `AttributeSpec.objectShape` (for
 * `coerce: "json"` overrides), this drives dot-notation expansion in the
 * dispatcher.
 */
const STRUCTURED_SHAPES: Partial<
  Record<CoerceType, Record<string, CoerceType>>
> = {
  padding: {
    top: "number",
    right: "number",
    bottom: "number",
    left: "number",
  },
  border: { color: "color", width: "number", dashType: "string" },
  fill: { color: "color", transparency: "number" },
  shadow: {
    type: "string",
    opacity: "number",
    blur: "number",
    angle: "number",
    offset: "number",
    color: "color",
  },
  underline: { style: "string", color: "color" },
  imageSizing: {
    type: "string",
    w: "number",
    h: "number",
    x: "number",
    y: "number",
  },
  lineArrow: { type: "string", color: "color" },
};

/**
 * Effective sub-field shape for an attribute. Falls back to the
 * type-implied shape when the spec does not override.
 */
export function getObjectShape(
  spec: AttributeSpec,
): Record<string, CoerceType> | undefined {
  return spec.objectShape ?? STRUCTURED_SHAPES[spec.coerce];
}

function coerceNumber(value: string): CoerceResult {
  if (value === "") {
    return { value: undefined, error: 'Cannot convert "" to number' };
  }
  const n = Number(value);
  if (Number.isNaN(n)) {
    return { value: undefined, error: `Cannot convert "${value}" to number` };
  }
  return { value: n, error: null };
}

function coerceBoolean(value: string): CoerceResult {
  if (value === "true") return { value: true, error: null };
  if (value === "false") return { value: false, error: null };
  return {
    value: undefined,
    error: `Cannot convert "${value}" to boolean (expected "true" or "false")`,
  };
}

function coerceJson(value: string): CoerceResult {
  try {
    return { value: JSON.parse(value), error: null };
  } catch {
    return { value: undefined, error: `Cannot parse JSON value: "${value}"` };
  }
}

function coerceLength(value: string): CoerceResult {
  if (LENGTH_NUM_RE.test(value)) return { value: Number(value), error: null };
  if (value === "max") return { value: "max", error: null };
  if (PERCENT_RE.test(value)) return { value, error: null };
  return {
    value: undefined,
    error: `Cannot convert "${value}" to length (number, "max", or percent-string)`,
  };
}

function coerceColor(value: string): CoerceResult {
  const stripped = value.startsWith("#") ? value.slice(1) : value;
  if (HEX_RE.test(stripped)) return { value: stripped, error: null };
  return {
    value: undefined,
    error: `Cannot convert "${value}" to color (6-digit hex with optional # prefix)`,
  };
}

/**
 * Validate sub-fields of an object literal against its declared shape.
 * Mirrors the legacy `coerceWithRule({type:"object",shape})` behavior:
 * - Always returns the parsed JSON, even if it isn't strictly an object.
 * - Walks declared sub-fields; string values are recursively coerced.
 * - For `number`/`boolean` sub-fields, rejects mismatched types.
 */
function coerceObjectShape(
  value: string,
  shape: Record<string, CoerceType>,
): CoerceResult {
  let parsed: unknown;
  try {
    parsed = JSON.parse(value);
  } catch {
    return { value: undefined, error: `Cannot parse JSON value: "${value}"` };
  }
  if (typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)) {
    const obj = parsed as Record<string, unknown>;
    for (const [k, subType] of Object.entries(shape)) {
      if (!(k in obj)) continue;
      const subVal = obj[k];
      if (typeof subVal === "string") {
        const sub = coerceByType(subVal, subType);
        if (sub.error !== null) {
          return { value: undefined, error: `Field "${k}": ${sub.error}` };
        }
        obj[k] = sub.value;
      } else if (subType === "number" && typeof subVal !== "number") {
        return {
          value: undefined,
          error: `Field "${k}": expected number, got ${typeof subVal}`,
        };
      } else if (subType === "boolean" && typeof subVal !== "boolean") {
        return {
          value: undefined,
          error: `Field "${k}": expected boolean, got ${typeof subVal}`,
        };
      }
    }
  }
  return { value: parsed, error: null };
}

/**
 * Padding union: number OR CSS-style 2/3/4-value shorthand OR
 * object{top,right,bottom,left}.
 *
 * Shorthand decomposition matches CSS:
 *   `"V"`       → { top: V, right: V, bottom: V, left: V }
 *   `"V H"`     → { top: V, right: H, bottom: V, left: H }
 *   `"T H B"`   → { top: T, right: H, bottom: B, left: H }
 *   `"T R B L"` → { top: T, right: R, bottom: B, left: L }
 *
 * Numbers in object form are preserved as-is; the box-shorthand
 * expansion happens at the dispatcher layer for shorthand-vs-dot
 * conflicts.
 */
function coercePadding(value: string): CoerceResult {
  if (LENGTH_NUM_RE.test(value)) return { value: Number(value), error: null };
  // Object form (JSON, from preprocessed dot notation).
  if (value.startsWith("{") || value.startsWith("[")) {
    const result = coerceObjectShape(value, STRUCTURED_SHAPES.padding!);
    return result;
  }
  // CSS shorthand: 2, 3, or 4 whitespace-separated numbers.
  const parts = value.trim().split(/\s+/);
  if (
    (parts.length === 2 || parts.length === 3 || parts.length === 4) &&
    parts.every((p) => LENGTH_NUM_RE.test(p))
  ) {
    const nums = parts.map(Number);
    let top: number, right: number, bottom: number, left: number;
    if (nums.length === 2) {
      [top, right] = nums as [number, number];
      bottom = top;
      left = right;
    } else if (nums.length === 3) {
      [top, right, bottom] = nums as [number, number, number];
      left = right;
    } else {
      [top, right, bottom, left] = nums as [number, number, number, number];
    }
    return { value: { top, right, bottom, left }, error: null };
  }
  return {
    value: undefined,
    error: `Cannot convert "${value}" — no union option matched`,
  };
}

/**
 * Boolean-or-object union (underline, lineArrow). Plain `"true"`/`"false"`
 * collapses to a boolean; `"{...}"` parses as the object shape; anything
 * else errors. No string fallback — matches the legacy union behavior
 * where neither option declares `string`.
 */
function coerceBooleanOrObject(
  value: string,
  shape: Record<string, CoerceType>,
): CoerceResult {
  if (value === "true") return { value: true, error: null };
  if (value === "false") return { value: false, error: null };
  if (value.startsWith("{") || value.startsWith("[")) {
    return coerceObjectShape(value, shape);
  }
  return {
    value: undefined,
    error: `Cannot convert "${value}" — no union option matched`,
  };
}

/**
 * Coerce a string XML attribute value into its declared registry type.
 * `objectShape` overrides the implied shape for `coerce: "json"` attrs
 * that nonetheless need structured dot-notation handling.
 */
export function coerceByType(
  value: string,
  coerceType: CoerceType,
  objectShape?: Record<string, CoerceType>,
): CoerceResult {
  if (coerceType === "string" || ENUM_TYPES.has(coerceType)) {
    return { value, error: null };
  }
  switch (coerceType) {
    case "number":
      return coerceNumber(value);
    case "boolean":
      return coerceBoolean(value);
    case "json":
      if (objectShape) return coerceObjectShape(value, objectShape);
      return coerceJson(value);
    case "length":
      return coerceLength(value);
    case "color":
    case "iconColor":
      return coerceColor(value);
    case "padding":
      return coercePadding(value);
    case "border":
    case "fill":
    case "shadow":
    case "imageSizing":
      return coerceObjectShape(value, STRUCTURED_SHAPES[coerceType]!);
    case "underline":
    case "lineArrow":
      return coerceBooleanOrObject(value, STRUCTURED_SHAPES[coerceType]!);
  }
  return { value, error: null };
}

/**
 * Coerce by attribute spec. Convenience wrapper that pulls
 * `objectShape` off the spec if present.
 */
export function coerceBySpec(value: string, spec: AttributeSpec): CoerceResult {
  return coerceByType(value, spec.coerce, spec.objectShape);
}

/**
 * Loose fallback used for unmapped attrs (e.g., x/y safety net for
 * future nodes not yet in the registry). Tries boolean → number → JSON →
 * raw string.
 */
export function coerceFallback(value: string): unknown {
  if (value === "true") return true;
  if (value === "false") return false;
  const num = Number(value);
  if (value !== "" && !Number.isNaN(num)) return num;
  if (value.startsWith("{") || value.startsWith("[")) {
    try {
      return JSON.parse(value);
    } catch {
      /* ignore */
    }
  }
  return value;
}

type MixedNotationResolution =
  | { mode: "merge"; value: Record<string, unknown> }
  | { mode: "ignore" }
  | { mode: "conflict" };

/**
 * Resolve the same-key shorthand-vs-dot-notation conflict against an
 * attribute spec. Mirrors `resolveMixedNotationShorthand` from the legacy
 * coercion engine.
 */
export function resolveMixedNotationForSpec(
  value: string,
  spec: AttributeSpec,
): MixedNotationResolution {
  const shape = getObjectShape(spec);
  if (!shape) return { mode: "conflict" };

  // Boolean shorthand on a boolean+object union is silently ignored so
  // the dot-notation form wins (e.g. `endArrow="true"` + `endArrow.type="…"`).
  if (
    (spec.coerce === "underline" || spec.coerce === "lineArrow") &&
    (value === "true" || value === "false")
  ) {
    return { mode: "ignore" };
  }

  const coerced = coerceBySpec(value, spec);
  if (coerced.error !== null) return { mode: "conflict" };

  if (
    typeof coerced.value === "object" &&
    coerced.value !== null &&
    !Array.isArray(coerced.value)
  ) {
    return { mode: "merge", value: coerced.value as Record<string, unknown> };
  }

  // For padding-shaped objects: a single number expands to a uniform box.
  if (typeof coerced.value === "number") {
    const keys = Object.keys(shape).sort();
    if (
      keys.length === 4 &&
      keys[0] === "bottom" &&
      keys[1] === "left" &&
      keys[2] === "right" &&
      keys[3] === "top"
    ) {
      return {
        mode: "merge",
        value: {
          top: coerced.value,
          right: coerced.value,
          bottom: coerced.value,
          left: coerced.value,
        },
      };
    }
  }

  return { mode: "conflict" };
}
