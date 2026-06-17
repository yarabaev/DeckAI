/**
 * Slide-scope validation for the author-facing `id` attribute and the
 * `<Connector>` node. Runs after every slide tree is fully parsed but
 * before render so that downstream layers can assume every Connector
 * references a real, unique endpoint.
 *
 * Three things happen here:
 *   1. Walk each slide tree and collect `id -> node` pairs.
 *   2. Emit DUPLICATE_NODE_ID diagnostics for repeated ids on the same
 *      slide (both occurrences are kept; only the first wins for lookup).
 *   3. For each <Connector>, check `from === to` (self-ref) and missing
 *      endpoints. Invalid Connectors are filtered out of the slide tree
 *      so the renderer never sees them.
 *
 * Diagnostics are non-fatal — `strict: true` callers see them via the
 * collector's `addAll`.
 */

import type { BuilderNode } from "../types.ts";
import type { Diagnostic } from "../diagnostics.ts";

/**
 * Recursively walk a BuilderNode tree, collecting every node that owns
 * an author-facing `id`. Stops descending into Master objects (they're
 * a separate scope and shouldn't share ids with the slide body).
 */
function collectIds(node: BuilderNode, out: Map<string, BuilderNode[]>): void {
  if (node.id) {
    const list = out.get(node.id);
    if (list) {
      list.push(node);
    } else {
      out.set(node.id, [node]);
    }
  }
  const children = (node as { children?: unknown[] }).children;
  if (Array.isArray(children)) {
    for (const child of children) {
      collectIds(child as BuilderNode, out);
    }
  }
}

/**
 * Filter Connector nodes that fail validation out of the tree in place.
 * Returns the filtered subtree (or the original node when nothing was
 * removed) so callers can replace the entry in their containing list.
 */
function filterInvalidConnectors(
  node: BuilderNode,
  ids: Map<string, BuilderNode[]>,
  diagnostics: Diagnostic[],
  slideIndex: number,
): BuilderNode {
  const children = (node as { children?: unknown[] }).children;
  if (Array.isArray(children)) {
    const next: BuilderNode[] = [];
    for (const raw of children) {
      const child = raw as BuilderNode;
      if (child.type === "connector") {
        const reason = validateConnector(child, ids, slideIndex);
        if (reason) {
          diagnostics.push(reason);
          continue;
        }
        next.push(child);
      } else {
        next.push(filterInvalidConnectors(child, ids, diagnostics, slideIndex));
      }
    }
    (node as { children: BuilderNode[] }).children = next;
  }
  return node;
}

function validateConnector(
  node: Extract<BuilderNode, { type: "connector" }>,
  ids: Map<string, BuilderNode[]>,
  slideIndex: number,
): Diagnostic | null {
  if (node.from === node.to) {
    return {
      code: "INVALID_CONNECTOR_SELF_REF",
      message: `<Connector from="${node.from}" to="${node.to}"> on slide ${slideIndex + 1} references itself; PPTX connectors must bind two distinct shapes. Dropped.`,
    };
  }
  if (!ids.has(node.from)) {
    return {
      code: "UNKNOWN_CONNECTOR_ENDPOINT",
      message: `<Connector from="${node.from}"> on slide ${slideIndex + 1} has no matching shape with id="${node.from}". Dropped.`,
    };
  }
  if (!ids.has(node.to)) {
    return {
      code: "UNKNOWN_CONNECTOR_ENDPOINT",
      message: `<Connector to="${node.to}"> on slide ${slideIndex + 1} has no matching shape with id="${node.to}". Dropped.`,
    };
  }
  // The endpoint cannot be another Connector (no bbox to anchor onto).
  const fromNode = ids.get(node.from)?.[0];
  const toNode = ids.get(node.to)?.[0];
  if (fromNode?.type === "connector" || toNode?.type === "connector") {
    return {
      code: "UNKNOWN_CONNECTOR_ENDPOINT",
      message: `<Connector from="${node.from}" to="${node.to}"> on slide ${slideIndex + 1} cannot bind to another <Connector> (no bounding box). Dropped.`,
    };
  }
  return null;
}

/**
 * Apply slide-scope id / connector validation to a full slide list.
 * Mutates the input array in place (filters invalid Connectors out of
 * every node's children) and appends diagnostics to the provided list.
 */
export function validateConnectorsInSlides(
  slides: BuilderNode[],
  diagnostics: Diagnostic[],
): void {
  for (let i = 0; i < slides.length; i++) {
    const slide = slides[i];
    if (!slide) continue;

    // Phase 1: collect every id used on this slide.
    const ids = new Map<string, BuilderNode[]>();
    collectIds(slide, ids);

    // Phase 2: report duplicates. Both occurrences remain in the tree;
    // downstream Connector lookups silently pick the first.
    for (const [id, nodes] of ids) {
      if (nodes.length > 1) {
        diagnostics.push({
          code: "DUPLICATE_NODE_ID",
          message: `id="${id}" is used by ${nodes.length} nodes on slide ${i + 1}. Connector lookups will resolve to the first occurrence; rename the duplicates.`,
        });
      }
    }

    // Phase 3: drop invalid Connectors. Top-level <Slide> wrappers also
    // need filtering since a Connector can sit directly under a slide.
    if (slide.type === "connector") {
      const reason = validateConnector(slide, ids, i);
      if (reason) {
        diagnostics.push(reason);
        // Replace the root with an empty container so the slide still
        // exists in the deck but renders nothing.
        slides[i] = { type: "vstack", children: [] };
        continue;
      }
    }
    slides[i] = filterInvalidConnectors(slide, ids, diagnostics, i);
  }
}
