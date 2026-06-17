/**
 * Element-to-BuilderNode dispatcher.
 *
 * Drives every per-element conversion off the compiled registry
 * (`registry/compiled/index.ts`): attribute existence, deprecation hints,
 * dot-notation expansion, child element routing, and Zod-schema-backed
 * post-processing all flow through `CompiledNodeDefinition.attributes`.
 * `coerceByType` performs the actual string-to-typed-value conversion.
 */

import { bulletNumberTypeSchema } from "../types.ts";
import { ALL_COMPILED_NODES } from "../registry/compiled/index.ts";
import type {
  AttributeSpec,
  CompiledNodeDefinition,
} from "../registry/defineNode.ts";
import {
  coerceBySpec,
  getObjectShape,
  resolveMixedNotationForSpec,
} from "./coerceByType.ts";
import { CHILD_ELEMENT_CONVERTERS } from "./childConverters.ts";
import { expandDotNotation } from "./coerceAttrs.ts";
import { getCurrentDiagnostics } from "./parseContext.ts";
import { applyStylesToAttrs, type StyleRegistry } from "./styles.ts";
import { findClosestMatch, validateLeafNode } from "./validation.ts";
import {
  type XmlElement,
  formatErrorAt,
  getAttributes,
  getChildElements,
  getTagName,
  getTextContent,
  registerSourcePosForElement,
} from "./xml.ts";

// ===== Compiled-registry lookup =====

const NODE_BY_TAG: Record<string, CompiledNodeDefinition> = {};
for (const def of ALL_COMPILED_NODES) {
  NODE_BY_TAG[def.tag] = def;
}

const CONTAINER_TYPES = new Set(["vstack", "hstack", "layer"]);
const TEXT_CONTENT_NODES = new Set(["text", "shape"]);

function getNodeAttributeSpec(
  def: CompiledNodeDefinition,
  attrName: string,
): AttributeSpec | undefined {
  return def.attributes[attrName];
}

/**
 * Convert one XML element to its BuilderNode object representation. Emits
 * recoverable errors into `errors`; returns `null` for unknown tags.
 */
export function convertElement(
  node: XmlElement,
  errors: string[],
  styles: StyleRegistry = {},
): Record<string, unknown> | null {
  const tagName = getTagName(node);
  const def = NODE_BY_TAG[tagName];
  if (!def || !def.type) {
    errors.push(formatErrorAt(node, `Unknown tag: <${tagName}>`));
    return null;
  }
  const nodeId = registerSourcePosForElement(node);
  const attrs = applyStylesToAttrs(
    tagName,
    getAttributes(node),
    styles,
    errors,
    tagName,
    node,
  );
  const childElements = getChildElements(node);
  const textContent = getTextContent(node);

  const result = convertPomNode(
    def,
    attrs,
    childElements,
    textContent,
    errors,
    node,
    styles,
  );
  if (result && nodeId !== undefined) {
    result.__nodeId = nodeId;
  }
  return result;
}

function convertPomNode(
  def: CompiledNodeDefinition,
  attrs: Record<string, string>,
  childElements: XmlElement[],
  textContent: string | undefined,
  errors: string[],
  xmlNode: XmlElement,
  styles: StyleRegistry,
): Record<string, unknown> {
  const tagName = def.tag;
  const nodeType = def.type as string;
  const result: Record<string, unknown> = { type: nodeType };

  // Expand dot-notation attributes (e.g., fill.color="hex" -> { fill: { color: "hex" } }).
  const { regular: regularAttrs, dotGroups } = expandDotNotation(attrs);

  for (const [prefix, subAttrs] of Object.entries(dotGroups)) {
    if (prefix === "type") continue;
    const spec = getNodeAttributeSpec(def, prefix);
    if (spec) {
      result[prefix] = coerceDotGroupForSpec(
        prefix,
        subAttrs,
        spec,
        tagName,
        errors,
      );
    } else {
      const knownAttrs = Object.keys(def.attributes);
      const suggestion = findClosestMatch(prefix, knownAttrs);
      errors.push(
        formatErrorAt(
          xmlNode,
          suggestion
            ? `<${tagName}>: Unknown attribute "${prefix}". Did you mean "${suggestion}"?`
            : `<${tagName}>: Unknown attribute "${prefix}"`,
        ),
      );
    }
  }

  for (const [key, value] of Object.entries(regularAttrs)) {
    if (key === "type") continue;
    // Conflict check: dot-notation and regular attribute for the same key.
    if (key in dotGroups) {
      const specForConflict = getNodeAttributeSpec(def, key);
      if (specForConflict) {
        const resolved = resolveMixedNotationForSpec(value, specForConflict);
        if (resolved.mode === "ignore") continue;
        if (resolved.mode === "merge") {
          result[key] = {
            ...resolved.value,
            ...(result[key] as Record<string, unknown>),
          };
          continue;
        }
      }
      errors.push(
        formatErrorAt(
          xmlNode,
          `<${tagName}>: Attribute "${key}" conflicts with dot-notation attributes (e.g., "${key}.xxx"). Use one or the other, not both`,
        ),
      );
      continue;
    }
    const spec = getNodeAttributeSpec(def, key);
    if (spec) {
      const coerced = coerceBySpec(value, spec);
      if (coerced.error !== null) {
        errors.push(formatErrorAt(xmlNode, `<${tagName}>: ${coerced.error}`));
      } else {
        result[key] = coerced.value;
      }
    } else {
      const knownAttrs = Object.keys(def.attributes);
      const suggestion = findClosestMatch(key, knownAttrs);
      errors.push(
        formatErrorAt(
          xmlNode,
          suggestion
            ? `<${tagName}>: Unknown attribute "${key}". Did you mean "${suggestion}"?`
            : `<${tagName}>: Unknown attribute "${key}"`,
        ),
      );
    }
  }

  // Body text -> `text` property for nodes that support it.
  if (
    textContent !== undefined &&
    TEXT_CONTENT_NODES.has(nodeType) &&
    !("text" in result)
  ) {
    result.text = textContent;
  }

  // Child element notation for complex properties.
  const childConverter = CHILD_ELEMENT_CONVERTERS[nodeType];
  if (childConverter && childElements.length > 0) {
    childConverter(childElements, result, errors, xmlNode, styles);
  } else if (CONTAINER_TYPES.has(nodeType)) {
    // Containers: recursively convert each child. Always set `children` —
    // downstream code iterates blindly and self-closed
    // <VStack/>/<HStack/>/<Layer/> would otherwise crash.
    result.children = childElements
      .map((child) => convertElement(child, errors, styles))
      .filter((c): c is Record<string, unknown> => c !== null);
  } else if (childElements.length > 0) {
    errors.push(
      formatErrorAt(
        xmlNode,
        `<${tagName}>: Unexpected child elements. <${tagName}> does not accept child elements`,
      ),
    );
  }

  applyPostProcessing(nodeType, tagName, result, errors, xmlNode);
  return result;
}

/**
 * Per-node-type post-processing: deprecation warnings, attribute name
 * migrations, and Zod-driven leaf validation.
 */
function applyPostProcessing(
  nodeType: string,
  tagName: string,
  result: Record<string, unknown>,
  errors: string[],
  node: XmlElement,
): void {
  const diagnostics = getCurrentDiagnostics();

  // T49: emit INVALID_NUMBER_TYPE for off-enum <Ol numberType> and strip the
  // value so Zod validation below does not also complain.
  if (nodeType === "ol" && result.numberType !== undefined) {
    const numberTypeCheck = bulletNumberTypeSchema.safeParse(result.numberType);
    if (!numberTypeCheck.success && diagnostics) {
      const rawValue =
        typeof result.numberType === "string" ? result.numberType : "unknown";
      diagnostics.push({
        code: "INVALID_NUMBER_TYPE",
        message: `<Ol numberType="${rawValue}"> is not a valid bullet number type and will be ignored`,
      });
      delete result.numberType;
    }
  }

  // Zod validation for leaf nodes.
  if (!CONTAINER_TYPES.has(nodeType)) {
    validateLeafNode(nodeType, result, errors, tagName, node);
  }

  // Icon: normalize color (prepend "#" when missing).
  if (nodeType === "icon") {
    if (typeof result.color === "string" && !result.color.startsWith("#")) {
      result.color = `#${result.color}`;
    }
    if (
      typeof result.backgroundColor === "string" &&
      !result.backgroundColor.startsWith("#")
    ) {
      result.backgroundColor = `#${result.backgroundColor}`;
    }
  }

  // Svg: normalize color and require a child <svg>.
  if (nodeType === "svg") {
    if (typeof result.color === "string" && !result.color.startsWith("#")) {
      result.color = `#${result.color}`;
    }
    if (result.svgContent === undefined) {
      errors.push("<Svg>: A <svg> child element is required");
    }
  }
}

/**
 * Coerce a dot-notation group (`prefix.subKey="..."`) against an attribute
 * spec. Uses the spec's effective object shape to validate sub-fields and
 * report unknown sub-attributes with Levenshtein suggestions.
 */
function coerceDotGroupForSpec(
  prefix: string,
  subAttrs: Record<string, string>,
  spec: AttributeSpec,
  tagName: string,
  errors: string[],
): Record<string, unknown> {
  const objectShape = getObjectShape(spec);
  const obj: Record<string, unknown> = {};
  if (!objectShape) {
    errors.push(
      `<${tagName}>: Attribute "${prefix}" does not support dot notation`,
    );
    return obj;
  }
  for (const [subKey, subValue] of Object.entries(subAttrs)) {
    const subType = objectShape[subKey];
    if (!subType) {
      const knownSubKeys = Object.keys(objectShape);
      const suggestion = findClosestMatch(subKey, knownSubKeys);
      errors.push(
        `<${tagName}>: Unknown sub-attribute "${prefix}.${subKey}"${suggestion ? `. Did you mean "${prefix}.${suggestion}"?` : ""}`,
      );
      continue;
    }
    const coerced = coerceBySpec(subValue, { coerce: subType });
    if (coerced.error !== null) {
      errors.push(`<${tagName}>: ${prefix}.${subKey}: ${coerced.error}`);
    } else {
      obj[subKey] = coerced.value;
    }
  }
  return obj;
}
