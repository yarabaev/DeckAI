import type { z } from "zod";
import type { BuilderNode, PositionedNode } from "../types.ts";
import type { Node as YogaNode } from "yoga-layout";
import type { RenderContext } from "../renderPptx/types.ts";
import type { BuildContext } from "../buildContext.ts";
import type { LayoutResultMap } from "../calcYogaLayout/types.ts";
import type { Yoga, NodeCategory } from "./types.ts";

/**
 * Coercion type for an XML attribute value.
 *
 * Codegen uses this to map an attribute to its XSD simpleType. The runtime
 * parser (Phase C) will use the same enum to dispatch string -> typed value.
 * For Session 1 the runtime path is unchanged; this enum exists only for
 * codegen / future use.
 */
export type CoerceType =
  | "string"
  | "number"
  | "boolean"
  | "json"
  | "length"
  | "color"
  | "padding"
  | "border"
  | "fill"
  | "shadow"
  | "underline"
  | "imageSizing"
  | "alignSelf"
  | "alignItems"
  | "justifyContent"
  | "flexWrap"
  | "positionType"
  | "shapeType"
  | "bulletNumberType"
  | "iconName"
  | "iconVariant"
  | "iconColor"
  | "textAlign"
  | "vAlign"
  | "lineArrow"
  | "borderDash";

/** Specification of a single XML attribute on a node. */
export interface AttributeSpec {
  /** Coercion type — drives XSD attribute type and runtime dispatch. */
  coerce: CoerceType;
  /** Documentation string surfaced in nodes.md and (where supported) XSD annotations. */
  doc?: string;
  /** Whether the attribute is required. */
  required?: boolean;
  /**
   * If true, the element body (text content) maps to this attribute when
   * the element has no other children. e.g. `<Text>foo</Text>` -> `text: "foo"`.
   */
  bodyAlias?: boolean;
  /** Enum members when coerce is "string" but with a fixed value set. */
  enum?: readonly string[];
  /**
   * Marks this attribute as belonging to a dot-notation group root, so codegen
   * knows that `prefix.subKey="..."` forms also produce sub-fields. The actual
   * sub-field schema comes from the Zod node schema.
   */
  dotNotation?: boolean;
  /**
   * Sub-field shape for object-typed attributes. Populated only when
   * `coerce` cannot fully describe the structure on its own — typically
   * `coerce: "json"` attributes that nonetheless support dot-notation
   * (e.g., `backgroundImage`, `connectorStyle`). For `coerce: "padding" |
   * "border" | "fill" | "shadow" | "underline" | "imageSizing" | "lineArrow"`
   * the structure is implied by the type itself; this field stays empty.
   */
  objectShape?: Record<string, CoerceType>;
}

/**
 * Specification of a child element family on a node.
 *
 * `element` is the XML tag name of allowed children. `min`/`max` define
 * cardinality (defaults: 0..unbounded). When `field` is set, matching
 * children collect into that BuilderNode field; otherwise the field is named
 * after the children-spec key.
 */
export interface ChildrenSpec {
  element: string;
  min?: number;
  max?: number;
  doc?: string;
  /**
   * Whether the children are *builder nodes* (any subtree, recursively converted)
   * or a fixed shape (handled by a node-specific custom converter today).
   */
  kind?: "node" | "structured";
}

/** Specification supplied by the author when defining a node. */
export interface NodeSpec {
  /** XML tag (PascalCase). Examples: "Text", "Image", "Slide". */
  tag: string;
  /** BuilderNode `type` discriminant. Examples: "text", "image". For containers without a BuilderNode type, omit. */
  type?: BuilderNode["type"];
  /** Human-readable description (one to two sentences). Surfaces in docs and XSD annotation. */
  description: string;
  /** Node category — drives default child handling at runtime. Omit for meta-only declarations. */
  category?: NodeCategory;
  /**
   * Zod schema describing the *shape* of the resulting BuilderNode (post-coercion).
   * When provided, codegen runs `z.toJSONSchema(schema)` and emits TS via z.infer.
   * For container nodes (SlideGlance/Slide) the schema may be omitted.
   */
  schema?: z.ZodTypeAny;
  /** XML attribute spec. */
  attributes?: Record<string, AttributeSpec>;
  /** XML child element spec. */
  children?: Record<string, ChildrenSpec>;
  /** Whether this element is a valid document root. */
  root?: boolean;
  /** Optional example XML for docs. */
  example?: string;

  // Runtime hooks — same shape as the existing NodeDefinition. Optional so a
  // declaration can be codegen-only.
  applyYogaStyle?: (
    node: BuilderNode,
    yn: YogaNode,
    yoga: Yoga,
    ctx: BuildContext,
  ) => void | Promise<void>;
  toPositioned?: (
    pom: BuilderNode,
    absoluteX: number,
    absoluteY: number,
    layout: { width: number; height: number },
    ctx: BuildContext,
    map: LayoutResultMap,
  ) => PositionedNode | Promise<PositionedNode>;
  render?: (node: PositionedNode, ctx: RenderContext) => void;
  collectImageSources?: (node: BuilderNode) => string[];
}

/** Compiled definition consumed by registry / codegen. */
export interface CompiledNodeDefinition extends Required<
  Pick<NodeSpec, "tag" | "description">
> {
  type?: BuilderNode["type"];
  category?: NodeCategory;
  schema?: z.ZodTypeAny;
  attributes: Record<string, AttributeSpec>;
  children: Record<string, ChildrenSpec>;
  root: boolean;
  example?: string;
  applyYogaStyle?: NodeSpec["applyYogaStyle"];
  toPositioned?: NodeSpec["toPositioned"];
  render?: NodeSpec["render"];
  collectImageSources?: NodeSpec["collectImageSources"];
  /** Marker — distinguishes from CompiledMetaDefinition. */
  readonly kind: "node";
}

/**
 * Validate and normalize a node specification.
 *
 * Throws (Error) on internal inconsistencies that should never reach a
 * production build — e.g. a `bodyAlias: true` attribute typed as something
 * other than string. Codegen also runs additional cross-node consistency
 * checks (duplicate tags, dangling child element references).
 */
export function defineNode(spec: NodeSpec): CompiledNodeDefinition {
  if (!spec.tag) throw new Error("defineNode: `tag` is required");
  if (!spec.description) {
    throw new Error(`defineNode(${spec.tag}): \`description\` is required`);
  }

  const attributes = spec.attributes ?? {};
  const children = spec.children ?? {};

  // bodyAlias must target a string-ish attribute.
  for (const [name, attr] of Object.entries(attributes)) {
    if (attr.bodyAlias && attr.coerce !== "string") {
      throw new Error(
        `defineNode(${spec.tag}): attribute "${name}" has bodyAlias:true but coerce is "${attr.coerce}" (must be "string")`,
      );
    }
  }

  return {
    tag: spec.tag,
    type: spec.type,
    description: spec.description,
    category: spec.category,
    schema: spec.schema,
    attributes,
    children,
    root: spec.root ?? false,
    example: spec.example,
    applyYogaStyle: spec.applyYogaStyle,
    toPositioned: spec.toPositioned,
    render: spec.render,
    collectImageSources: spec.collectImageSources,
    kind: "node",
  };
}
