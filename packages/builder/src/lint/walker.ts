import type { PositionedNode } from "../types.ts";

/** Depth-first walk yielding every positioned node, with parent + path. */
export function* walk(
  root: PositionedNode,
  parent: PositionedNode | null = null,
  path: PositionedNode[] = [],
): Generator<{
  node: PositionedNode;
  parent: PositionedNode | null;
  path: readonly PositionedNode[];
}> {
  yield { node: root, parent, path };
  const kids = getChildren(root);
  if (kids) {
    const nextPath = [...path, root];
    for (const c of kids) yield* walk(c, root, nextPath);
  }
}

export function getChildren(
  node: PositionedNode,
): readonly PositionedNode[] | null {
  if (
    "children" in node &&
    Array.isArray((node as { children?: unknown }).children)
  ) {
    return (node as { children: PositionedNode[] }).children;
  }
  return null;
}

/** A simple stable id for a node — file:line if known, else node-type path. */
export function nodeIdOf(
  node: PositionedNode,
  path: readonly PositionedNode[],
): string {
  const sp = node as { __sourceLine?: number; __sourceFile?: string };
  if (sp.__sourceFile && sp.__sourceLine !== undefined) {
    return `${sp.__sourceFile}:${sp.__sourceLine}`;
  }
  return [...path.map((p) => p.type), node.type].join(" > ");
}

export function nodeSourcePos(
  node: PositionedNode,
): { file?: string; line?: number } | undefined {
  const sp = node as { __sourceLine?: number; __sourceFile?: string };
  if (sp.__sourceLine === undefined && !sp.__sourceFile) return undefined;
  return { line: sp.__sourceLine, file: sp.__sourceFile };
}

/** True if a node carries text content (Text/Ul/Ol/Shape with text). */
export function isTextBearing(node: PositionedNode): boolean {
  return (
    node.type === "text" ||
    node.type === "ul" ||
    node.type === "ol" ||
    (node.type === "shape" &&
      typeof (node as { text?: string }).text === "string")
  );
}
