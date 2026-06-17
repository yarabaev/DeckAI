/**
 * Web Worker-backed `SlideController`.
 *
 * Spawns the bundled `pptx-worker.ts` chunk (or accepts a
 * pre-instantiated `Worker` for callers that need to share a worker
 * pool / inject a custom URL) and routes `open` / `render` / `close`
 * messages over a small monotonic-id correlation protocol.
 */

import type { MediaBlob, SlideController, TypefaceUsage } from "./types.js";

/**
 * Wire form of `TypefaceUsage` — snake_case keys preserved from the
 * WASM serde output. `worker-controller` translates to the camelCase
 * `TypefaceUsage` shape the rest of the viewer consumes.
 */
interface WireTypefaceUsage {
  requested: string;
  fallback_chain: string[];
  resolved_family: string | null;
}

interface PendingHandlers {
  resolve: (value: unknown) => void;
  reject: (error: unknown) => void;
}

/**
 * Build a Worker-backed `SlideController`. The worker imports
 * `@slideglance/core/bundler`, parses the deck into a `PptxDocument`, and
 * renders one slide per `render` message with `externalMedia` enabled.
 */
export async function createWorkerController(
  workerOverride?: Worker,
): Promise<SlideController> {
  let worker: Worker;
  if (workerOverride) {
    worker = workerOverride;
  } else {
    // Vite recognises `?worker&url` and emits a sibling chunk,
    // returning its public URL as a string — so consumer bundlers can
    // co-locate the worker alongside their app bundle without us
    // having to hardcode paths.
    const { default: workerUrl } =
      (await import("./pptx-worker.ts?worker&url")) as { default: string };
    worker = new Worker(workerUrl, { type: "module" });
  }

  let nextId = 0;
  const pending = new Map<number, PendingHandlers>();

  worker.addEventListener("message", (ev) => {
    const msg = ev.data as { type: string; id: number } & Record<
      string,
      unknown
    >;
    const handler = pending.get(msg.id);
    if (!handler) return;
    pending.delete(msg.id);
    if (msg.type === "error") {
      handler.reject(new Error(String(msg.message ?? "worker error")));
    } else {
      handler.resolve(msg);
    }
  });
  worker.addEventListener("error", (ev) => {
    for (const handler of pending.values()) {
      handler.reject(
        new Error(String((ev as ErrorEvent).message ?? "worker crashed")),
      );
    }
    pending.clear();
  });

  function call<T>(
    payload: Record<string, unknown>,
    transfer: Transferable[] = [],
  ): Promise<T> {
    const id = ++nextId;
    return new Promise<T>((resolve, reject) => {
      pending.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
      });
      worker.postMessage({ ...payload, id }, transfer);
    });
  }

  return {
    async open(bytes: Uint8Array, options?: { extraFontDefsCss?: string }) {
      const result = await call<{
        slideCount: number;
        fontDefs: string;
        fontUsage?: WireTypefaceUsage[];
        fontLoadFailures?: Array<{ family: string; reason: string }>;
        decodedFonts?: Array<{
          family: string;
          weight: string;
          style: string;
          bytes: Uint8Array;
        }>;
      }>(
        {
          type: "open",
          bytes,
          // Optional host-supplied stylesheet whose `@font-face` rules
          // are loaded into the worker's `FontFaceSet` alongside the
          // deck's embedded fonts. The chrome-extension uses this to
          // ship its bundled Google Fonts so that decks referring to
          // (e.g.) `Anton` resolve to the bundled face during the
          // worker's canvas measurement — without the extra defs the
          // measurer falls back to a generic OS sans-serif and produces
          // wider lines than the browser's eventual paint will use.
          extraFontDefsCss: options?.extraFontDefsCss,
        },
        // Transfer the underlying buffer so the worker doesn't pay a
        // 200MB+ copy cost on large decks.
        [bytes.buffer as ArrayBuffer],
      );
      const fontUsage: TypefaceUsage[] = (result.fontUsage ?? []).map((u) => ({
        requested: u.requested,
        fallbackChain: u.fallback_chain,
        resolvedFamily: u.resolved_family,
      }));
      // Embedded-font load failures are a known, bounded class
      // (MTX-compressed payloads OTS rejects; fonts the deck doesn't
      // grant the embedding bit on; etc.) and the metric-match
      // fallback chain plus bundled Google Fonts cover the visible
      // paint. We deliberately do NOT console.warn here — hosts that
      // want to surface failure detail (e.g. a status-bar indicator)
      // read the structured `fontLoadFailures` array directly.
      const fontLoadFailures = result.fontLoadFailures ?? [];
      return {
        slideCount: result.slideCount,
        fontDefs: result.fontDefs,
        fontUsage,
        fontLoadFailures,
        decodedFonts: result.decodedFonts ?? [],
      };
    },
    async renderSlide(slide: number) {
      return call<{
        slide: number;
        svg: string;
        media: Map<string, MediaBlob>;
        notes?: string;
        layoutName?: string;
        sectionName?: string;
      }>({ type: "render", slide });
    },
    close() {
      try {
        worker.postMessage({ type: "close", id: ++nextId });
      } catch {
        // Worker may already be terminated.
      }
      worker.terminate();
    },
  };
}
