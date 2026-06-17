#!/usr/bin/env node
//
// Cross-platform wrapper around `scripts/build-wasm.sh`. JS package
// `prebuild` / `predev` scripts invoke this wrapper so the wasm
// artefacts are always fresh before vite reads them.
//
// The bash script itself short-circuits when nothing under `crates/`
// changed since the last successful run — so this wrapper costs only
// a `find` walk on no-op rebuilds.
//
// Why a Node wrapper instead of calling bash directly:
// - npm `prebuild` runs in the package's working directory, but the
//   target script lives at the workspace root. A relative path works
//   only when the CWD is predictable; this wrapper resolves the
//   workspace root from its own location, so it's invariant to where
//   the consuming `pnpm build` was invoked from.
// - It centralises error handling — a failed wasm build aborts the
//   whole pnpm script chain with a clear message instead of leaving a
//   stale `dist/` for vite to pick up silently.
// - It propagates the parent's `PROFILE` / `FORCE` env vars so
//   `PROFILE=dev pnpm build` still does what the developer expects.

import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const scriptPath = resolve(__dirname, "build-wasm.sh");

// CLI flags that produce env-var equivalents — on Windows / cmd.exe
// the `PROFILE=dev pnpm build` shell prefix doesn't work, so the
// dev / force lanes are also exposed as plain CLI flags.
const args = process.argv.slice(2);
const env = { ...process.env };
if (args.includes("--dev")) env.PROFILE = "dev";
if (args.includes("--force")) env.FORCE = "1";

const result = spawnSync("bash", [scriptPath], {
  stdio: "inherit",
  env,
});

if (result.error) {
  console.error(`[run-build-wasm] failed to spawn bash: ${result.error.message}`);
  process.exit(1);
}
process.exit(result.status ?? 1);
