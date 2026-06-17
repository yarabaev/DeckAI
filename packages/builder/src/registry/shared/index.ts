/**
 * Shared atom schemas + inferred types for builder nodes.
 *
 * These are the small, composable building blocks (length, padding,
 * border, fill, shadow, color enums, etc.) that compose the per-node
 * schemas in `src/types.ts`. The compiled registry references the same
 * atoms via `CoerceType`; runtime coercion lives in
 * `src/parseXml/coerceByType.ts`.
 */

export * from "./length.ts";
export * from "./padding.ts";
export * from "./border.ts";
export * from "./fill.ts";
export * from "./enums.ts";
export * from "./text-style.ts";
export * from "./background.ts";
export * from "./shape.ts";
