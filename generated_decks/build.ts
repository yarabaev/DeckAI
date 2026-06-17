// Build script for generated decks.
//
// Run from the workspace root:
//   pnpm --filter @slideglance/generated-decks run build -- deck-1
//
// Run from this directory:
//   pnpm run build -- deck-1
//
// Flags:
//   --out <file>   Write to a custom .pptx path.
//   --size WxH     Fallback slide size for decks without <Document size="">.
//   --font <file>  Add a TTF/OTF font file to layout-time text measurement.
//   --no-lint      Skip the post-build lint pass.
//   --no-fonts     Do not auto-load generated_decks/fonts.

import { promises as fs, readFileSync } from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";
import { buildPptx, type ImportResolver } from "@slideglance/builder";

interface CliOptions {
  deckDirArg?: string;
  lintEnabled: boolean;
  fontsEnabled: boolean;
  fontPathArgs: string[];
  outPathArg?: string;
  slideSize: { w: number; h: number };
}

const here = path.dirname(fileURLToPath(import.meta.url));
const defaultSlideSize = { w: 1280, h: 720 };
const defaultFontsDir = path.join(here, "fonts");
const supportedFontFilePattern = /\.(otf|ttf|ttc)$/i;
const unsupportedMeasurementStylePattern =
  /(^|[-_\s])(italic|oblique)([-_\s.]|$)/i;

function usage(): string {
  return [
    "Usage: pnpm --filter @slideglance/generated-decks run build -- <deck-folder> [flags]",
    "",
    "Examples:",
    "  pnpm --filter @slideglance/generated-decks run build -- deck-1",
    "  pnpm --filter @slideglance/generated-decks run build -- generated_decks/deck-1",
    "  pnpm --dir generated_decks run build -- deck-1 --out deck-1/restoration.pptx",
    "  pnpm exec tsx generated_decks/build.ts generated_decks/deck-1",
    "",
    "Flags:",
    "  --out <file>   Write to a custom .pptx path.",
    "  --size WxH     Fallback slide size, default 1280x720.",
    "  --font <file>  Add a TTF/OTF/TTC font file to layout-time text measurement. Repeatable.",
    "  --no-fonts     Do not auto-load generated_decks/fonts.",
    "  --no-lint      Skip the post-build lint pass.",
    "  -h, --help     Show this help.",
  ].join("\n");
}

function parseSlideSize(value: string): { w: number; h: number } {
  const match = /^(\d+(?:\.\d+)?)x(\d+(?:\.\d+)?)$/i.exec(value.trim());
  if (!match) {
    throw new Error(
      `Invalid --size value "${value}". Expected format: 1280x720`,
    );
  }

  return { w: Number(match[1]), h: Number(match[2]) };
}

function parseArgs(argv: string[]): CliOptions {
  const options: CliOptions = {
    fontsEnabled: true,
    fontPathArgs: [],
    lintEnabled: true,
    slideSize: defaultSlideSize,
  };

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--") {
      continue;
    }
    if (arg === "-h" || arg === "--help") {
      console.log(usage());
      process.exit(0);
    }
    if (arg === "--no-lint") {
      options.lintEnabled = false;
      continue;
    }
    if (arg === "--no-fonts") {
      options.fontsEnabled = false;
      continue;
    }
    if (arg === "--font") {
      const value = argv[i + 1];
      if (!value) {
        throw new Error("--font requires a font file path");
      }
      options.fontPathArgs.push(value);
      i += 1;
      continue;
    }
    if (arg === "--out" || arg === "--output") {
      const value = argv[i + 1];
      if (!value) {
        throw new Error(`${arg} requires a file path`);
      }
      options.outPathArg = value;
      i += 1;
      continue;
    }
    if (arg === "--size") {
      const value = argv[i + 1];
      if (!value) {
        throw new Error("--size requires a value like 1280x720");
      }
      options.slideSize = parseSlideSize(value);
      i += 1;
      continue;
    }
    if (arg.startsWith("-")) {
      throw new Error(`Unknown flag: ${arg}`);
    }
    if (options.deckDirArg) {
      throw new Error(`Unexpected extra argument: ${arg}`);
    }
    options.deckDirArg = arg;
  }

  return options;
}

function isInside(parent: string, child: string): boolean {
  const relative = path.relative(parent, child);
  return (
    relative === "" ||
    (!relative.startsWith("..") && !path.isAbsolute(relative))
  );
}

function uniqPaths(paths: string[]): string[] {
  return [...new Set(paths.map((item) => path.resolve(item)))];
}

async function resolveDeckDir(arg: string | undefined): Promise<string> {
  if (!arg) {
    throw new Error(`Deck folder argument is required.\n\n${usage()}`);
  }

  const candidates = path.isAbsolute(arg)
    ? [arg]
    : uniqPaths([
        path.resolve(process.cwd(), arg),
        path.resolve(here, arg),
        path.resolve(here, "..", arg),
      ]);

  for (const candidate of candidates) {
    const entry = path.join(candidate, "main.sgx");
    try {
      const stat = await fs.stat(entry);
      if (stat.isFile()) {
        const resolved = path.resolve(candidate);
        if (!isInside(here, resolved)) {
          throw new Error(
            `Deck folder must be inside ${here}: ${path.relative(process.cwd(), resolved)}`,
          );
        }
        return resolved;
      }
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code !== "ENOENT") {
        throw err;
      }
    }
  }

  throw new Error(
    `Cannot find main.sgx in "${arg}". Pass a folder such as "deck-1" or "generated_decks/deck-1".`,
  );
}

function resolveOutputPath(
  deckDir: string,
  outPathArg: string | undefined,
): string {
  if (!outPathArg) {
    return path.join(deckDir, `${path.basename(deckDir)}.pptx`);
  }

  const candidates = path.isAbsolute(outPathArg)
    ? [outPathArg]
    : uniqPaths([
        path.resolve(process.cwd(), outPathArg),
        path.resolve(here, outPathArg),
        path.resolve(here, "..", outPathArg),
      ]);
  const absolute = candidates.find((candidate) => isInside(here, candidate));
  if (!absolute) {
    throw new Error(
      `Output path must be inside ${here}: ${path.relative(process.cwd(), candidates[0])}`,
    );
  }
  return absolute;
}

function resolvePathInsideGeneratedDecks(
  inputPath: string,
  label: string,
): string {
  const candidates = path.isAbsolute(inputPath)
    ? [inputPath]
    : uniqPaths([
        path.resolve(process.cwd(), inputPath),
        path.resolve(here, inputPath),
        path.resolve(here, "..", inputPath),
      ]);
  const absolute = candidates.find((candidate) => isInside(here, candidate));
  if (!absolute) {
    throw new Error(
      `${label} must be inside ${here}: ${path.relative(process.cwd(), candidates[0])}`,
    );
  }
  return absolute;
}

async function collectFontFiles(dir: string): Promise<string[]> {
  try {
    const entries = await fs.readdir(dir, { withFileTypes: true });
    return entries
      .filter(
        (entry) => entry.isFile() && supportedFontFilePattern.test(entry.name),
      )
      .map((entry) => path.join(dir, entry.name))
      .sort((a, b) => a.localeCompare(b));
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === "ENOENT") {
      return [];
    }
    throw err;
  }
}

async function loadFonts(options: CliOptions): Promise<Uint8Array[]> {
  const defaultFontFiles = options.fontsEnabled
    ? await collectFontFiles(defaultFontsDir)
    : [];
  const explicitFontFiles = options.fontPathArgs.map((fontPath) =>
    resolvePathInsideGeneratedDecks(fontPath, "Font path"),
  );
  const fontFiles = uniqPaths([...defaultFontFiles, ...explicitFontFiles]);
  const measurementFontFiles: string[] = [];
  const skippedFontFiles: string[] = [];

  for (const fontFile of fontFiles) {
    if (unsupportedMeasurementStylePattern.test(path.basename(fontFile))) {
      skippedFontFiles.push(fontFile);
      continue;
    }
    measurementFontFiles.push(fontFile);
  }

  if (measurementFontFiles.length === 0) {
    if (skippedFontFiles.length > 0) {
      console.log(
        `  fonts: 0 loaded, ${skippedFontFiles.length} italic/oblique skipped for layout measurement`,
      );
    }
    return [];
  }

  const fonts = await Promise.all(
    measurementFontFiles.map(async (fontFile) => {
      const bytes = await fs.readFile(fontFile);
      return new Uint8Array(bytes);
    }),
  );

  console.log(
    `  fonts: loaded ${measurementFontFiles.length} for layout measurement (${measurementFontFiles.map((fontFile) => path.relative(here, fontFile)).join(", ")})`,
  );
  if (skippedFontFiles.length > 0) {
    console.log(
      `  fonts: skipped ${skippedFontFiles.length} italic/oblique file(s) for layout measurement (${skippedFontFiles.map((fontFile) => path.relative(here, fontFile)).join(", ")})`,
    );
  }

  return fonts;
}

function makeImportResolver(deckDir: string): ImportResolver {
  return (src, fromPath) => {
    const baseDir = fromPath ? path.dirname(fromPath) : deckDir;
    const absolute = path.resolve(baseDir, src);
    if (!isInside(deckDir, absolute)) {
      throw new Error(
        `Import "${src}" escapes the deck folder ${path.relative(here, deckDir)}`,
      );
    }
    return { content: readFileSync(absolute, "utf8"), path: absolute };
  };
}

function printDiagnostics(
  label: string,
  diagnostics: readonly {
    severity?: string;
    code?: string;
    message: string;
    sourcePos?: { file?: string; line?: number; column?: number };
  }[],
): void {
  if (diagnostics.length === 0) {
    return;
  }

  console.log(`  ${label}:`);
  for (const d of diagnostics) {
    const pos = d.sourcePos?.line
      ? ` ${d.sourcePos.file ? path.relative(here, d.sourcePos.file) : ""}:${d.sourcePos.line}`
      : "";
    console.log(
      `    [${d.severity ?? "warn"}] [${d.code ?? "UNKNOWN"}]${pos} ${d.message}`,
    );
  }
}

async function main(): Promise<void> {
  const options = parseArgs(process.argv.slice(2));
  const deckDir = await resolveDeckDir(options.deckDirArg);
  const entry = path.join(deckDir, "main.sgx");
  const outPath = resolveOutputPath(deckDir, options.outPathArg);
  const slug = path.relative(here, deckDir);

  console.log(
    `Building ${slug}${options.lintEnabled ? "" : " (lint disabled)"}...`,
  );

  const fonts = await loadFonts(options);
  const xml = await fs.readFile(entry, "utf8");
  const { pptx, diagnostics, lintReport } = await buildPptx(
    xml,
    options.slideSize,
    {
      textMeasurement: "auto",
      sourcePath: entry,
      resolveImport: makeImportResolver(deckDir),
      equalize: true,
      lint: { enabled: options.lintEnabled, ruleset: "recommended" },
      fonts,
    },
  );

  printDiagnostics("diagnostics", diagnostics);
  if (lintReport) {
    const s = lintReport.summary;
    console.log(`  lint: ${s.error} error · ${s.warn} warn · ${s.info} info`);
    printDiagnostics("lint diagnostics", lintReport.diagnostics);
  } else {
    console.log("  lint: skipped (--no-lint)");
  }

  const bytes = (await pptx.write({ outputType: "uint8array" })) as Uint8Array;
  await fs.mkdir(path.dirname(outPath), { recursive: true });
  await fs.writeFile(outPath, bytes);
  console.log(
    `  wrote ${path.relative(here, outPath)} (${bytes.byteLength} bytes)`,
  );

  const hasDiagnostics = diagnostics.some((d) => d.severity !== "info");
  const hasLintIssues =
    lintReport !== undefined &&
    (lintReport.summary.error > 0 || lintReport.summary.warn > 0);
  if (hasDiagnostics || hasLintIssues) {
    process.exitCode = 1;
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
