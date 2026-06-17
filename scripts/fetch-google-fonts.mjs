#!/usr/bin/env node
// Download a curated set of Google Fonts as WOFF2 files into the
// chrome-extension public/assets directory and emit a `@font-face`
// CSS bundle that points at the cached binaries.
//
// Why bundle Google Fonts statically?
// ----------------------------------
// PPTX decks exported from Google Slides routinely embed Google Fonts
// (Anton, Alata, Roboto Condensed, …). Those embedded payloads are
// frequently MicroType Express compressed — a format we don't decode
// — so the renderer drops the embedded face and the browser falls
// back to whatever similar typeface the host system happens to ship
// (Helvetica Neue / Arial). Wider-than-authored metrics blow line
// wraps apart.
//
// Bundling the upstream Google Fonts WOFF2 binaries makes the fonts
// available to the browser via @font-face the moment the viewer
// loads. The `font-family: Anton` chain the renderer emits then
// resolves to the bundled face, and both rendering and canvas
// measurement use the right metrics. No network at runtime — the
// extension is self-contained per the offline-first requirement.

import { mkdirSync, writeFileSync, existsSync } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, "..");
const ASSETS_DIR = resolve(
  REPO_ROOT,
  "apps/chrome-extension/public/assets/google-fonts",
);
const CSS_OUTPUT = resolve(
  REPO_ROOT,
  "apps/chrome-extension/public/assets/google-fonts.css",
);

// Curated list — roughly the top Google Fonts seen in PPTX exports
// from Google Slides + Canva + popular templates. Kept compact (~40)
// to keep the extension under ~1.5 MB total (woff2 is ~30–60 KB
// per Latin face). Append entries as decks surface new authored
// faces; the metric catalog (`font_metric.rs`) should mirror this
// list so wrap measurement stays accurate even when a face fails
// to load (subset / network / CSP edge cases).
const FAMILIES = [
  // Display / condensed (most affected by fallback substitution)
  "Anton",
  "Alata",
  "Bebas Neue",
  "Oswald",
  "Roboto Condensed",
  "Archivo Narrow",
  "Archivo Black",
  "PT Sans Narrow",
  // Body sans (Google Slides defaults + popular)
  "Lato",
  "Roboto",
  "Open Sans",
  "Montserrat",
  "Poppins",
  "Inter",
  "Source Sans Pro",
  "Source Sans 3",
  "Nunito",
  "Nunito Sans",
  "Work Sans",
  "DM Sans",
  "Mulish",
  "Karla",
  "Quicksand",
  "Rubik",
  "Outfit",
  "Manrope",
  "Raleway",
  "Cabin",
  "PT Sans",
  // Serif (editorial)
  "Playfair Display",
  "Merriweather",
  "Lora",
  "PT Serif",
  "Crimson Text",
  "EB Garamond",
  "Source Serif Pro",
  // Monospace
  "Roboto Mono",
  "Source Code Pro",
  "Fira Code",
  "JetBrains Mono",
  // Script / display
  "Pacifico",
  "Dancing Script",
  "Caveat",
];

// Use a UA string the Google Fonts CSS API recognises as
// woff2-capable. Chrome 100+ is a safe modern baseline; older UAs
// receive TTF, which is ~3× larger.
const UA =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15";

mkdirSync(ASSETS_DIR, { recursive: true });

/**
 * Slugify a family name into a filesystem-safe filename stem.
 * `"Roboto Condensed"` -> `"roboto-condensed"`.
 */
function slugify(name) {
  return name.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
}

/**
 * Fetch the Google Fonts CSS for a single family + weight, then
 * download every `src: url(...) format('woff2')` referenced in the
 * stylesheet. Returns the @font-face declarations rewritten to point
 * at the local cached binaries.
 */
async function downloadFamily(family) {
  const slug = slugify(family);
  const cssUrl = `https://fonts.googleapis.com/css2?family=${encodeURIComponent(family)}&display=swap`;
  const cssRes = await fetch(cssUrl, { headers: { "User-Agent": UA } });
  if (!cssRes.ok) {
    throw new Error(`CSS fetch failed for ${family}: ${cssRes.status}`);
  }
  const css = await cssRes.text();
  // Each @font-face block carries one src URL. Walk every block,
  // download the binary, and rewrite the src to point at the local
  // path. The output CSS is what `<link rel="stylesheet">` consumes.
  const blocks = css.split(/(?=@font-face\s*\{)/);
  const rewritten = [];
  for (let i = 0; i < blocks.length; i++) {
    const block = blocks[i];
    if (!block.includes("@font-face")) continue;
    const srcMatch = block.match(/src:\s*url\((https:\/\/[^)]+)\)\s*format\('([^']+)'\)/);
    if (!srcMatch) {
      rewritten.push(block);
      continue;
    }
    const upstreamUrl = srcMatch[1];
    const fmt = srcMatch[2];
    const ext = fmt === "woff2" ? "woff2" : fmt === "woff" ? "woff" : "ttf";
    const localFilename = `${slug}-${i}.${ext}`;
    const localPath = join(ASSETS_DIR, localFilename);
    if (!existsSync(localPath)) {
      const binRes = await fetch(upstreamUrl, { headers: { "User-Agent": UA } });
      if (!binRes.ok) {
        throw new Error(`Binary fetch failed for ${upstreamUrl}: ${binRes.status}`);
      }
      const buf = Buffer.from(await binRes.arrayBuffer());
      writeFileSync(localPath, buf);
      console.log(`  ↳ ${localFilename} (${(buf.byteLength / 1024).toFixed(1)} KB)`);
    } else {
      console.log(`  ↳ ${localFilename} (cached)`);
    }
    // Bundled CSS uses relative URLs so the same file works whether
    // the chrome-extension serves it from a `chrome-extension://`
    // origin or a raw http server (used in dev / tests).
    const localUrl = `./google-fonts/${localFilename}`;
    rewritten.push(
      block.replace(srcMatch[0], `src: url(${localUrl}) format('${fmt}')`),
    );
  }
  return rewritten.join("");
}

const cssParts = [
  "/* Auto-generated by scripts/fetch-google-fonts.mjs — do not edit by hand. */",
  "/* Run `node scripts/fetch-google-fonts.mjs` to refresh. */",
  "",
];

let total = 0;
let failed = 0;
for (const family of FAMILIES) {
  console.log(`Fetching ${family}…`);
  try {
    const css = await downloadFamily(family);
    cssParts.push(css);
    total += 1;
  } catch (err) {
    console.warn(`  ✖ ${family}: ${err.message}`);
    failed += 1;
  }
}

writeFileSync(CSS_OUTPUT, cssParts.join("\n"));
console.log(
  `\nFetched ${total}/${FAMILIES.length} families (${failed} failed) → ${CSS_OUTPUT}`,
);
