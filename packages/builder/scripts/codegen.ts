#!/usr/bin/env -S node --experimental-strip-types
/**
 * Entry point for `pnpm run codegen`.
 *
 * Generates (at the package root so unpkg surfaces them at
 * `unpkg.com/@slideglance/builder@^0.1/<file>`, matching the
 * `schemaLocation` URL embedded in every `.sgx` template):
 *   builder.xsd          — XML Schema
 *   builder.schema.json  — JSON Schema for BuilderNode
 *   reference.md         — human-readable node reference
 *   .codegen-hash.json   — content hashes for CI verify
 *
 * Flags:
 *   --check    do not write; re-emit, hash, and exit non-zero on drift
 */

import { mkdirSync, readFileSync, writeFileSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { generateXsd } from "../src/codegen/xsd.ts";
import { generateJsonSchemaString } from "../src/codegen/jsonSchema.ts";
import { generateNodesMd } from "../src/codegen/docs.ts";
import { generateReferenceHtml } from "../src/codegen/html.ts";
import { findOrphans, sweepOrphans } from "../src/codegen/orphanSweep.ts";
import {
  buildHashRecord,
  sha256,
  verifyAgainstHashes,
  type CodegenHashes,
} from "../src/codegen/verify.ts";

const HERE = dirname(fileURLToPath(import.meta.url));
const PKG = resolve(HERE, "..");

function buildOutputs(): Record<string, () => string> {
  const html = generateReferenceHtml();
  const htmlEntries: Record<string, () => string> = {};
  for (const [rel, content] of html.entries()) {
    htmlEntries[`reference-html/${rel}`] = () => content;
  }
  return {
    "builder.xsd": () => generateXsd(),
    "builder.schema.json": () => generateJsonSchemaString(),
    // Generated reference published alongside the schema artifacts. The
    // hand-curated `docs/xml-reference.md` (with rich XML samples) remains
    // as the primary user-facing doc.
    "reference.md": () => generateNodesMd(),
    ...htmlEntries,
  };
}

const OUTPUTS = buildOutputs();

function main(): void {
  const isCheck = process.argv.includes("--check");

  // Generate
  const generated: Record<string, string> = {};
  for (const [rel, gen] of Object.entries(OUTPUTS)) {
    generated[rel] = gen();
  }

  if (isCheck) {
    const hashPath = join(PKG, ".codegen-hash.json");
    if (!existsSync(hashPath)) {
      console.error(
        "Codegen --check failed: .codegen-hash.json is missing. Run `pnpm run codegen` and commit.",
      );
      process.exit(1);
    }
    const recorded: CodegenHashes = JSON.parse(readFileSync(hashPath, "utf8"));
    const v = verifyAgainstHashes(generated, recorded);
    if (!v.ok) {
      console.error("Codegen drift detected:");
      for (const d of v.diffs) {
        console.error(`  ${d.file}: ${d.reason}`);
      }
      console.error(
        "Re-run `pnpm run codegen` and commit the regenerated files.",
      );
      process.exit(1);
    }
    // Compare against on-disk content too — hashes alone don't catch a
    // committed-but-stale generated file.
    for (const [rel, expected] of Object.entries(generated)) {
      const onDiskPath = join(PKG, rel);
      if (!existsSync(onDiskPath)) {
        console.error(`Codegen --check: missing on-disk file ${rel}`);
        process.exit(1);
      }
      const onDisk = readFileSync(onDiskPath, "utf8");
      if (sha256(onDisk) !== sha256(expected)) {
        console.error(
          `Codegen --check: ${rel} on-disk content differs from regenerated.`,
        );
        process.exit(1);
      }
    }
    // Orphan sweep (check mode): fail if any stale files live under
    // reference-html/ that are no longer in OUTPUTS.
    const orphans = findOrphans(PKG, generated);
    if (orphans.length > 0) {
      console.error("Codegen --check: orphan reference-html files detected:");
      for (const o of orphans) console.error(`  ${o}`);
      console.error("Re-run `pnpm run codegen` to remove them.");
      process.exit(1);
    }
    console.log("✔ Codegen check passed.");
    return;
  }

  // Write outputs
  for (const [rel, content] of Object.entries(generated)) {
    const abs = join(PKG, rel);
    mkdirSync(dirname(abs), { recursive: true });
    writeFileSync(abs, content);
    console.log(`✔ ${rel} (${content.length} bytes)`);
  }

  // Orphan sweep (write mode): remove files under reference-html/ that are
  // no longer in OUTPUTS, then prune any empty sub-directories.
  sweepOrphans(PKG, generated);

  // Write hash record
  const hashRel = ".codegen-hash.json";
  const hashAbs = join(PKG, hashRel);
  const record = buildHashRecord(generated);
  // Strip generatedAt for stable diffs across runs.
  const stable = { ...record, generatedAt: "" };
  writeFileSync(hashAbs, JSON.stringify(stable, null, 2) + "\n");
  console.log(`✔ ${hashRel}`);
}

main();
