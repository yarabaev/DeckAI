import { describe, expect, it, beforeEach, afterEach } from "vitest";
import {
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  existsSync,
  rmSync,
} from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { findOrphans, sweepOrphans } from "./orphanSweep.ts";

describe("orphan sweep", () => {
  let pkgRoot: string;
  beforeEach(() => {
    pkgRoot = mkdtempSync(join(tmpdir(), "orphan-sweep-"));
    mkdirSync(join(pkgRoot, "reference-html", "zombie"), { recursive: true });
    writeFileSync(
      join(pkgRoot, "reference-html", "zombie", "index.html"),
      "stale",
    );
    mkdirSync(join(pkgRoot, "reference-html", "text"), { recursive: true });
    writeFileSync(
      join(pkgRoot, "reference-html", "text", "index.html"),
      "fresh",
    );
  });
  afterEach(() => rmSync(pkgRoot, { recursive: true, force: true }));

  it("findOrphans reports files not in expected set", () => {
    const expected = { "reference-html/text/index.html": "fresh" };
    const result = findOrphans(pkgRoot, expected);
    expect(result).toHaveLength(1);
    expect(result[0]).toContain("zombie/index.html");
  });

  it("sweepOrphans removes orphan files and prunes empty dirs", () => {
    sweepOrphans(pkgRoot, { "reference-html/text/index.html": "fresh" });
    expect(existsSync(join(pkgRoot, "reference-html", "zombie"))).toBe(false);
    expect(
      existsSync(join(pkgRoot, "reference-html", "text", "index.html")),
    ).toBe(true);
  });

  it("sweepOrphans never deletes reference-html root itself", () => {
    sweepOrphans(pkgRoot, { "reference-html/text/index.html": "fresh" });
    expect(existsSync(join(pkgRoot, "reference-html"))).toBe(true);
  });
});
