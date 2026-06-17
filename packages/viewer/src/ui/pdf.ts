/**
 * PDF export — pure-browser, zero-dependency.
 *
 * Pipeline: each already-rendered slide SVG → `<canvas>` → JPEG bytes
 * → embedded into a minimal hand-rolled PDF 1.4 byte stream.
 *
 * Why hand-rolled instead of pulling in `pdf-lib` or `jsPDF`?
 *   The viewer has to keep working as a drop-in browser plugin with
 *   no install-time tooling. A user dropping the bundle into a static
 *   site shouldn't have to add a peer dependency just to export PDFs.
 *
 * Why JPEG instead of PNG?
 *   PDF supports both, but PNG embedding requires deflate-compressed
 *   `FlateDecode` streams. Producing a deflate stream from scratch
 *   means shipping a zlib polyfill or running a wasm gzip; JPEG just
 *   uses `DCTDecode` (the JPEG bit-stream is dropped in literally),
 *   which is what the browser's `canvas.toBlob('image/jpeg')` already
 *   produces.
 */

import type { SlideSvg } from "../types.js";

export interface ExportPdfOptions {
  /** Already-rendered slides (with media URLs already rewritten to data URLs). */
  slides: SlideSvg[];
  /** Output width per slide in pixels (height scales). Defaults to 1920. */
  width?: number;
  /** JPEG quality 0..1. Defaults to 0.9. */
  quality?: number;
  /**
   * Progress hook invoked once per phase tick. Hosts can mirror this
   * into a status bar to keep the UI feeling alive during a large
   * export — without a yield point the rasterize-loop blocks the
   * main thread for seconds on 100+ slide decks.
   */
  onProgress?: (info: {
    phase: "rasterize" | "encode";
    current: number;
    total: number;
  }) => void;
}

/**
 * Returns a `Uint8Array` containing a multi-page PDF — one slide per
 * page, each rendered as a JPEG. Throws when a slide SVG cannot be
 * rasterized.
 */
export async function exportToPdf(
  options: ExportPdfOptions,
): Promise<Uint8Array> {
  if (options.slides.length === 0) {
    throw new Error("exportToPdf: no slides to export");
  }
  const targetWidth = options.width ?? 1920;
  const quality = options.quality ?? 0.9;
  const total = options.slides.length;
  const onProgress = options.onProgress ?? (() => {});

  const pages: Array<{ jpeg: Uint8Array; width: number; height: number }> = [];
  for (let i = 0; i < options.slides.length; i += 1) {
    onProgress({ phase: "rasterize", current: i, total });
    const page = await rasterizeSvgToJpeg(
      options.slides[i].svg,
      targetWidth,
      quality,
    );
    if (page) pages.push(page);
    // Yield to the event loop so the browser can repaint progress UI
    // and stay responsive to user input. Without this the entire
    // rasterize-loop runs in a single microtask chain and blocks the
    // main thread until the last slide is done. `requestAnimationFrame`
    // would also work but yields slower (~16 ms); a 0-ms timeout
    // releases the event loop more aggressively which is what we
    // want for a 100+ slide deck.
    await new Promise<void>((resolve) => setTimeout(resolve, 0));
  }
  if (pages.length === 0) {
    throw new Error("exportToPdf: no pages could be rasterized");
  }
  onProgress({ phase: "encode", current: total, total });
  // Yield once more before the final byte-stream stitch — for a 100+
  // slide deck `buildPdf` itself can take 100+ ms because of the
  // string concatenation overhead.
  await new Promise<void>((resolve) => setTimeout(resolve, 0));
  return buildPdf(pages);
}

/**
 * Rasterize an SVG string to JPEG bytes using the browser's native
 * `Image` + `<canvas>` pipeline. Width is fixed; height scales to
 * preserve the SVG's aspect ratio (parsed from `viewBox`).
 */
async function rasterizeSvgToJpeg(
  svg: string,
  targetWidth: number,
  quality: number,
): Promise<{ jpeg: Uint8Array; width: number; height: number } | null> {
  const aspect = parseSvgAspect(svg);
  const w = targetWidth;
  const h = Math.round(targetWidth / aspect);

  const blob = new Blob([svg], { type: "image/svg+xml;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  try {
    const img = await loadImage(url);
    const canvas = document.createElement("canvas");
    canvas.width = w;
    canvas.height = h;
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("Canvas 2D context unavailable");
    // White background — JPEG has no alpha, so an unfilled canvas would
    // show as black where the slide had transparency.
    ctx.fillStyle = "#ffffff";
    ctx.fillRect(0, 0, w, h);
    ctx.drawImage(img, 0, 0, w, h);
    const jpegBlob = await canvasToBlob(canvas, "image/jpeg", quality);
    const arrayBuf = await jpegBlob.arrayBuffer();
    return { jpeg: new Uint8Array(arrayBuf), width: w, height: h };
  } finally {
    URL.revokeObjectURL(url);
  }
}

function parseSvgAspect(svg: string): number {
  const m =
    /viewBox\s*=\s*"([^"]+)"/i.exec(svg) ??
    /viewBox\s*=\s*'([^']+)'/i.exec(svg);
  if (m) {
    const parts = m[1].split(/\s+/).map(Number);
    const w = parts[2];
    const h = parts[3];
    if (w && h && w > 0 && h > 0) return w / h;
  }
  // PowerPoint default 16:9.
  return 16 / 9;
}

function loadImage(url: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error("Failed to load SVG into <img>"));
    img.src = url;
  });
}

function canvasToBlob(
  canvas: HTMLCanvasElement,
  type: string,
  quality: number,
): Promise<Blob> {
  return new Promise((resolve, reject) => {
    canvas.toBlob(
      (blob) => {
        if (blob) resolve(blob);
        else reject(new Error("canvas.toBlob() returned null"));
      },
      type,
      quality,
    );
  });
}

/**
 * Build a minimal PDF 1.4 document from a list of JPEG-encoded pages.
 *
 * Object layout (1-indexed object numbers):
 *   1  /Catalog
 *   2  /Pages
 *   3..N      odd  → /Page
 *   3..N      even → /XObject (the JPEG image)
 *   N..       even → content stream
 *
 * For simplicity we group each page as 3 sequential objects:
 *   page i → page object, image XObject, content stream.
 */
function buildPdf(
  pages: Array<{ jpeg: Uint8Array; width: number; height: number }>,
): Uint8Array {
  const enc = new TextEncoder();
  const chunks: Array<Uint8Array | string> = [];
  const offsets: number[] = []; // byte offset of each object (1-indexed: offsets[1] = first object)
  let cursor = 0;

  const pushString = (s: string): void => {
    chunks.push(s);
    cursor += enc.encode(s).length;
  };
  const pushBytes = (b: Uint8Array): void => {
    chunks.push(b);
    cursor += b.length;
  };
  const beginObject = (n: number): void => {
    offsets[n] = cursor;
    pushString(`${n} 0 obj\n`);
  };
  const endObject = (): void => {
    pushString("\nendobj\n");
  };

  // PDF header. The binary marker is required so transports that
  // sniff the first bytes don't mangle the file as text.
  pushString("%PDF-1.4\n");
  pushBytes(new Uint8Array([0x25, 0xe2, 0xe3, 0xcf, 0xd3, 0x0a]));

  // Reserve object numbers up front so we can cross-reference them.
  // Catalog=1, Pages=2, then per page: page=3+3i, image=4+3i, content=5+3i
  const catalogId = 1;
  const pagesId = 2;
  const pageIds: number[] = [];
  const imageIds: number[] = [];
  const contentIds: number[] = [];
  for (let i = 0; i < pages.length; i += 1) {
    pageIds.push(3 + i * 3);
    imageIds.push(4 + i * 3);
    contentIds.push(5 + i * 3);
  }

  // 1: catalog
  beginObject(catalogId);
  pushString(`<< /Type /Catalog /Pages ${pagesId} 0 R >>`);
  endObject();

  // 2: pages tree
  beginObject(pagesId);
  pushString(
    `<< /Type /Pages /Count ${pages.length} /Kids [${pageIds.map((id) => `${id} 0 R`).join(" ")}] >>`,
  );
  endObject();

  // Per-page triplet
  for (let i = 0; i < pages.length; i += 1) {
    const { jpeg, width, height } = pages[i];

    // Page
    beginObject(pageIds[i]);
    pushString(
      `<< /Type /Page /Parent ${pagesId} 0 R ` +
        `/MediaBox [0 0 ${width} ${height}] ` +
        `/Resources << /XObject << /Im${i} ${imageIds[i]} 0 R >> /ProcSet [/PDF /ImageC] >> ` +
        `/Contents ${contentIds[i]} 0 R >>`,
    );
    endObject();

    // Image XObject — JPEG bit-stream embedded directly via DCTDecode.
    beginObject(imageIds[i]);
    pushString(
      `<< /Type /XObject /Subtype /Image /Width ${width} /Height ${height} ` +
        `/ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length ${jpeg.length} >>\n` +
        `stream\n`,
    );
    pushBytes(jpeg);
    pushString(`\nendstream`);
    endObject();

    // Content stream — draw the image filling the page.
    const op = `q ${width} 0 0 ${height} 0 0 cm /Im${i} Do Q`;
    const opBytes = enc.encode(op);
    beginObject(contentIds[i]);
    pushString(`<< /Length ${opBytes.length} >>\nstream\n`);
    pushBytes(opBytes);
    pushString(`\nendstream`);
    endObject();
  }

  // xref
  const xrefOffset = cursor;
  const objectCount = 2 + pages.length * 3;
  pushString(`xref\n0 ${objectCount + 1}\n`);
  pushString("0000000000 65535 f \n");
  for (let i = 1; i <= objectCount; i += 1) {
    const off = offsets[i] ?? 0;
    pushString(`${off.toString().padStart(10, "0")} 00000 n \n`);
  }

  // trailer
  pushString(
    `trailer\n<< /Size ${objectCount + 1} /Root ${catalogId} 0 R >>\nstartxref\n${xrefOffset}\n%%EOF\n`,
  );

  // Concatenate.
  const totalLen = chunks.reduce(
    (sum, c) => sum + (typeof c === "string" ? enc.encode(c).length : c.length),
    0,
  );
  const out = new Uint8Array(totalLen);
  let p = 0;
  for (const c of chunks) {
    if (typeof c === "string") {
      const b = enc.encode(c);
      out.set(b, p);
      p += b.length;
    } else {
      out.set(c, p);
      p += c.length;
    }
  }
  return out;
}
