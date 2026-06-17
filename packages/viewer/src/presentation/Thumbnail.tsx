/**
 * Slide thumbnail tile — used both by the sidebar (vertical strip,
 * `layout="row"`) and the grid view (slide-sorter, `layout="tile"`).
 *
 * Rendering strategy:
 * 1. The button mounts in a "placeholder" state with no SVG.
 * 2. An `IntersectionObserver` watches the button; when it enters the
 *    viewport (or sidebar scroll container), the slide is fetched
 *    from the host's `getThumbnail` callback. This keeps a 132-slide
 *    deck from fanning out to 132 IPC calls on first paint.
 * 3. The fetched SVG is id-namespaced (so multiple thumbnails on the
 *    same page don't collide on `clipPath` / gradient ids) and
 *    inserted via DOMParser → `importNode` so the SVG namespace is
 *    handled correctly by the browser's foreign-element machinery.
 *
 * Once visible, the flag stays sticky: there's no benefit to
 * unloading a slide we already paid to render, and re-fetching on
 * scroll-out / scroll-in produces a perceptible flash.
 */

import { memo, useEffect, useRef, useState, type CSSProperties } from "react";

import { parseAspect, uniquifyIds } from "../svg-utils.js";
import { t } from "../ui/i18n.js";
import {
  sidebarEmptyStyle,
  thumbnailButtonActiveStyle,
  thumbnailButtonStyle,
  thumbnailCaptionStyle,
  thumbnailFrameStyle,
  thumbnailIndexStyle,
  thumbnailInnerStyle,
  thumbnailPlaceholderStyle,
  thumbnailTileActiveStyle,
  thumbnailTileFrameStyle,
  thumbnailTileStyle,
  thumbStripStyle,
} from "./styles.js";
import type { CachedSlide } from "./types.js";

export interface ThumbnailProps {
  slide: number;
  active: boolean;
  onClick: () => void;
  getThumbnail: (slide: number) => Promise<CachedSlide | null>;
  aspectFallback: number;
  /**
   * `"row"` — sidebar style: slide number on the left, frame on the
   * right (vertical strip). `"tile"` — slide-sorter style: frame on
   * top, caption underneath, all tiles share a uniform footprint via
   * the `aspectFallback` (deck aspect), so the grid stays aligned
   * even when individual slides parse at slightly different aspects.
   */
  layout?: "row" | "tile";
}

export function Thumbnail(props: ThumbnailProps): JSX.Element {
  const {
    slide,
    active,
    onClick,
    getThumbnail,
    aspectFallback,
    layout = "row",
  } = props;
  const [svg, setSvg] = useState<string | null>(null);
  const [aspect, setAspect] = useState<number>(aspectFallback);
  const hostRef = useRef<HTMLDivElement | null>(null);
  const buttonRef = useRef<HTMLButtonElement | null>(null);
  // `visible` gates the slide fetch so a 132-slide deck doesn't fan
  // out to 132 IPC calls when the sidebar mounts. We watch the button
  // with `IntersectionObserver` and only request the slide once it
  // crosses into the viewport (or its sidebar's scroll container).
  // Once true, the flag stays true — there's no benefit to unloading
  // a slide we already paid to render.
  const [visible, setVisible] = useState<boolean>(active);

  useEffect(() => {
    if (active) setVisible(true);
  }, [active]);

  useEffect(() => {
    if (visible) return;
    const el = buttonRef.current;
    if (!el || typeof IntersectionObserver === "undefined") {
      setVisible(true); // SSR / older browsers — fall back to eager
      return;
    }
    const obs = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            setVisible(true);
            obs.disconnect();
            return;
          }
        }
      },
      // Generous rootMargin so the slide is already rendered by the
      // time the user scrolls the sidebar into it — no perceptible
      // pop-in.
      { rootMargin: "200px" },
    );
    obs.observe(el);
    return () => obs.disconnect();
  }, [visible]);

  useEffect(() => {
    if (!visible) return;
    let cancelled = false;
    void (async () => {
      const cached = await getThumbnail(slide);
      if (cancelled || !cached) return;
      setAspect(parseAspect(cached.svg) ?? aspectFallback);
      setSvg(cached.preparedSvg);
    })();
    return () => {
      cancelled = true;
    };
  }, [visible, slide, getThumbnail, aspectFallback]);

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;
    while (host.firstChild) host.removeChild(host.firstChild);
    if (!svg) return;
    try {
      // Per-thumbnail unique ID namespace — see svg-utils.uniquifyIds
      // for why this is required (multiple SVGs share document ID
      // namespace, leading to clipPath / gradient cross-references).
      const namespaced = uniquifyIds(svg, `${layout}-s${slide}`);
      const doc = new DOMParser().parseFromString(namespaced, "image/svg+xml");
      const root = doc.documentElement;
      if (!root) return;
      host.appendChild(document.importNode(root, true));
    } catch {
      /* swallow */
    }
  }, [svg, slide, layout]);

  const isTile = layout === "tile";
  const frameAspect = isTile ? aspectFallback : aspect;
  const buttonStyle = isTile
    ? active
      ? thumbnailTileActiveStyle
      : thumbnailTileStyle
    : active
      ? thumbnailButtonActiveStyle
      : thumbnailButtonStyle;
  const frameStyle: CSSProperties = isTile
    ? thumbnailTileFrameStyle
    : thumbnailFrameStyle;

  return (
    <button
      ref={buttonRef}
      style={buttonStyle}
      onClick={onClick}
      title={t("viewer.slideTitle", { number: slide })}
      aria-label={t("viewer.slideTitle", { number: slide })}
    >
      {!isTile && <span style={thumbnailIndexStyle}>{slide}</span>}
      <div style={{ ...frameStyle, aspectRatio: `${frameAspect}` }}>
        {svg ? (
          <div ref={hostRef} style={thumbnailInnerStyle} />
        ) : (
          <div style={thumbnailPlaceholderStyle}>…</div>
        )}
      </div>
      {isTile && <span style={thumbnailCaptionStyle}>{slide}</span>}
    </button>
  );
}

export interface ThumbnailSidebarProps {
  slideCount: number;
  currentSlide: number;
  onSelect: (slide: number) => void;
  getThumbnail: (slide: number) => Promise<CachedSlide | null>;
  aspectFallback: number;
  /**
   * Per-deck identifier — included in each `<Thumbnail>`'s React key
   * so a deck swap forces every thumbnail to unmount + remount with
   * fresh internal state. Without this, Thumbnail's `useEffect` deps
   * (`[visible, slide, getThumbnail, aspectFallback]`) don't change
   * across decks (`slide` is still 1, `getThumbnail` is the same
   * stable callback), the cached `svg` state from the previous deck
   * sticks, and the panel keeps showing stale tiles even after
   * `slideCache` itself is flushed.
   */
  deckKey: string;
}

export const ThumbnailSidebar = memo(function ThumbnailSidebar(
  props: ThumbnailSidebarProps,
): JSX.Element {
  const {
    slideCount,
    currentSlide,
    onSelect,
    getThumbnail,
    aspectFallback,
    deckKey,
  } = props;
  return (
    <div style={thumbStripStyle}>
      {Array.from({ length: slideCount }, (_, i) => {
        const n = i + 1;
        return (
          <Thumbnail
            key={`${deckKey}::${n}`}
            slide={n}
            active={n === currentSlide}
            onClick={() => onSelect(n)}
            getThumbnail={getThumbnail}
            aspectFallback={aspectFallback}
            layout="tile"
          />
        );
      })}
      {slideCount === 0 && (
        <div style={sidebarEmptyStyle}>{t("viewer.empty")}</div>
      )}
    </div>
  );
});
