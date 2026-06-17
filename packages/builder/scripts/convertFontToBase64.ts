/**
 * Script that Base64-encodes a font file into a TypeScript module.
 *
 * Usage:
 *   npx tsx scripts/convertFontToBase64.ts
 */

import * as fs from "node:fs";
import * as path from "node:path";

const FONTS_DIR = path.join(import.meta.dirname, "../src/calcYogaLayout/fonts");
const TMP_FONTS_DIR = path.join(import.meta.dirname, "../tmp-fonts");

interface FontConfig {
  inputFile: string;
  outputName: string;
  exportName: string;
}

const fonts: FontConfig[] = [
  {
    inputFile: "NotoSansJP-Regular-min.otf",
    outputName: "notoSansJPRegular.ts",
    exportName: "NOTO_SANS_JP_REGULAR_BASE64",
  },
  {
    inputFile: "NotoSansJP-Bold-min.otf",
    outputName: "notoSansJPBold.ts",
    exportName: "NOTO_SANS_JP_BOLD_BASE64",
  },
  {
    inputFile: "Pretendard-Regular.otf",
    outputName: "pretendardRegular.ts",
    exportName: "PRETENDARD_REGULAR_BASE64",
  },
  {
    inputFile: "Pretendard-Bold.otf",
    outputName: "pretendardBold.ts",
    exportName: "PRETENDARD_BOLD_BASE64",
  },
];

for (const font of fonts) {
  const inputPath = path.join(TMP_FONTS_DIR, font.inputFile);
  const outputPath = path.join(FONTS_DIR, font.outputName);

  // Read the font file and Base64-encode it.
  const fontBuffer = fs.readFileSync(inputPath);
  const base64 = fontBuffer.toString("base64");

  // Generate the TypeScript file.
  const source = font.inputFile.startsWith("Pretendard")
    ? "https://github.com/orioncactus/pretendard"
    : "https://github.com/hiz8/Noto-Sans-CJK-JP.min";
  const description = font.inputFile.startsWith("Pretendard")
    ? `Pretendard font (${font.inputFile})`
    : `Noto Sans CJK JP min font (${font.inputFile})`;

  const content = `/**
 * ${description}
 * License: SIL Open Font License 1.1.
 * Source: ${source}
 */

export const ${font.exportName} = "${base64}";
`;

  fs.writeFileSync(outputPath, content);
  console.log(
    `Generated: ${outputPath} (${Math.round(base64.length / 1024)}KB)`,
  );
}

console.log("\nDone!");
