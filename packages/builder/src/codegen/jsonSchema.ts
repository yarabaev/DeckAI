/**
 * JSON Schema generator — emits a schema describing the *BuilderNode shape*
 * (post-coercion). Useful for tools that consume BuilderNode trees as JSON
 * (e.g. Monaco editor, AI tools that produce structured node trees).
 *
 * For XML-input schema use the XSD instead — these are different artefacts
 * because the BuilderNode shape (e.g. `padding: { top: 10 }`) does not match the
 * raw XML attribute encoding (`padding="10"` or `padding.top="10"`).
 */

import { z } from "zod";
import { walkRegistry } from "./walkRegistry.ts";
import { BUILDER_NAMESPACE_URI } from "./xsd.ts";

interface JsonSchemaOutput {
  $schema: string;
  $id: string;
  title: string;
  description: string;
  oneOf: Array<{ $ref: string }>;
  $defs: Record<string, unknown>;
}

export function generateJsonSchema(): JsonSchemaOutput {
  const registry = walkRegistry();

  const $defs: Record<string, unknown> = {};
  const oneOf: Array<{ $ref: string }> = [];

  for (const node of registry.nodes) {
    // Skip document containers — they don't have a Zod schema and aren't
    // BuilderNode entries.
    if (!node.schema || node.type === undefined) continue;

    try {
      const schema = z.toJSONSchema(node.schema, {
        target: "draft-2020-12",
      });
      $defs[node.tag] = schema;
      oneOf.push({ $ref: `#/$defs/${node.tag}` });
    } catch (err) {
      // Recursive types (vstack/hstack/layer have lazy children) may need
      // special handling. We capture and continue, leaving them out of the
      // top-level union; codegen verify will surface this.
      throw new Error(
        `JSON Schema generation failed for <${node.tag}>: ${(err as Error).message}`,
        { cause: err },
      );
    }
  }

  return {
    $schema: "https://json-schema.org/draft/2020-12/schema",
    $id: `${BUILDER_NAMESPACE_URI}/builder.schema.json`,
    title: "BuilderNode",
    description:
      "Post-coercion shape of a single slide builder node. The top level " +
      "union lists every leaf node type. Container nodes " +
      "(vstack/hstack/layer) and document containers (Slide/SlideGlance) " +
      "are described separately.",
    oneOf,
    $defs,
  };
}

/** Stringify with stable, indented output. */
export function generateJsonSchemaString(): string {
  return JSON.stringify(generateJsonSchema(), null, 2) + "\n";
}
