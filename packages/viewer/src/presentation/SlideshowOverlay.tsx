/**
 * Fullscreen slideshow overlay — fades in over the editing shell when
 * the user clicks Slideshow. Handles two interaction surfaces:
 *
 * 1. The stage itself: plain left-click advances one slide; clicking
 *    on the last slide auto-exits the slideshow and drops out of
 *    fullscreen. Right-click / modified clicks are ignored so context
 *    menus + accessibility tooling still work.
 * 2. A bottom-right hover-fade nav pad with prev / next / exit
 *    buttons — anchored to a transparent 220×100 hit-zone so the
 *    pointer-near-corner reveal feels deliberate, not flickery.
 *
 * Slide rendering is the host's responsibility: the host passes a
 * `slideshowSlideRef` callback the overlay attaches to its inner
 * `<div>`, and the host pipes the rendered SVG into that ref via
 * the same imperative-DOM path the editing stage uses.
 */

import type { CSSProperties, MutableRefObject, ReactNode } from "react";
import { CaretLeft, CaretRight, X } from "@phosphor-icons/react";

import { t } from "../ui/i18n.js";
import {
  slideshowNavButtonStyle,
  slideshowNavGroupStyle,
  slideshowNavZoneStyle,
  slideshowStyle,
  stageStyle,
} from "./styles.js";

export interface SlideshowOverlayProps {
  open: boolean;
  currentSlide: number;
  slideCount: number;
  /** Set to false → host also exits fullscreen if the document is in it. */
  setSlideshow: (next: boolean) => void;
  setCurrentSlide: (next: number | ((prev: number) => number)) => void;
  /** Stage-relative scale of the slide ("fit" factor). Zero suppresses
   *  the inner slide host so the pre-mount frame stays black. */
  fit: number;
  slideSvg: string;
  canvasW: number;
  canvasH: number;
  slideW: number;
  slideH: number;
  slideshowStageRef: MutableRefObject<HTMLElement | null>;
  slideshowSlideRef: MutableRefObject<HTMLDivElement | null>;
  /** Optional extra child rendered after the corner-nav pad. */
  children?: ReactNode;
}

export function SlideshowOverlay(
  props: SlideshowOverlayProps,
): JSX.Element | null {
  if (!props.open) return null;
  const {
    currentSlide,
    slideCount,
    setSlideshow,
    setCurrentSlide,
    fit,
    slideSvg,
    canvasW,
    canvasH,
    slideW,
    slideH,
    slideshowStageRef,
    slideshowSlideRef,
    children,
  } = props;

  const stageInlineStyle: CSSProperties = {
    ...stageStyle,
    background: "#000",
    cursor: "pointer",
  };

  return (
    <div style={slideshowStyle}>
      <main
        ref={slideshowStageRef as MutableRefObject<HTMLElement | null>}
        style={stageInlineStyle}
        onClick={(ev) => {
          // Plain left-click anywhere on the stage advances one
          // slide — PowerPoint Slide Show convention. Auto-exit
          // on the last slide so the user lands back on the
          // editing surface naturally. Right-click / modified
          // clicks are ignored so context menus and accessibility
          // tooling still work.
          if (ev.button !== 0 || ev.ctrlKey || ev.metaKey || ev.shiftKey) {
            return;
          }
          if (currentSlide >= slideCount) {
            setSlideshow(false);
            if (document.fullscreenElement) {
              void document.exitFullscreen();
            }
          } else {
            setCurrentSlide((s) => Math.min(slideCount, s + 1));
          }
        }}
      >
        {fit > 0 && slideSvg && (
          <div
            style={{
              width: canvasW,
              height: canvasH,
              position: "relative",
            }}
          >
            <div
              ref={slideshowSlideRef}
              style={{
                width: slideW,
                height: slideH,
                position: "absolute",
                left: "50%",
                top: "50%",
                transform: `translate(-50%, -50%)`,
                background: "white",
              }}
            />
          </div>
        )}
      </main>
      {/* Corner navigation pad — bottom-right hover zone reveals
          prev / next / exit buttons. Sits above the click-to-
          advance handler with `stopPropagation` so button clicks
          never double-trigger the stage advance. */}
      <div
        data-pptx-slideshow-nav=""
        style={slideshowNavZoneStyle}
        onClick={(ev) => ev.stopPropagation()}
      >
        <div style={slideshowNavGroupStyle}>
          <button
            style={slideshowNavButtonStyle}
            onClick={() => setCurrentSlide((s) => Math.max(1, s - 1))}
            disabled={currentSlide <= 1}
            title={t("nav.previousSlide")}
            aria-label={t("nav.previousSlide")}
          >
            <CaretLeft size={20} weight="bold" />
          </button>
          <button
            style={slideshowNavButtonStyle}
            onClick={() => setCurrentSlide((s) => Math.min(slideCount, s + 1))}
            disabled={currentSlide >= slideCount}
            title={t("nav.nextSlide")}
            aria-label={t("nav.nextSlide")}
          >
            <CaretRight size={20} weight="bold" />
          </button>
          <button
            style={slideshowNavButtonStyle}
            onClick={() => {
              setSlideshow(false);
              if (document.fullscreenElement) {
                void document.exitFullscreen();
              }
            }}
            title={t("common.close")}
            aria-label={t("common.close")}
          >
            <X size={18} weight="bold" />
          </button>
        </div>
      </div>
      {children}
    </div>
  );
}
