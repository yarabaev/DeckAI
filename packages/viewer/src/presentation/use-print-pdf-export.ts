/**
 * `usePrintPdfExport` — extracts the print + PDF export handlers from
 * the `PptxPresentation` component body.
 *
 * Both flows share the same shape:
 * 1. show the centred progress overlay,
 * 2. force every slide to render via `ensureAllSlidesRendered`,
 * 3. inline media as data URIs (so the print window / PDF doesn't
 *    fetch from the deck-scoped `pptx-media://` blob namespace),
 * 4. invoke the underlying export utility,
 * 5. dismiss the overlay (with a small grace period so the
 *    "Opening print dialog…" / "Saving…" steps stay visible).
 *
 * Pulled out so the main component can stop carrying the ~110-line
 * pair of `useCallback`s along with the rest of its state graph.
 */

import { useCallback } from "react";

import { exportToPdf } from "../ui/pdf.js";
import { inlineMediaAsDataUrls } from "../ui/media-inline.js";
import { printDeck } from "../ui/print.js";
import { t } from "../ui/i18n.js";
import type { SlideSvg } from "../types.js";

/**
 * Live progress payload — exactly the shape the centred export-overlay
 * consumes. `current` / `total` are optional so the caller can paint
 * an indeterminate bar before the per-slide loop starts.
 */
export interface ExportProgress {
  title: string;
  step: string;
  current?: number;
  total?: number;
}

export interface UsePrintPdfExportArgs {
  /** Display name (filename) used as the print job title and PDF stem. */
  name?: string | null;
  /**
   * Force-render every slide and return their SVGs. Reports progress
   * via the optional callback (1-based current index, 1-based total).
   * Mirrors the `ensureAllSlidesRendered` private helper inside
   * `PptxPresentation` — pass it in from the host because it owns the
   * underlying slide cache.
   */
  ensureAllSlidesRendered: (
    silent: boolean,
    onProgress?: (current: number, total: number) => void,
  ) => Promise<SlideSvg[]>;
  setProgress: (progress: ExportProgress | null) => void;
  setErrorMsg: (msg: string | null) => void;
  setPhase: (phase: string) => void;
}

export interface UsePrintPdfExportResult {
  /** Open the browser's native print dialog with every slide laid out. */
  handlePrint: () => Promise<void>;
  /** Save every slide as a single PDF via `exportToPdf`. */
  handleExportPdf: () => Promise<void>;
}

export function usePrintPdfExport(
  args: UsePrintPdfExportArgs,
): UsePrintPdfExportResult {
  const { name, ensureAllSlidesRendered, setProgress, setErrorMsg, setPhase } =
    args;

  const handlePrint = useCallback(async () => {
    const printTitle = t("progress.titlePrint");
    setProgress({ title: printTitle, step: t("phase.preparingPrint") });
    try {
      const slides = await ensureAllSlidesRendered(false, (current, total) => {
        setProgress({
          title: printTitle,
          step: t("phase.preparingSlideOf", { current, total }),
          current,
          total,
        });
      });
      if (slides.length === 0) {
        setErrorMsg(t("status.nothingToPrint"));
        return;
      }
      const inlined = slides.map((s) => ({
        ...s,
        svg: inlineMediaAsDataUrls(s.svg, new Map()),
      }));
      await printDeck(inlined, {
        title: name ?? t("dialog.title"),
        onProgress: ({ phase: p, current, total }) => {
          setProgress({
            title: printTitle,
            step:
              p === "open-dialog"
                ? t("phase.openingPrintDialog")
                : t("phase.layingOutPrintOf", { current: current + 1, total }),
            current: p === "open-dialog" ? total : current + 1,
            total,
          });
        },
      });
    } catch (err) {
      setErrorMsg((err as Error).message ?? String(err));
    } finally {
      // Leave the overlay up briefly so the "Opening print dialog…"
      // step is visible — the native print dialog itself can take a
      // moment to surface and a too-fast dismiss feels jarring.
      setTimeout(() => setProgress(null), 400);
    }
  }, [ensureAllSlidesRendered, name, setProgress, setErrorMsg]);

  const handleExportPdf = useCallback(async () => {
    const pdfTitle = t("progress.titlePdf");
    setProgress({ title: pdfTitle, step: t("phase.preparingPdf") });
    setPhase(t("phase.preparingPdf"));
    try {
      const slides = await ensureAllSlidesRendered(false, (current, total) => {
        setProgress({
          title: pdfTitle,
          step: t("phase.preparingSlideOf", { current, total }),
          current,
          total,
        });
      });
      if (slides.length === 0) {
        setErrorMsg(t("status.nothingToExport"));
        return;
      }
      const inlined = slides.map((s) => ({
        ...s,
        svg: inlineMediaAsDataUrls(s.svg, new Map()),
      }));
      const bytes = await exportToPdf({
        slides: inlined,
        onProgress: ({ phase: p, current, total }) => {
          if (p === "rasterize") {
            setPhase(t("phase.renderingPdf", { current: current + 1, total }));
            setProgress({
              title: pdfTitle,
              step: t("phase.renderingPdf", { current: current + 1, total }),
              current: current + 1,
              total,
            });
          } else {
            setPhase(t("phase.encodingPdf"));
            setProgress({
              title: pdfTitle,
              step: t("phase.encodingPdf"),
              current: total,
              total,
            });
          }
        },
      });
      setProgress({ title: pdfTitle, step: t("phase.savingPdf") });
      const blob = new Blob([bytes as BlobPart], { type: "application/pdf" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      const baseName = (name ?? "presentation").replace(/\.[^.]+$/, "");
      a.download = `${baseName}.pdf`;
      a.click();
      setTimeout(() => URL.revokeObjectURL(url), 4000);
      setPhase(t("status.exported", { count: slides.length }));
    } catch (err) {
      setErrorMsg(
        t("status.pdfFailed", {
          reason: (err as Error).message ?? String(err),
        }),
      );
    } finally {
      setTimeout(() => setPhase(""), 2000);
      setTimeout(() => setProgress(null), 400);
    }
  }, [ensureAllSlidesRendered, name, setProgress, setErrorMsg, setPhase]);

  return { handlePrint, handleExportPdf };
}
