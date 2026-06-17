/**
 * Codegen output hash + verification.
 *
 * `pnpm run codegen` writes hashes to `.codegen-hash.json`. CI runs codegen
 * with `--check` which re-emits and diffs against the committed files.
 */

import { createHash } from "node:crypto";

export interface CodegenHashes {
  /** ISO timestamp of when the hash file was written. */
  generatedAt: string;
  /** Source code revision (best-effort) — can be empty when unavailable. */
  sourceRev?: string;
  /** Hash per generated file (relative path -> sha256). */
  files: Record<string, string>;
}

export function sha256(content: string): string {
  return createHash("sha256").update(content).digest("hex");
}

export function buildHashRecord(files: Record<string, string>): CodegenHashes {
  const hashes: Record<string, string> = {};
  for (const k of Object.keys(files).sort()) {
    hashes[k] = sha256(files[k]!);
  }
  return {
    generatedAt: new Date().toISOString(),
    files: hashes,
  };
}

interface VerifyResult {
  ok: boolean;
  diffs: Array<{ file: string; reason: string }>;
}

export function verifyAgainstHashes(
  current: Record<string, string>,
  recorded: CodegenHashes,
): VerifyResult {
  const diffs: VerifyResult["diffs"] = [];
  for (const [file, content] of Object.entries(current)) {
    const got = sha256(content);
    const want = recorded.files[file];
    if (!want) diffs.push({ file, reason: "missing in recorded hash" });
    else if (got !== want)
      diffs.push({ file, reason: `hash mismatch (${got} vs ${want})` });
  }
  for (const file of Object.keys(recorded.files)) {
    if (!(file in current)) diffs.push({ file, reason: "no longer generated" });
  }
  return { ok: diffs.length === 0, diffs };
}
