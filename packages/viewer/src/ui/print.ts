/**
 * Print mode — open a new window containing one SVG-per-page so the
 * browser's print dialog can produce a PDF or paper printout.
 *
 * This deliberately avoids the heavier `pdf-lib` round-trip provided
 * by {@link exportToPdf}; it leaves typography rendering up to
 * the browser engine.
 */

import type { SlideSvg } from "../types.js";

export interface PrintOptions {
  /** Document title shown by the browser's print dialog. */
  title?: string;
  /**
   * Progress hook invoked while slides are inserted into the print
   * window. Without a yield point a 100+ slide deck stalls the main
   * thread for seconds; the host can mirror this into a status bar
   * so the UI keeps feeling alive.
   */
  onProgress?: (info: {
    phase: "layout" | "open-dialog";
    current: number;
    total: number;
  }) => void;
}

/**
 * Open a new window containing each slide as a full-page SVG and
 * trigger the browser's print dialog. Returns the opened window
 * reference (or `null` when popups are blocked).
 *
 * Page orientation is auto-detected from the slide aspect ratio so
 * portrait decks (e.g. A4-portrait reports rendered as PPTX) print
 * upright instead of being squeezed into landscape paper.
 */
export async function printDeck(
  slides: SlideSvg[],
  options: PrintOptions = {},
): Promise<Window | null> {
  if (slides.length === 0) return null;
  const win = window.open("", "_blank", "width=1024,height=768");
  if (!win) return null;
  const orientation = detectOrientation(slides);
  const onProgress = options.onProgress ?? (() => {});
  const total = slides.length;

  const doc = win.document;
  doc.open();
  doc.write("<!doctype html><html><head>");
  doc.write(
    `<title>${escapeHtml(options.title ?? "slideglance slides")}</title>`,
  );
  // `size: <orientation>` lets the user keep their paper-size choice
  // (A4, Letter, …) from the print dialog while the browser picks the
  // matching orientation. Forcing `landscape` here would override a
  // portrait deck and clip the slide thumbnails.
  doc.write(`<style>
    @page { margin: 0; size: ${orientation}; }
    html, body { margin: 0; padding: 0; background: #fff; color: #000; }
    .pptx-page { page-break-after: always; display: flex; align-items: center; justify-content: center; height: 100vh; padding: 24px; box-sizing: border-box; }
    .pptx-page svg { max-width: 100%; max-height: 100%; }
    .pptx-page:last-child { page-break-after: auto; }
  </style>`);
  doc.write("</head><body>");
  doc.close();

  for (let i = 0; i < slides.length; i += 1) {
    onProgress({ phase: "layout", current: i, total });
    const slide = slides[i];
    const wrapper = doc.createElement("div");
    wrapper.className = "pptx-page";
    const parser = new DOMParser();
    const parsed = parser.parseFromString(slide.svg, "image/svg+xml");
    if (
      parsed.documentElement &&
      parsed.documentElement.tagName.toLowerCase() !== "parsererror"
    ) {
      wrapper.appendChild(doc.importNode(parsed.documentElement, true));
    }
    doc.body.appendChild(wrapper);
    // Yield so the host's progress UI can repaint between slides; on a
    // 100+ slide deck the importNode/appendChild loop blocks the main
    // thread long enough that an overlay never gets a chance to render.
    await new Promise<void>((resolve) => setTimeout(resolve, 0));
  }
  onProgress({ phase: "open-dialog", current: total, total });
  // Defer the print() call so the browser has a chance to lay out the
  // newly-mounted SVGs.
  win.setTimeout(() => win.print(), 50);
  return win;
}

function detectOrientation(slides: SlideSvg[]): "portrait" | "landscape" {
  for (const slide of slides) {
    const aspect = parseSvgAspect(slide.svg);
    if (aspect != null) return aspect >= 1 ? "landscape" : "portrait";
  }
  // Fall back to landscape — the dominant PowerPoint default.
  return "landscape";
}

function parseSvgAspect(svg: string): number | null {
  const m =
    /viewBox\s*=\s*"([^"]+)"/i.exec(svg) ??
    /viewBox\s*=\s*'([^']+)'/i.exec(svg);
  if (m) {
    const parts = m[1].split(/\s+/).map(Number);
    const w = parts[2];
    const h = parts[3];
    if (w && h && w > 0 && h > 0) return w / h;
  }
  return null;
}

function escapeHtml(s: string): string {
  return s
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}
