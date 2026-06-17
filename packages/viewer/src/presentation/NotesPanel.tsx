/**
 * Footer panel that renders the current slide's speaker notes plus
 * its layout / section labels. Mounted inside the shell when the
 * notes drawer toggle is on.
 */

import { t } from "../ui/i18n.js";
import type { SlideMeta } from "../types.js";
import {
  notesBodyStyle,
  notesEmptyStyle,
  notesHeadingStyle,
  notesMetaStyle,
  notesPanelStyle,
} from "./styles.js";

export interface NotesPanelProps {
  currentSlide: number;
  meta: SlideMeta | null;
}

export function NotesPanel(props: NotesPanelProps): JSX.Element {
  const { currentSlide, meta } = props;
  const heading = meta?.section_name
    ? t("notes.headingWithSection", {
        current: currentSlide,
        section: meta.section_name,
      })
    : t("notes.heading", { current: currentSlide });
  return (
    <div style={notesPanelStyle}>
      <h4 style={notesHeadingStyle}>{heading}</h4>
      {meta?.layout_name ? (
        <div style={notesMetaStyle}>
          {t("notes.layoutLabel", { value: meta.layout_name })}
        </div>
      ) : null}
      {meta?.section_name ? (
        <div style={notesMetaStyle}>
          {t("notes.sectionLabel", { value: meta.section_name })}
        </div>
      ) : null}
      {meta?.notes ? (
        <div style={notesBodyStyle}>{meta.notes}</div>
      ) : (
        <em style={notesEmptyStyle}>{t("notes.empty")}</em>
      )}
    </div>
  );
}
