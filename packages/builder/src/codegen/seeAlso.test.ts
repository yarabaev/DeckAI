import { describe, expect, it } from "vitest";
import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { SEE_ALSO } from "./seeAlso.ts";
import { walkRegistry } from "./walkRegistry.ts";

describe("SEE_ALSO curation", () => {
  it("is a frozen record of arrays", () => {
    expect(typeof SEE_ALSO).toBe("object");
    for (const [tag, entries] of Object.entries(SEE_ALSO)) {
      expect(Array.isArray(entries), `${tag} entries`).toBe(true);
      for (const e of entries) {
        expect(typeof e.label).toBe("string");
        expect(typeof e.href).toBe("string");
      }
    }
  });

  it("every key matches an actual registry tag", () => {
    const r = walkRegistry();
    const tags = new Set([
      ...r.nodes.map((n) => n.tag),
      ...r.meta.map((m) => m.tag),
    ]);
    for (const tag of Object.keys(SEE_ALSO)) {
      expect(tags.has(tag), `unknown SEE_ALSO tag: ${tag}`).toBe(true);
    }
  });

  it("in-repo href paths point to existing files", () => {
    const HERE = fileURLToPath(import.meta.url);
    const REPO_ROOT = resolve(HERE, "../../../../..");
    const inRepoRegex =
      /^https:\/\/github\.com\/SlideGlance\/slideglance\/(?:blob|tree)\/main\/(.+?)(?:#.*)?$/;
    for (const [tag, entries] of Object.entries(SEE_ALSO)) {
      for (const { href } of entries) {
        const m = href.match(inRepoRegex);
        if (!m) continue; // external URL — skipped, see design §4.3 limitation
        const repoPath = resolve(REPO_ROOT, m[1]);
        expect(existsSync(repoPath), `${tag} href missing: ${m[1]}`).toBe(true);
      }
    }
  });
});
