import type { BuilderNode } from "../types.ts";
import type { NodeDefinition } from "./types.ts";

type NodeType = BuilderNode["type"];

const registry = new Map<NodeType, NodeDefinition>();

export function registerNode(def: NodeDefinition): void {
  if (registry.has(def.type)) {
    throw new Error(`Duplicate node registration: ${def.type}`);
  }
  registry.set(def.type, def);
}

export function getNodeDef(type: NodeType): NodeDefinition {
  const def = registry.get(type);
  if (!def) throw new Error(`Unknown node type: ${type}`);
  return def;
}
