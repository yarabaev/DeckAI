import fs from "fs";
import path from "path";
import {
  FILES,
  OUTPUT_DIR,
  EXPECTED_DIR,
  ACTUAL_DIR,
  DIFF_DIR,
  PAGE_NAMES,
} from "./config.js";
import { generatePptx } from "./generatePptx.js";
import { pptxToPng } from "../lib/pptxToPng.js";
import { comparePng } from "../lib/comparePng.js";

async function main() {
  const updateBaseline = process.argv.includes("--update");

  // Create the output directory.
  fs.mkdirSync(OUTPUT_DIR, { recursive: true });
  fs.mkdirSync(ACTUAL_DIR, { recursive: true });
  fs.mkdirSync(DIFF_DIR, { recursive: true });

  console.log("=== VRT: Visual Regression Testing ===\n");

  // 1. Generate PPTX.
  console.log("1. Generating PPTX...");
  await generatePptx(FILES.actualPptx);

  // 2. Convert each PPTX page to PNG.
  console.log("2. Converting PPTX to PNG...");
  await pptxToPng(FILES.actualPptx, ACTUAL_DIR, PAGE_NAMES);

  // With --update, refresh the baseline.
  if (updateBaseline) {
    console.log("3. Updating baseline...");
    fs.mkdirSync(EXPECTED_DIR, { recursive: true });
    for (const name of PAGE_NAMES) {
      const src = path.join(ACTUAL_DIR, `${name}.png`);
      const dst = path.join(EXPECTED_DIR, `${name}.png`);
      fs.copyFileSync(src, dst);
    }
    console.log(`\nBaseline updated: ${EXPECTED_DIR}`);
    return;
  }

  // 3. Compare the per-page images.
  console.log("3. Comparing images...");

  if (!fs.existsSync(EXPECTED_DIR)) {
    console.error(`\nExpected directory not found: ${EXPECTED_DIR}`);
    console.error("Run with --update to create baseline");
    process.exit(1);
  }

  const failedPages: { name: string; diffPixels: number }[] = [];

  console.log("\nResults:");
  for (const name of PAGE_NAMES) {
    const actualPath = path.join(ACTUAL_DIR, `${name}.png`);
    const expectedPath = path.join(EXPECTED_DIR, `${name}.png`);
    const diffPath = path.join(DIFF_DIR, `${name}.png`);

    if (!fs.existsSync(expectedPath)) {
      console.log(`  ? ${name} (baseline not found)`);
      failedPages.push({ name, diffPixels: -1 });
      continue;
    }

    const diffPixels = comparePng(actualPath, expectedPath, diffPath);

    if (diffPixels > 0) {
      console.log(`  x ${name} (${diffPixels} pixels differ)`);
      failedPages.push({ name, diffPixels });
    } else {
      // Remove the diff image when there's no delta.
      if (fs.existsSync(diffPath)) {
        fs.unlinkSync(diffPath);
      }
      console.log(`  o ${name}`);
    }
  }

  if (failedPages.length > 0) {
    console.error(
      `\nFAILED: ${failedPages.length} of ${PAGE_NAMES.length} pages differ.`,
    );
    console.error(`Diff images saved in: ${DIFF_DIR}`);
    process.exit(1);
  }

  console.log(
    `\nNo visual differences found. (${PAGE_NAMES.length} pages checked)`,
  );
}

main().catch((err) => {
  console.error(err instanceof Error ? err.message : String(err));
  process.exit(1);
});
