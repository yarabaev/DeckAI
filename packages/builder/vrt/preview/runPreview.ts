// Baseline-less VRT pipeline: build the editable sample deck (sample.ts),
// rasterize it via the shared LibreOffice-backed converter, and write
// `output/sample.png` so a developer (or Claude Code) can visually
// inspect a layout / rendering change without maintaining an expected
// baseline. Sister to vrt/decks/runVrt.ts.

import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

import { pptxToPng } from "../lib/pptxToPng.js";
import { buildSamplePptx } from "./sample.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const OUTPUT_DIR = path.join(__dirname, "output");

async function main() {
  fs.mkdirSync(OUTPUT_DIR, { recursive: true });

  console.log("=== Preview: PPTX to PNG ===\n");

  const pptxPath = path.join(OUTPUT_DIR, "sample.pptx");
  console.log("1. Building PPTX from vrt/preview/sample.ts...");
  await buildSamplePptx(pptxPath);

  console.log("2. Converting PPTX to PNG...");
  await pptxToPng(pptxPath, OUTPUT_DIR, ["sample"]);

  console.log(`\nPreview saved: ${path.join(OUTPUT_DIR, "sample.png")}`);
}

main().catch((err) => {
  console.error(err instanceof Error ? err.message : String(err));
  process.exit(1);
});
