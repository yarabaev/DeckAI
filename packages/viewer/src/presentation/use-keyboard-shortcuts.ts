/**
 * `useKeyboardShortcuts` — global keydown / keyup / blur listener
 * that owns every shell-level keyboard binding.
 *
 * Bindings:
 * - Cmd/Ctrl + F → toggle search drawer
 * - Cmd/Ctrl + P → print (gated on slideCount > 0 && allSlidesReady)
 * - Cmd/Ctrl + A → select every shape with a known bbox
 * - Cmd/Ctrl + C → copy selected shapes' text content
 * - Cmd/Ctrl + (= / +) / - / 0 → zoom in / out / reset
 * - ←/→ + Home/End + PageUp/PageDown → slide navigation
 * - Esc → exit slideshow / close search / clear selection
 * - Space (held) → engage pan mode
 * - ? → toggle shortcuts help dialog
 *
 * Extracted from `PptxPresentation` so the keyboard surface lives in
 * one place — the host component just wires its state slots to the
 * hook arguments and the hook installs a single window-level
 * keydown / keyup / blur handler that orchestrates them.
 */

import { useEffect } from "react";
import type { MutableRefObject } from "react";

const ZOOM_MIN = 0.25;
const ZOOM_MAX = 8;

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

export interface UseKeyboardShortcutsArgs {
  slideCount: number;
  allSlidesReady: boolean;
  searchOpen: boolean;
  slideshow: boolean;
  selectedIds: Set<string>;
  textEditId: string | null;

  /** Ref-handle for Cmd/Ctrl+P. The keyboard handler runs in an
   *  effect that sees the ref's *current* value, so the parent can
   *  install / replace `handlePrint` without forcing the keyboard
   *  effect to re-subscribe on every print-handler identity flip. */
  handlePrintRef: MutableRefObject<(() => Promise<void>) | null>;
  /** Live cache of every shape's bbox. Cmd/Ctrl+A selects every key. */
  bboxMapRef: MutableRefObject<
    Map<string, { x: number; y: number; w: number; h: number }>
  >;
  /** Slide DOM host for Cmd/Ctrl+C text extraction. */
  slideRef: MutableRefObject<HTMLElement | null>;

  setSearchOpen: (next: boolean | ((prev: boolean) => boolean)) => void;
  setSlideshow: (next: boolean) => void;
  setSelectedIds: (next: Set<string>) => void;
  setTextEditId: (next: string | null) => void;
  setRubberBand: (next: null) => void;
  setSpaceHeld: (next: boolean) => void;
  setShortcutsOpen: (next: boolean | ((prev: boolean) => boolean)) => void;
  setCurrentSlide: (next: number | ((prev: number) => number)) => void;
  setZoom: (next: number | ((prev: number) => number)) => void;
  setPanX: (next: number) => void;
  setPanY: (next: number) => void;
}

export function useKeyboardShortcuts(args: UseKeyboardShortcutsArgs): void {
  const {
    slideCount,
    allSlidesReady,
    searchOpen,
    slideshow,
    selectedIds,
    textEditId,
    handlePrintRef,
    bboxMapRef,
    slideRef,
    setSearchOpen,
    setSlideshow,
    setSelectedIds,
    setTextEditId,
    setRubberBand,
    setSpaceHeld,
    setShortcutsOpen,
    setCurrentSlide,
    setZoom,
    setPanX,
    setPanY,
  } = args;

  useEffect(() => {
    const onKey = (ev: KeyboardEvent): void => {
      const mod = ev.metaKey || ev.ctrlKey;
      if (mod && ev.key === "f") {
        ev.preventDefault();
        setSearchOpen((o) => !o);
        return;
      }
      // Cmd/Ctrl+P → Print. Honour the same readiness gate as the
      // toolbar button so the user never gets a partial deck in
      // their print preview.
      if (mod && ev.key === "p") {
        ev.preventDefault();
        if (slideCount > 0 && allSlidesReady) void handlePrintRef.current?.();
        return;
      }
      if (ev.key === "Escape") {
        if (slideshow) {
          setSlideshow(false);
          if (document.fullscreenElement) void document.exitFullscreen();
          ev.preventDefault();
          return;
        }
        if (searchOpen) {
          setSearchOpen(false);
          ev.preventDefault();
          return;
        }
        // Esc clears selection / exits text-edit mode (PowerPoint
        // convention — same single keystroke deselects everything).
        setSelectedIds(new Set());
        setTextEditId(null);
        setRubberBand(null);
      }
      // Cmd/Ctrl + A → select every shape with a known bbox on the
      // active slide.  Defer to the input field if the user is editing
      // a text shape — text-edit owns the keyboard.
      if (mod && ev.key.toLowerCase() === "a" && !textEditId) {
        ev.preventDefault();
        setSelectedIds(new Set(bboxMapRef.current.keys()));
        return;
      }
      // Cmd/Ctrl+C — copy selected shapes' text content.
      if (
        mod &&
        ev.key.toLowerCase() === "c" &&
        selectedIds.size > 0 &&
        !textEditId
      ) {
        const host = slideRef.current;
        if (host) {
          const parts: string[] = [];
          for (const id of selectedIds) {
            const el = host.querySelector(`[data-sp-id="${CSS.escape(id)}"]`);
            const txt = el?.textContent?.trim();
            if (txt) parts.push(txt);
          }
          if (parts.length > 0 && navigator.clipboard) {
            ev.preventDefault();
            void navigator.clipboard.writeText(parts.join("\n\n"));
          }
        }
        return;
      }
      // Space → engage pan mode. Don't preventDefault on the space
      // itself (it's not a default-bound key in this context) — just
      // flip the flag so pointer handlers know to pan instead of
      // select.
      if (ev.key === " " && !ev.repeat && !textEditId) {
        setSpaceHeld(true);
      }
      // `?` → toggle shortcuts help dialog. Honour both the literal
      // `?` (US layout) and Shift+/ (most layouts) without colliding
      // with text-edit input.
      if (ev.key === "?" && !textEditId) {
        ev.preventDefault();
        setShortcutsOpen((o) => !o);
        return;
      }
      if (ev.key === "ArrowRight" || ev.key === "PageDown") {
        setCurrentSlide((s) => Math.min(slideCount, s + 1));
        ev.preventDefault();
      } else if (ev.key === "ArrowLeft" || ev.key === "PageUp") {
        setCurrentSlide((s) => Math.max(1, s - 1));
        ev.preventDefault();
      } else if (ev.key === "Home") {
        setCurrentSlide(1);
        ev.preventDefault();
      } else if (ev.key === "End") {
        setCurrentSlide(slideCount);
        ev.preventDefault();
      } else if (mod && (ev.key === "=" || ev.key === "+")) {
        setZoom((z) => clamp(z * 1.25, ZOOM_MIN, ZOOM_MAX));
        ev.preventDefault();
      } else if (mod && ev.key === "-") {
        setZoom((z) => clamp(z * 0.8, ZOOM_MIN, ZOOM_MAX));
        ev.preventDefault();
      } else if (mod && ev.key === "0") {
        setZoom(1);
        setPanX(0);
        setPanY(0);
        ev.preventDefault();
      }
    };
    const onKeyUp = (ev: KeyboardEvent): void => {
      if (ev.key === " ") setSpaceHeld(false);
    };
    const onBlur = (): void => setSpaceHeld(false);
    window.addEventListener("keydown", onKey);
    window.addEventListener("keyup", onKeyUp);
    window.addEventListener("blur", onBlur);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("keyup", onKeyUp);
      window.removeEventListener("blur", onBlur);
    };
  }, [
    slideCount,
    allSlidesReady,
    searchOpen,
    slideshow,
    selectedIds,
    textEditId,
    handlePrintRef,
    bboxMapRef,
    slideRef,
    setSearchOpen,
    setSlideshow,
    setSelectedIds,
    setTextEditId,
    setRubberBand,
    setSpaceHeld,
    setShortcutsOpen,
    setCurrentSlide,
    setZoom,
    setPanX,
    setPanY,
  ]);
}
