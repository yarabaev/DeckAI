/**
 * Top ribbon — filename · slide nav · search · print / pdf /
 * slideshow · shortcuts · settings. Sub-extracted from
 * `PptxPresentation.tsx`.
 *
 * Each handler is supplied by the host so this component can stay
 * stateless: it never owns a `useState`, just renders the props it
 * was handed. The host's `toolbarStart` / `toolbarEnd` slots flank
 * the built-in controls so embedders can splice their own buttons
 * in (the chrome-extension uses `toolbarStart` for an Open File
 * picker, for example).
 */

import type { ReactNode } from "react";
import {
  CaretLeft,
  CaretRight,
  FilePdf,
  GearSix,
  MagnifyingGlass,
  Play,
  Printer,
  Question,
  SkipBack,
  SkipForward,
} from "@phosphor-icons/react";

import { t } from "../ui/i18n.js";
import {
  activeIconStyle,
  counterStyle,
  disabledTextButtonStyle,
  dividerStyle,
  filenameStyle,
  iconButtonStyle,
  ribbonStyle,
  spacerStyle,
  textButtonStyle,
} from "./styles.js";

export interface ToolbarProps {
  toolbarStart?: ReactNode;
  toolbarEnd?: ReactNode;
  name?: string | null;
  slideCount: number;
  currentSlide: number;
  setCurrentSlide: (next: number | ((prev: number) => number)) => void;

  searchOpen: boolean;
  setSearchOpen: (next: boolean | ((prev: boolean) => boolean)) => void;

  /** Whether the deck-wide actions can fire. When false, host has
   *  prefetching enabled and the deck isn't fully rendered yet — the
   *  buttons disable + show a "preparing…" hover hint. */
  allSlidesReady: boolean;
  /** Inverse of the prefetching gate: when true, deck-wide actions
   *  fire on demand and skip the readiness check. */
  noPrefetch: boolean;
  /** Helper that turns a localized base label into the
   *  "Foo (waiting on N slides…)" form when the gate is closed. */
  deckGateTitle: (base: string, ready: boolean, slideCount: number) => string;

  handlePrint: () => Promise<void>;
  handleExportPdf: () => Promise<void>;
  handleSlideshow: () => Promise<void>;

  shortcutsOpen: boolean;
  setShortcutsOpen: (next: boolean | ((prev: boolean) => boolean)) => void;
  settingsOpen: boolean;
  setSettingsOpen: (next: boolean) => void;
}

export function Toolbar(props: ToolbarProps): JSX.Element {
  const {
    toolbarStart,
    toolbarEnd,
    name,
    slideCount,
    currentSlide,
    setCurrentSlide,
    searchOpen,
    setSearchOpen,
    allSlidesReady,
    noPrefetch,
    deckGateTitle,
    handlePrint,
    handleExportPdf,
    handleSlideshow,
    shortcutsOpen,
    setShortcutsOpen,
    settingsOpen,
    setSettingsOpen,
  } = props;
  const exportDisabled = slideCount === 0 || (!noPrefetch && !allSlidesReady);
  const exportButtonStyle =
    !noPrefetch && !allSlidesReady && slideCount > 0
      ? disabledTextButtonStyle
      : textButtonStyle;

  return (
    <header style={ribbonStyle}>
      {toolbarStart}
      <span style={filenameStyle} title={name ?? ""}>
        {name ?? t("viewer.noFile")}
      </span>
      <div style={spacerStyle} />
      <button
        style={iconButtonStyle}
        onClick={() => setCurrentSlide(1)}
        disabled={slideCount === 0 || currentSlide <= 1}
        title={t("nav.firstSlide")}
        aria-label={t("nav.firstSlide")}
      >
        <SkipBack size={16} weight="fill" />
      </button>
      <button
        style={iconButtonStyle}
        onClick={() => setCurrentSlide((s) => Math.max(1, s - 1))}
        disabled={currentSlide <= 1}
        title={t("nav.previousSlide")}
        aria-label={t("nav.previousSlide")}
      >
        <CaretLeft size={16} weight="bold" />
      </button>
      <span style={counterStyle}>
        {slideCount === 0 ? "—" : `${currentSlide} / ${slideCount}`}
      </span>
      <button
        style={iconButtonStyle}
        onClick={() => setCurrentSlide((s) => Math.min(slideCount, s + 1))}
        disabled={currentSlide >= slideCount}
        title={t("nav.nextSlide")}
        aria-label={t("nav.nextSlide")}
      >
        <CaretRight size={16} weight="bold" />
      </button>
      <button
        style={iconButtonStyle}
        onClick={() => setCurrentSlide(slideCount)}
        disabled={slideCount === 0 || currentSlide >= slideCount}
        title={t("nav.lastSlide")}
        aria-label={t("nav.lastSlide")}
      >
        <SkipForward size={16} weight="fill" />
      </button>
      <span style={dividerStyle} />
      <button
        style={searchOpen ? activeIconStyle : iconButtonStyle}
        onClick={() => setSearchOpen((o) => !o)}
        title={t("search.title")}
        aria-label={t("search.title")}
        aria-pressed={searchOpen}
      >
        <MagnifyingGlass size={16} weight="bold" />
      </button>
      <span style={dividerStyle} />
      {/* Deck-wide actions. The "ready" gate is bypassed when the host
          disables prefetching (`noPrefetch`) — the click handler then
          triggers `ensureAllSlidesRendered()` itself, surfacing a
          `phase.preparingSlideOf` message in the status bar. */}
      <button
        style={exportButtonStyle}
        onClick={() => void handlePrint()}
        title={
          noPrefetch
            ? t("output.printTitle")
            : deckGateTitle(t("output.printTitle"), allSlidesReady, slideCount)
        }
        disabled={exportDisabled}
      >
        <Printer size={16} weight="regular" /> {t("output.print")}
      </button>
      <button
        style={exportButtonStyle}
        onClick={() => void handleExportPdf()}
        title={
          noPrefetch
            ? t("output.pdfTitle")
            : deckGateTitle(t("output.pdfTitle"), allSlidesReady, slideCount)
        }
        disabled={exportDisabled}
      >
        <FilePdf size={16} weight="regular" /> {t("output.pdf")}
      </button>
      <button
        style={exportButtonStyle}
        onClick={() => void handleSlideshow()}
        title={
          noPrefetch
            ? t("output.slideshowTitle")
            : deckGateTitle(
                t("output.slideshowTitle"),
                allSlidesReady,
                slideCount,
              )
        }
        disabled={exportDisabled}
      >
        <Play size={16} weight="fill" /> {t("output.slideshow")}
      </button>
      <span style={dividerStyle} />
      <button
        style={shortcutsOpen ? activeIconStyle : iconButtonStyle}
        onClick={() => setShortcutsOpen((o) => !o)}
        title={t("shortcuts.openTitle")}
        aria-label={t("shortcuts.openTitle")}
        aria-haspopup="dialog"
        aria-expanded={shortcutsOpen}
      >
        <Question size={16} weight="regular" />
      </button>
      <button
        style={iconButtonStyle}
        onClick={() => setSettingsOpen(true)}
        title={t("settings.openTitle")}
        aria-label={t("settings.openTitle")}
        aria-haspopup="dialog"
        aria-expanded={settingsOpen}
      >
        <GearSix size={16} weight="regular" />
      </button>
      {toolbarEnd}
    </header>
  );
}
