import { type CSSProperties, useEffect, useState } from "react";
import { subscribeLocale, t } from "./i18n.js";
import type { SlideSvg } from "../types.js";

interface Section {
  name: string;
  startSlide: number;
}

export interface SectionNavProps {
  slides: SlideSvg[];
  currentSlide: number;
  onJump: (slide: number) => void;
}

/**
 * Section breadcrumb + jump links derived from each slide's
 * `section_name`. React port of `<pptx-section-nav>`.
 */
export function SectionNav(props: SectionNavProps): JSX.Element {
  const { slides, currentSlide, onJump } = props;
  const [, setLocaleTick] = useState<number>(0);

  useEffect(() => {
    const unsub = subscribeLocale(() => setLocaleTick((n) => n + 1));
    return unsub;
  }, []);

  const sections = computeSections(slides);
  if (sections.length === 0) {
    return (
      <div style={{ ...hostStyle, ...emptyStyle }}>{t("section.empty")}</div>
    );
  }

  // Active section is the highest entry whose startSlide <= currentSlide.
  let activeIndex = 0;
  for (let i = 0; i < sections.length; i += 1) {
    if (sections[i].startSlide <= currentSlide) activeIndex = i;
  }

  return (
    <div style={hostStyle}>
      {sections.map((section, idx) => {
        const isActive = idx === activeIndex;
        return (
          <button
            key={`${section.name}-${section.startSlide}`}
            style={
              isActive ? { ...buttonStyle, ...activeButtonStyle } : buttonStyle
            }
            onClick={() => onJump(section.startSlide)}
            title={`${section.name} (${t("viewer.slideTitle", { number: section.startSlide })})`}
          >
            {section.name}
          </button>
        );
      })}
    </div>
  );
}

function computeSections(slides: SlideSvg[]): Section[] {
  const out: Section[] = [];
  let last: string | undefined;
  for (const slide of slides) {
    if (slide.section_name && slide.section_name !== last) {
      out.push({ name: slide.section_name, startSlide: slide.slide_number });
      last = slide.section_name;
    } else if (!slide.section_name) {
      last = undefined;
    }
  }
  return out;
}

const hostStyle: CSSProperties = {
  display: "flex",
  gap: 6,
  padding: "6px 12px",
  background: "var(--pptx-section-bg, #1a1a1f)",
  color: "var(--pptx-section-fg, #ccc)",
  font: "12px system-ui, sans-serif",
  overflowX: "auto",
  boxSizing: "border-box",
  borderBottom: "1px solid var(--pptx-shell-border, #2a2a30)",
};

const buttonStyle: CSSProperties = {
  background: "var(--pptx-section-tile, #25252b)",
  color: "inherit",
  border: "1px solid transparent",
  padding: "4px 10px",
  borderRadius: 999,
  cursor: "pointer",
  font: "inherit",
  whiteSpace: "nowrap",
};

const activeButtonStyle: CSSProperties = {
  borderColor: "var(--pptx-section-active, #6aa3ff)",
  color: "var(--pptx-section-active-fg, #fff)",
};

const emptyStyle: CSSProperties = {
  color: "var(--pptx-shell-status, #666)",
  fontStyle: "italic",
};
