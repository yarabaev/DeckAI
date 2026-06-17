/**
 * Status bar — bottom-of-shell strip showing the live phase
 * message, slide counter, selected fonts, font-usage indicator,
 * notes toggle, view-mode picker, and zoom controls.
 *
 * Stateless — every interactive element either dispatches via a
 * callback the host supplies or toggles state the host owns.
 */

import type { MutableRefObject } from "react";
import {
  ArrowsOutSimple,
  ChatCircleText,
  GridFour,
  Minus,
  Plus,
  SquaresFour,
} from "@phosphor-icons/react";

import { FontUsageIndicator } from "../ui/FontUsageIndicator.js";
import { t } from "../ui/i18n.js";
import type { SlideMeta, TypefaceUsage } from "../types.js";
import {
  activeIconSmStyle,
  metaStyle,
  phaseStyle,
  selectionFontsButtonActiveStyle,
  selectionFontsButtonStyle,
  selectionFontsContainerStyle,
  selectionFontsListItemStyle,
  selectionFontsListStyle,
  selectionFontsPopoverStyle,
  spacerStyle,
  statusBarStyle,
  statusIconStyle,
  statusSepStyle,
  zoomPctStyle,
  zoomSliderStyle,
} from "./styles.js";

const ZOOM_MIN = 0.25;
const ZOOM_MAX = 8;

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

export interface StatusBarProps {
  slideshow: boolean;

  phase: string;
  errorMsg: string | null;
  slideMeta: SlideMeta | null;
  slideCount: number;
  currentSlide: number;

  selectionFonts: string[];
  selectionFontsOpen: boolean;
  setSelectionFontsOpen: (next: boolean | ((prev: boolean) => boolean)) => void;
  selectionFontsRef: MutableRefObject<HTMLDivElement | null>;

  fontUsage: TypefaceUsage[];

  notesOpen: boolean;
  setNotesOpen: (next: boolean | ((prev: boolean) => boolean)) => void;

  viewMode: "normal" | "grid";
  setViewMode: (next: "normal" | "grid") => void;

  zoom: number;
  zoomPct: number;
  setZoom: (next: number | ((prev: number) => number)) => void;
  setZoomFromPct: (pct: number) => void;
  /** Reset zoom + pan to fit-window. */
  reset: () => void;
}

export function StatusBar(props: StatusBarProps): JSX.Element | null {
  if (props.slideshow) return null;
  const {
    phase,
    errorMsg,
    slideMeta,
    slideCount,
    currentSlide,
    selectionFonts,
    selectionFontsOpen,
    setSelectionFontsOpen,
    selectionFontsRef,
    fontUsage,
    notesOpen,
    setNotesOpen,
    viewMode,
    setViewMode,
    zoomPct,
    setZoom,
    setZoomFromPct,
    reset,
  } = props;

  return (
    <footer style={statusBarStyle}>
      <span style={phaseStyle}>
        {phase || (errorMsg ? `⚠ ${errorMsg}` : t("common.ready"))}
      </span>
      <div style={spacerStyle} />
      {slideMeta?.section_name ? (
        <span style={metaStyle}>{slideMeta.section_name}</span>
      ) : null}
      <span style={metaStyle}>
        {slideCount === 0
          ? t("status.slideEmpty")
          : t("status.slideOf", {
              current: currentSlide,
              total: slideCount,
            })}
      </span>
      {selectionFonts.length > 0 ? (
        <>
          <span style={statusSepStyle} />
          <div ref={selectionFontsRef} style={selectionFontsContainerStyle}>
            <button
              type="button"
              style={
                selectionFontsOpen
                  ? selectionFontsButtonActiveStyle
                  : selectionFontsButtonStyle
              }
              title={t("status.selectionFontTitle", {
                fonts: selectionFonts.join(", "),
              })}
              aria-haspopup="dialog"
              aria-expanded={selectionFontsOpen}
              onClick={() => setSelectionFontsOpen((o) => !o)}
            >
              {t("status.selectionFontLabel")}{" "}
              {selectionFonts.length === 1
                ? selectionFonts[0]
                : t("status.selectionFontMultiple", {
                    first: selectionFonts[0],
                    extra: selectionFonts.length - 1,
                  })}
            </button>
            {selectionFontsOpen ? (
              <div
                style={selectionFontsPopoverStyle}
                role="dialog"
                aria-label={t("status.selectionFontTitle", {
                  fonts: selectionFonts.join(", "),
                })}
              >
                <ul style={selectionFontsListStyle}>
                  {selectionFonts.map((family) => (
                    <li key={family} style={selectionFontsListItemStyle}>
                      {family}
                    </li>
                  ))}
                </ul>
              </div>
            ) : null}
          </div>
        </>
      ) : null}
      <span style={statusSepStyle} />
      <FontUsageIndicator
        fontUsage={fontUsage}
        buttonStyle={statusIconStyle}
        buttonActiveStyle={activeIconSmStyle}
      />
      <button
        style={notesOpen ? activeIconSmStyle : statusIconStyle}
        onClick={() => setNotesOpen((o) => !o)}
        title={t("status.toggleNotes")}
        aria-label={t("status.toggleNotes")}
        aria-pressed={notesOpen}
      >
        <ChatCircleText size={14} weight={notesOpen ? "fill" : "regular"} />
      </button>
      <span style={statusSepStyle} />
      <button
        style={viewMode === "normal" ? activeIconSmStyle : statusIconStyle}
        onClick={() => setViewMode("normal")}
        title={t("status.normalView")}
        aria-label={t("status.normalView")}
        aria-pressed={viewMode === "normal"}
      >
        <SquaresFour
          size={14}
          weight={viewMode === "normal" ? "fill" : "regular"}
        />
      </button>
      <button
        style={viewMode === "grid" ? activeIconSmStyle : statusIconStyle}
        onClick={() => setViewMode("grid")}
        title={t("status.gridView")}
        aria-label={t("status.gridView")}
        aria-pressed={viewMode === "grid"}
      >
        <GridFour size={14} weight={viewMode === "grid" ? "fill" : "regular"} />
      </button>
      <span style={statusSepStyle} />
      <button
        style={statusIconStyle}
        onClick={() => setZoom((z) => clamp(z * 0.8, ZOOM_MIN, ZOOM_MAX))}
        title={t("status.zoomOut")}
        aria-label={t("status.zoomOut")}
      >
        <Minus size={14} weight="bold" />
      </button>
      <input
        type="range"
        min={Math.round(ZOOM_MIN * 100)}
        max={400}
        step={1}
        value={Math.min(zoomPct, 400)}
        onChange={(e) => setZoomFromPct(parseFloat(e.target.value))}
        style={zoomSliderStyle}
        title={t("status.zoom")}
        aria-label={t("status.zoom")}
      />
      <button
        style={statusIconStyle}
        onClick={() => setZoom((z) => clamp(z * 1.25, ZOOM_MIN, ZOOM_MAX))}
        title={t("status.zoomIn")}
        aria-label={t("status.zoomIn")}
      >
        <Plus size={14} weight="bold" />
      </button>
      <span
        style={zoomPctStyle}
        onClick={reset}
        title={t("status.zoomReset")}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") reset();
        }}
      >
        {zoomPct}%
      </span>
      <button
        style={statusIconStyle}
        onClick={reset}
        title={t("status.fitWindow")}
        aria-label={t("status.fitWindow")}
      >
        <ArrowsOutSimple size={14} weight="bold" />
      </button>
    </footer>
  );
}
