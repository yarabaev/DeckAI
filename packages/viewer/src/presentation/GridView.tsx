/**
 * Slide-sorter / grid view — every slide rendered as a uniform tile
 * so the user can scan large decks at a glance and jump straight to
 * a specific slide. Reuses the lazy `<Thumbnail>` component so an
 * 800-slide deck doesn't fan out to 800 IPC calls on first paint.
 */

import { t } from "../ui/i18n.js";
import { Thumbnail } from "./Thumbnail.js";
import { gridViewStyle, overlayStyle } from "./styles.js";
import type { CachedSlide } from "./types.js";

export interface GridViewProps {
  slideCount: number;
  currentSlide: number;
  cache: Map<number, CachedSlide>;
  aspect: number;
  onSelect: (slide: number) => void;
  getThumbnail: (slide: number) => Promise<CachedSlide | null>;
  /** See `ThumbnailSidebarProps.deckKey` for the rationale. */
  deckKey: string;
}

export function GridView(props: GridViewProps): JSX.Element {
  if (props.slideCount === 0) {
    return <div style={overlayStyle}>{t("viewer.empty")}</div>;
  }
  return (
    <div style={gridViewStyle}>
      {Array.from({ length: props.slideCount }, (_, i) => {
        const n = i + 1;
        return (
          <Thumbnail
            key={`${props.deckKey}::${n}`}
            slide={n}
            active={n === props.currentSlide}
            onClick={() => props.onSelect(n)}
            getThumbnail={props.getThumbnail}
            aspectFallback={props.aspect}
            layout="tile"
          />
        );
      })}
    </div>
  );
}
