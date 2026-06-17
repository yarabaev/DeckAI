/**
 * Codegen helpers — flatten the compiled registry into a structure each
 * emitter can iterate independently.
 *
 * The compiled registry already exposes ALL_COMPILED_NODES / ALL_COMPILED_META;
 * this module wraps them with computed projections (root elements, container
 * categories, per-coerce simpleType mapping) so each emitter does not have to
 * recompute them.
 */

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  ALL_COMPILED_NODES,
  ALL_COMPILED_META,
  validateCompiledRegistry,
} from "../registry/compiled/index.ts";
import type {
  CompiledNodeDefinition,
  CoerceType,
} from "../registry/defineNode.ts";
import type { NodeCategory } from "../registry/types.ts";
import type { CompiledMetaDefinition } from "../registry/defineMeta.ts";
import { SEE_ALSO, type SeeAlsoEntry } from "./seeAlso.ts";

interface WalkedRegistry {
  readonly nodes: readonly CompiledNodeDefinition[];
  readonly meta: readonly CompiledMetaDefinition[];
  /** Root-eligible element tags (SlideGlance, Fragment). */
  readonly roots: readonly string[];
  /** Tags accepted as a generic POM node child (used in xs:choice for containers). */
  readonly nodeTags: readonly string[];
}

export function walkRegistry(): WalkedRegistry {
  validateCompiledRegistry();

  const nodes = ALL_COMPILED_NODES;
  const meta = ALL_COMPILED_META;
  const roots = nodes.filter((n) => n.root).map((n) => n.tag);

  // builder node tags = nodes that have a `type` discriminant; excludes
  // document containers (SlideGlance/Slide/Fragment).
  const nodeTags = nodes.filter((n) => n.type !== undefined).map((n) => n.tag);

  return { nodes, meta, roots, nodeTags };
}

/** Map a coerce type to a named XSD simpleType (or null for inline xs:string). */
export function coerceToXsdSimpleType(coerce: CoerceType): {
  /** Named simpleType reference (with `b:` prefix) or null when inline-only. */
  named: string | null;
  /** Built-in xs primitive used inline when `named` is null. */
  primitive: string;
} {
  switch (coerce) {
    case "number":
      return { named: null, primitive: "xs:double" };
    case "boolean":
      return { named: "b:Boolean", primitive: "xs:boolean" };
    case "string":
    case "json":
      return { named: null, primitive: "xs:string" };
    case "length":
      return { named: "b:Length", primitive: "xs:string" };
    case "color":
    case "iconColor":
      return { named: "b:Color", primitive: "xs:string" };
    case "padding":
      return { named: "b:Padding", primitive: "xs:string" };
    case "border":
      return { named: "b:BorderStyle", primitive: "xs:string" };
    case "fill":
      return { named: "b:FillStyle", primitive: "xs:string" };
    case "shadow":
      return { named: "b:ShadowStyle", primitive: "xs:string" };
    case "underline":
      return { named: "b:Underline", primitive: "xs:string" };
    case "imageSizing":
      return { named: "b:ImageSizing", primitive: "xs:string" };
    case "lineArrow":
      return { named: "b:LineArrow", primitive: "xs:string" };
    case "borderDash":
      return { named: "b:BorderDash", primitive: "xs:string" };
    case "shapeType":
      return { named: "b:ShapeType", primitive: "xs:string" };
    case "bulletNumberType":
      return { named: "b:BulletNumberType", primitive: "xs:string" };
    case "iconName":
      return { named: "b:IconName", primitive: "xs:string" };
    case "iconVariant":
      return { named: "b:IconVariant", primitive: "xs:string" };
    case "alignSelf":
    case "alignItems":
    case "justifyContent":
    case "flexWrap":
    case "positionType":
    case "textAlign":
    case "vAlign":
      // These are simple enums, emitted inline via the attribute's `enum` field.
      return { named: null, primitive: "xs:string" };
  }
}

/**
 * Aggregate reference model consumed by the HTML emitter. Built on top of
 * `walkRegistry()`, this adds metadata (namespace, package version), a
 * pre-parsed see-also table, and placeholders for the reverse index +
 * source locations populated by later tasks.
 */
export interface ReferenceModel {
  readonly generatedAt: string;
  readonly packageVersion: string;
  readonly namespace: string;
  readonly nodes: readonly CompiledNodeDefinition[];
  readonly meta: readonly CompiledMetaDefinition[];
  readonly usedBy: ReadonlyMap<
    string,
    ReadonlyArray<{ parent: string; cardinality: string }>
  >;
  readonly seeAlso: ReadonlyMap<string, ReadonlyArray<SeeAlsoEntry>>;
  readonly sourceLocations: ReadonlyMap<string, { file: string; line: number }>;
}

const BUILDER_NS = "urn:slideglance:builder:v1";

function readPackageVersion(): string {
  const HERE = fileURLToPath(import.meta.url);
  const pkgPath = resolve(HERE, "../../../package.json");
  const pkg = JSON.parse(readFileSync(pkgPath, "utf8")) as { version: string };
  return pkg.version;
}

export function buildReferenceModel(): ReferenceModel {
  const r = walkRegistry();
  return {
    generatedAt: "",
    packageVersion: readPackageVersion(),
    namespace: BUILDER_NS,
    nodes: r.nodes,
    meta: r.meta,
    usedBy: deriveUsedBy(r.nodes, r.meta),
    seeAlso: new Map(Object.entries(SEE_ALSO)),
    sourceLocations: deriveSourceLocations(),
  };
}

/**
 * Scan `registry/compiled/index.ts` to associate each registered tag with the
 * source line of its `defineNode(...)` / `defineMeta(...)` call. The HTML
 * reference uses this to render "Defined at: <file>:<line>" footers.
 *
 * The registry uses the object form exclusively, with `tag:` on the line
 * immediately following `defineNode({` / `defineMeta({`. The regex matches
 * across the line break (`\s*` straddles whitespace including newlines);
 * `matchAll` returns the match plus its index so the line number is derived
 * by counting newlines in the prefix.
 */
function deriveSourceLocations(): Map<string, { file: string; line: number }> {
  const HERE = fileURLToPath(import.meta.url);
  const file = "src/registry/compiled/index.ts";
  const abs = resolve(HERE, "../../registry/compiled/index.ts");
  const content = readFileSync(abs, "utf8");
  const result = new Map<string, { file: string; line: number }>();
  const re = /define(?:Node|Meta)\(\s*\{\s*tag:\s*["'](\w+)["']/g;
  for (const match of content.matchAll(re)) {
    const tag = match[1];
    if (tag === undefined) continue; // unreachable: \w+ always captures.
    const idx = match.index ?? 0;
    const line = content.slice(0, idx).split("\n").length;
    result.set(tag, { file, line });
  }
  return result;
}

/**
 * Whether a `NodeCategory` represents a container that accepts arbitrary
 * builder-node children. Switch is exhaustive against `NodeCategory` so adding
 * a new category in `src/registry/types.ts` triggers a TypeScript error here.
 */
function isContainerCategory(c: NodeCategory): boolean {
  switch (c) {
    case "leaf":
      return false;
    case "multi-child":
    case "absolute-child":
      return true;
  }
}

/**
 * Build the reverse parent-of index used by the HTML reference's "Used by"
 * section. Three passes merge declarative + inferred relationships:
 *
 *   Pass A — visual nodes declare children forward (e.g. Ul → Li, Table → Tr).
 *   Pass B — meta elements declare their parents via `allowedParents`.
 *   Pass C — category-based container inference: `multi-child` /
 *            `absolute-child` containers, plus `Slide`, accept any non-root
 *            builder node as a child (these relationships are not declared
 *            in `children` because the containers use polymorphic child
 *            handling at runtime).
 */
function deriveUsedBy(
  nodes: readonly CompiledNodeDefinition[],
  meta: readonly CompiledMetaDefinition[],
): Map<string, Array<{ parent: string; cardinality: string }>> {
  const result = new Map<
    string,
    Array<{ parent: string; cardinality: string }>
  >();
  const ensure = (tag: string) => {
    let arr = result.get(tag);
    if (!arr) {
      arr = [];
      result.set(tag, arr);
    }
    return arr;
  };

  // Pass A — visual nodes declare children forward (e.g. Ul → Li, Table → Tr).
  for (const parent of nodes) {
    for (const [key, spec] of Object.entries(parent.children ?? {})) {
      const childTag = spec.element ?? key;
      const min = spec.min ?? 0;
      const max = spec.max;
      ensure(childTag).push({
        parent: parent.tag,
        cardinality: formatCardinality(min, max),
      });
    }
  }

  // Pass B — meta elements declare their parents (allowedParents). Reverse
  // direction: a meta tag M with allowedParents=[P1, P2] means P1 and P2
  // are parents of M.
  for (const m of meta) {
    for (const parent of m.allowedParents) {
      ensure(m.tag).push({ parent, cardinality: "0..∞" });
    }
  }

  // Pass C — inferred container relationships. Nodes with container categories
  // (`multi-child`, `absolute-child`) accept any non-root builder node as a
  // child. `Slide` accepts a single body root and is treated the same way for
  // usedBy purposes (cardinality reflects "0..∞" since the page lists possible
  // parents, not occurrence counts). Self-nesting is omitted for readability —
  // a VStack page does not list itself in "Used by".
  const containers = nodes.filter(
    (n) =>
      (n.category !== undefined && isContainerCategory(n.category)) ||
      n.tag === "Slide",
  );
  for (const container of containers) {
    for (const child of nodes) {
      if (child.tag === container.tag) continue; // skip self-nesting
      if (child.root) continue; // SlideGlance/Fragment never appear as children
      if (child.tag === "Slide") continue; // Slide is parent-only, never a child here
      ensure(child.tag).push({ parent: container.tag, cardinality: "0..∞" });
    }
  }

  // Pass D — Slide → SlideGlance is the document-root relationship that
  // SlideGlance does not declare via `children` (SlideGlance is a root node
  // without a `children` spec). Capture it explicitly so the Slide page does
  // not render as "Top-level".
  ensure("Slide").push({ parent: "SlideGlance", cardinality: "1..∞" });

  // Dedupe (parent, cardinality) tuples that could appear across passes.
  for (const [tag, entries] of result) {
    const seen = new Set<string>();
    const unique = entries.filter((e) => {
      const key = `${e.parent}|${e.cardinality}`;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
    result.set(tag, unique);
  }
  return result;
}

function formatCardinality(min: number, max: number | undefined): string {
  if (max === undefined) return min === 0 ? "0..∞" : `${min}..∞`;
  if (min === max) return String(min);
  return `${min}..${max}`;
}
