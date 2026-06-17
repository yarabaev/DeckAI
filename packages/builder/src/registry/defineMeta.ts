import type { z } from "zod";
import type { AttributeSpec } from "./defineNode.ts";

/** Content model of a meta element. Drives XSD generation only. */
type MetaContentModel =
  /** Allows any builder node child. e.g. <Template> body. */
  | "any"
  /** Only <Slot> children. e.g. <Use>. */
  | "slots"
  /** Only <Template> children. e.g. <Templates>. */
  | "templates"
  /** Only <Style> children. e.g. <Styles>. */
  | "styles"
  /** No children. e.g. <Import>, <Slot> placeholder. */
  | "none";

interface MetaSpec {
  /** XML tag. Examples: "Template", "Use", "Slot", "Import", "Templates", "Styles", "Style". */
  tag: string;
  /** Human-readable description. */
  description: string;
  /** Whether the element is a valid document root. */
  root?: boolean;
  /** XML attribute spec — same shape as a regular node. */
  attributes?: Record<string, AttributeSpec>;
  /**
   * When true, the element accepts arbitrary additional attributes beyond
   * those in `attributes` (an untyped attribute bag). Used by `<Style>`, whose
   * attributes are stored verbatim and validated by the element that opts in
   * via `class="..."`. Drives an `xs:anyAttribute` in the generated XSD.
   */
  openAttributes?: boolean;
  /** Where in the tree the element may appear (for XSD). */
  contentModel: MetaContentModel;
  /** Allowed parent element tags (XSD positioning hint). */
  allowedParents?: readonly string[];
  /** Optional example XML. */
  example?: string;
  /** Optional Zod schema for the resulting object (rarely useful). */
  schema?: z.ZodTypeAny;
}

export interface CompiledMetaDefinition extends Required<
  Pick<MetaSpec, "tag" | "description" | "contentModel">
> {
  attributes: Record<string, AttributeSpec>;
  openAttributes: boolean;
  allowedParents: readonly string[];
  root: boolean;
  example?: string;
  schema?: z.ZodTypeAny;
  readonly kind: "meta";
}

export function defineMeta(spec: MetaSpec): CompiledMetaDefinition {
  if (!spec.tag) throw new Error("defineMeta: `tag` is required");
  if (!spec.description) {
    throw new Error(`defineMeta(${spec.tag}): \`description\` is required`);
  }

  return {
    tag: spec.tag,
    description: spec.description,
    contentModel: spec.contentModel,
    attributes: spec.attributes ?? {},
    openAttributes: spec.openAttributes ?? false,
    allowedParents: spec.allowedParents ?? [],
    root: spec.root ?? false,
    example: spec.example,
    schema: spec.schema,
    kind: "meta",
  };
}
