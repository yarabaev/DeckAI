import { execSync } from "child_process";
import fs from "fs";
import path from "path";

/**
 * Use LibreOffice to convert PPTX to PDF, then ImageMagick to convert each page to PNG.
 */
export async function pptxToPng(
  pptxPath: string,
  outputDir: string,
  pageNames: readonly string[],
): Promise<void> {
  const tempDir = path.join(path.dirname(pptxPath), ".pptx-temp");

  // Create a temporary directory.
  if (fs.existsSync(tempDir)) {
    fs.rmSync(tempDir, { recursive: true });
  }
  fs.mkdirSync(tempDir, { recursive: true });

  // Create the output directory.
  fs.mkdirSync(outputDir, { recursive: true });

  try {
    // 1. LibreOffice converts the PPTX to a PDF (all pages included).
    execSync(
      `soffice --headless --convert-to pdf --outdir "${tempDir}" "${pptxPath}"`,
      { stdio: "inherit" },
    );

    // Get the PDF file path.
    const pptxBasename = path.basename(pptxPath, path.extname(pptxPath));
    const pdfPath = path.join(tempDir, `${pptxBasename}.pdf`);

    if (!fs.existsSync(pdfPath)) {
      throw new Error(`PDF file not generated: ${pdfPath}`);
    }

    // 2. ImageMagick converts each PDF page into a PNG.
    const pngPrefix = path.join(tempDir, "page");
    execSync(
      `convert -density 150 -strip "${pdfPath}" "${pngPrefix}-%03d.png"`,
      { stdio: "inherit" },
    );

    // Read the generated PNG files (sorted by page order).
    const pngFiles = fs
      .readdirSync(tempDir)
      .filter((f) => f.startsWith("page-") && f.endsWith(".png"))
      .sort();

    if (pngFiles.length === 0) {
      throw new Error("No PNG pages generated from PDF");
    }

    if (pngFiles.length !== pageNames.length) {
      throw new Error(
        `Page count mismatch: generated ${pngFiles.length} pages, expected ${pageNames.length}`,
      );
    }

    // 3. Copy each page's PNG into the output directory.
    for (const [i, pngFile] of pngFiles.entries()) {
      const src = path.join(tempDir, pngFile);
      const dst = path.join(outputDir, `${pageNames[i]!}.png`);
      fs.copyFileSync(src, dst);
    }
  } finally {
    // Delete the temporary directory.
    if (fs.existsSync(tempDir)) {
      fs.rmSync(tempDir, { recursive: true });
    }
  }
}
