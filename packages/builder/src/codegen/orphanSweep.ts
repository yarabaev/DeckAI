/**
 * Orphan-file sweep for reference-html/ codegen output. See design §9.2.
 */
import { readdirSync, rmSync, rmdirSync, existsSync } from "node:fs";
import { join, relative } from "node:path";

export function findOrphans(
  pkgRoot: string,
  generated: Record<string, unknown>,
): string[] {
  const expected = new Set(Object.keys(generated).map((k) => join(pkgRoot, k)));
  const refRoot = join(pkgRoot, "reference-html");
  if (!existsSync(refRoot)) return [];
  const result: string[] = [];
  const entries = readdirSync(refRoot, {
    recursive: true,
    withFileTypes: true,
  });
  for (const ent of entries) {
    if (!ent.isFile()) continue;
    const parent =
      (ent as { parentPath?: string; path?: string }).parentPath ??
      (ent as { path?: string }).path ??
      refRoot;
    const abs = join(parent, ent.name);
    if (!expected.has(abs)) result.push(abs);
  }
  return result;
}

export function sweepOrphans(
  pkgRoot: string,
  generated: Record<string, unknown>,
): void {
  for (const orphan of findOrphans(pkgRoot, generated)) {
    rmSync(orphan);
    console.log(`✔ swept orphan: ${orphan}`);
  }
  const refRoot = join(pkgRoot, "reference-html");
  pruneEmptyDirsBounded(refRoot, refRoot);
}

function pruneEmptyDirsBounded(root: string, current: string): void {
  if (!existsSync(current)) return;
  const entries = readdirSync(current, { withFileTypes: true });
  for (const ent of entries) {
    if (ent.isDirectory()) {
      pruneEmptyDirsBounded(root, join(current, ent.name));
    }
  }
  // Never prune the root itself; only sub-directories.
  if (relative(root, current) === "") return;
  if (readdirSync(current).length === 0) {
    rmdirSync(current);
  }
}
