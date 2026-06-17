import { type CSSProperties, useEffect, useState } from "react";
import { subscribeLocale, t, type MessageKey } from "./i18n.js";

export interface ShortcutsDialogProps {
  open: boolean;
  onClose: () => void;
}

interface ShortcutRow {
  keys: string;
  descKey: MessageKey;
}

/**
 * Discoverable keyboard-shortcut reference. Mirrors PowerPoint's
 * `?` quick-help convention: a small modal that lists every key the
 * viewer reacts to, grouped by topic.
 *
 * Single source of truth — the actual handlers live in
 * `<PptxPresentation>`'s keyboard `useEffect`. If this list and the
 * handler ever drift, the handler wins; this dialog is a hint.
 */
export function ShortcutsDialog(
  props: ShortcutsDialogProps,
): JSX.Element | null {
  const { open, onClose } = props;
  const [, setLocaleTick] = useState<number>(0);

  useEffect(() => subscribeLocale(() => setLocaleTick((n) => n + 1)), []);

  // Close on Escape — the dialog itself doesn't catch the key
  // (window-level handler), so we don't preventDefault here; we just
  // observe and dismiss.
  useEffect(() => {
    if (!open) return;
    const onKey = (ev: KeyboardEvent): void => {
      if (ev.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  const groups: { titleKey: MessageKey; rows: ShortcutRow[] }[] = [
    {
      titleKey: "shortcuts.groupNavigation",
      rows: [
        { keys: "← / Page Up", descKey: "shortcuts.prevSlide" },
        { keys: "→ / Page Down", descKey: "shortcuts.nextSlide" },
        { keys: "Home", descKey: "shortcuts.firstSlide" },
        { keys: "End", descKey: "shortcuts.lastSlide" },
      ],
    },
    {
      titleKey: "shortcuts.groupView",
      rows: [
        { keys: "⌘/Ctrl + +", descKey: "shortcuts.zoomIn" },
        { keys: "⌘/Ctrl + −", descKey: "shortcuts.zoomOut" },
        { keys: "⌘/Ctrl + 0", descKey: "shortcuts.zoomReset" },
        { keys: "Space + Drag", descKey: "shortcuts.panSlide" },
      ],
    },
    {
      titleKey: "shortcuts.groupSelection",
      rows: [
        { keys: t("shortcuts.click"), descKey: "shortcuts.selectShape" },
        {
          keys: "Shift / ⌘ + " + t("shortcuts.click"),
          descKey: "shortcuts.toggleSelect",
        },
        { keys: t("shortcuts.drag"), descKey: "shortcuts.rubberBand" },
        { keys: "⌘/Ctrl + A", descKey: "shortcuts.selectAll" },
        { keys: "⌘/Ctrl + C", descKey: "shortcuts.copyText" },
        { keys: t("shortcuts.doubleClick"), descKey: "shortcuts.editText" },
        { keys: "Esc", descKey: "shortcuts.clearSelection" },
      ],
    },
    {
      titleKey: "shortcuts.groupOutput",
      rows: [
        { keys: "⌘/Ctrl + F", descKey: "shortcuts.toggleSearch" },
        { keys: "⌘/Ctrl + P", descKey: "shortcuts.print" },
      ],
    },
  ];

  return (
    <div
      style={backdropStyle}
      role="dialog"
      aria-modal="true"
      onClick={onClose}
    >
      <div style={panelStyle} onClick={(ev) => ev.stopPropagation()}>
        <header style={headerStyle}>
          <h2 style={titleStyle}>{t("shortcuts.title")}</h2>
          <button
            style={closeBtnStyle}
            onClick={onClose}
            aria-label={t("common.close")}
            title={t("common.close")}
          >
            ✕
          </button>
        </header>
        <div style={bodyStyle}>
          {groups.map((g) => (
            <section key={g.titleKey} style={groupStyle}>
              <h3 style={groupTitleStyle}>{t(g.titleKey)}</h3>
              <dl style={dlStyle}>
                {g.rows.map((row) => (
                  <div key={row.keys + row.descKey} style={rowStyle}>
                    <dt style={dtStyle}>
                      <kbd style={kbdStyle}>{row.keys}</kbd>
                    </dt>
                    <dd style={ddStyle}>{t(row.descKey)}</dd>
                  </div>
                ))}
              </dl>
            </section>
          ))}
        </div>
      </div>
    </div>
  );
}

const backdropStyle: CSSProperties = {
  position: "fixed",
  inset: 0,
  background: "var(--pptx-shell-dialog-overlay, rgba(0, 0, 0, 0.5))",
  display: "grid",
  placeItems: "center",
  zIndex: 1000,
  padding: 24,
};

const panelStyle: CSSProperties = {
  background: "var(--pptx-shell-dialog-bg, #1f1f23)",
  color: "var(--pptx-shell-dialog-fg, #ececec)",
  border: "1px solid var(--pptx-shell-border, #2a2a30)",
  borderRadius: 8,
  width: "min(640px, 100%)",
  maxHeight: "min(720px, 100%)",
  display: "flex",
  flexDirection: "column",
  boxShadow: "0 12px 40px var(--pptx-shell-shadow, rgba(0, 0, 0, 0.5))",
};

const headerStyle: CSSProperties = {
  padding: "12px 16px",
  borderBottom: "1px solid var(--pptx-shell-border, #2a2a30)",
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
};

const titleStyle: CSSProperties = {
  margin: 0,
  fontSize: 14,
  fontWeight: 600,
};

const closeBtnStyle: CSSProperties = {
  background: "transparent",
  color: "inherit",
  border: "1px solid var(--pptx-shell-border, #2a2a30)",
  borderRadius: 4,
  padding: "2px 8px",
  cursor: "pointer",
};

const bodyStyle: CSSProperties = {
  padding: 16,
  overflowY: "auto",
  display: "grid",
  gap: 16,
};

const groupStyle: CSSProperties = {
  display: "block",
};

const groupTitleStyle: CSSProperties = {
  margin: "0 0 6px",
  fontSize: 11,
  letterSpacing: "0.05em",
  textTransform: "uppercase",
  color: "var(--pptx-shell-status, #888)",
};

const dlStyle: CSSProperties = {
  margin: 0,
  display: "grid",
  gridTemplateColumns: "minmax(140px, auto) 1fr",
  rowGap: 4,
  columnGap: 12,
};

const rowStyle: CSSProperties = {
  display: "contents",
};

const dtStyle: CSSProperties = {
  margin: 0,
};

const ddStyle: CSSProperties = {
  margin: 0,
  fontSize: 12,
  alignSelf: "center",
  color: "var(--pptx-shell-fg, #ddd)",
};

const kbdStyle: CSSProperties = {
  display: "inline-block",
  padding: "2px 8px",
  border: "1px solid var(--pptx-shell-border, #2a2a30)",
  borderRadius: 4,
  background: "var(--pptx-shell-kbd-bg, rgba(255, 255, 255, 0.03))",
  fontSize: 11,
  fontFamily: "ui-monospace, Menlo, monospace",
  color: "inherit",
};
