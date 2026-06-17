import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { ICON_DATA } from "./iconData.ts";

// Load @resvg/resvg-wasm lazily.
// Prevent webpack (Next.js) static analysis from tracing the .wasm file.
// Load createRequire dynamically and build the module name via string concatenation.
type ResvgWasm = typeof import("@resvg/resvg-wasm");
const RESVG_PKG = ["@resvg", "resvg-wasm"].join("/");
let resvgModule: ResvgWasm | undefined;
let wasmInitPromise: Promise<void> | undefined;

/**
 * Resolve the path to the WASM binary.
 * In bundled environments (esbuild), reference index_bg.wasm in the same directory;
 * In non-bundled environments, resolve via `createRequire` from `node_modules`.
 */
function resolveWasmPath(): string {
  const dir = dirname(fileURLToPath(import.meta.url));
  const localPath = join(dir, "index_bg.wasm");
  if (existsSync(localPath)) return localPath;
  const require = createRequire(import.meta.url);
  return require.resolve(`${RESVG_PKG}/index_bg.wasm`);
}

/**
 * Initialize the WASM module and return the Resvg class.
 * Safe for concurrent invocation (the Promise itself is cached).
 */
function ensureWasmInitialized(): Promise<void> {
  if (!wasmInitPromise) {
    wasmInitPromise = (async () => {
      const req = createRequire(import.meta.url);
      const mod = req(RESVG_PKG) as ResvgWasm;
      const wasmPath = resolveWasmPath();
      const wasmBuffer = await readFile(wasmPath);
      await mod.initWasm(wasmBuffer);
      resvgModule = mod;
    })();
  }
  return wasmInitPromise;
}

function getResvg() {
  if (!resvgModule) throw new Error("WASM not initialized");
  return resvgModule.Resvg;
}

function buildIconSvg(name: string, size: number, color: string): string {
  const pathData = ICON_DATA[name];
  if (!pathData) {
    throw new Error(`Unknown icon name: "${name}"`);
  }
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="${color}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${pathData}</svg>`;
}

export async function rasterizeIcon(
  name: string,
  size: number,
  color: string,
  cache: Map<string, string>,
): Promise<string> {
  const key = `${name}|${size}|${color}`;
  const cached = cache.get(key);
  if (cached) return cached;

  await ensureWasmInitialized();
  const Resvg = getResvg();
  const svg = buildIconSvg(name, size, color);
  const resvg = new Resvg(svg, { fitTo: { mode: "width", value: size } });
  const pngData = resvg.render();
  const pngBuffer = pngData.asPng();
  const result = `image/png;base64,${Buffer.from(pngBuffer).toString("base64")}`;
  cache.set(key, result);
  return result;
}

/**
 * Rasterize an inline SVG string at the requested size and return a Base64 PNG.
 * When `color` is specified, set the `stroke` / `fill` attributes on the SVG root.
 */
export async function rasterizeSvgContent(
  svgContent: string,
  width: number,
  color: string | undefined,
  cache: Map<string, string>,
  height?: number,
): Promise<string> {
  const h = height ?? width;
  const key = `svg:${svgContent}|${width}|${h}|${color ?? ""}`;
  const cached = cache.get(key);
  if (cached) return cached;

  // Set xmlns / width / height on the SVG, and inject stroke / fill when color is given.
  let svg = svgContent;

  // Add xmlns when missing.
  if (!svg.includes("xmlns")) {
    svg = svg.replace("<svg", '<svg xmlns="http://www.w3.org/2000/svg"');
  }

  // Override width / height.
  svg = svg.replace(/<svg([^>]*)>/, (_match, attrs: string) => {
    let newAttrs = attrs
      .replace(/\bwidth\s*=\s*"[^"]*"/g, "")
      .replace(/\bheight\s*=\s*"[^"]*"/g, "");
    newAttrs += ` width="${width}" height="${h}"`;

    // When color is specified, set stroke / fill (matches preset icons).
    if (color) {
      if (!attrs.includes("stroke=")) {
        newAttrs += ` stroke="${color}"`;
      }
      if (!attrs.includes("fill=")) {
        newAttrs += ` fill="none"`;
      }
    }

    return `<svg${newAttrs}>`;
  });

  await ensureWasmInitialized();
  const Resvg = getResvg();
  const resvg = new Resvg(svg, { fitTo: { mode: "width", value: width } });
  const pngData = resvg.render();
  const pngBuffer = pngData.asPng();
  const result = `image/png;base64,${Buffer.from(pngBuffer).toString("base64")}`;
  cache.set(key, result);
  return result;
}
