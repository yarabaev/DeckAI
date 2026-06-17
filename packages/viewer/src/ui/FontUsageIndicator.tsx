/**
 * Status-bar indicator that surfaces the deck's font fallback mapping.
 *
 * For every typeface referenced by the deck, the indicator probes the
 * SVG `font-family` chain via `document.fonts.check()` to identify the
 * actually-rendered font in the current browser. When at least one
 * authored typeface falls back to a substitute, a warning triangle
 * appears in the status bar; clicking it expands a popover with the
 * full mapping ("Calibri → Carlito", "맑은 고딕 → Noto Sans KR", …) so
 * the user can decide whether to install the original font.
 *
 * The probe runs once on mount and again whenever `fontUsage` or the
 * `FontFaceSet` changes. The mapping is exact about what we know
 * ("authored name X is being rendered as Y") and silent about what
 * we don't (PowerPoint-side line break parity).
 */

import {
  type CSSProperties,
  type ReactNode,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { Warning, X } from "@phosphor-icons/react";

import type { TypefaceUsage } from "../types.js";
import { t } from "./i18n.js";

/**
 * One row in the rendered mapping table.
 * `effective` is the first chain entry that `document.fonts.check()`
 * confirmed as installed; `null` means none of the chain entries are
 * installed and the browser will fall back to the generic family.
 */
export interface FontMappingRow {
  requested: string;
  effective: string | null;
  isSubstitute: boolean;
}

export interface FontUsageIndicatorProps {
  /** Per-typeface usage report from the slide controller. */
  fontUsage: TypefaceUsage[];
  /**
   * Optional style override — the host status bar passes its compact
   * icon-button style so the indicator looks the same as siblings
   * (notes, sidebar, view-mode, zoom buttons).
   */
  buttonStyle?: CSSProperties;
  /** Style applied when the popover is open (active state). */
  buttonActiveStyle?: CSSProperties;
}

/**
 * Probe `document.fonts.check()` for every entry in `chain` and return
 * the first one that's installed.
 *
 * `document.fonts.check(font)` uses the CSS Font Loading API. Quoting
 * is required for family names that contain spaces; we always quote
 * to keep the call site uniform.
 */
function findEffectiveFamily(chain: string[]): string | null {
  if (typeof document === "undefined" || !document.fonts) {
    return null;
  }
  for (const family of chain) {
    if (family.length === 0) continue;
    try {
      // Browsers reject family strings with unescaped quotes; we use
      // double quotes inside the spec string and rely on family names
      // not containing them (consistent with build_font_family_value
      // on the Rust side, which escapes XML-unsafe chars).
      const escaped = family.replace(/"/g, '\\"');
      if (document.fonts.check(`12px "${escaped}"`)) {
        return family;
      }
    } catch {
      // Malformed family name — skip and try the next entry.
    }
  }
  return null;
}

/**
 * Compute the mapping rows from the raw usage report. Pure function so
 * we can re-evaluate it whenever the `FontFaceSet` notifies of a load.
 */
function computeMappingRows(usage: TypefaceUsage[]): FontMappingRow[] {
  return usage
    .filter((entry) => entry.requested.length > 0)
    .map((entry) => {
      // The full chain we probe is: requested → fallback chain.
      // `requested` may already appear in `fallbackChain` (the renderer
      // puts it first), but include it explicitly to be defensive.
      const probeChain =
        entry.fallbackChain[0] === entry.requested
          ? entry.fallbackChain
          : [entry.requested, ...entry.fallbackChain];
      const effective = findEffectiveFamily(probeChain);
      const isSubstitute = effective !== null && effective !== entry.requested;
      return { requested: entry.requested, effective, isSubstitute };
    });
}

export function FontUsageIndicator({
  fontUsage,
  buttonStyle,
  buttonActiveStyle,
}: FontUsageIndicatorProps): ReactNode {
  const [open, setOpen] = useState(false);
  const [rows, setRows] = useState<FontMappingRow[]>(() =>
    computeMappingRows(fontUsage),
  );
  const containerRef = useRef<HTMLDivElement | null>(null);

  // Re-run the probe when the usage list changes or the FontFaceSet
  // emits a load — newly-loaded `@font-face` entries (deck embedded
  // fonts) flip `document.fonts.check()` from false to true and the
  // mapping needs to reflect that.
  useEffect(() => {
    setRows(computeMappingRows(fontUsage));
    if (typeof document === "undefined" || !document.fonts) return;
    const refresh = () => setRows(computeMappingRows(fontUsage));
    document.fonts.addEventListener("loadingdone", refresh);
    return () => {
      document.fonts.removeEventListener("loadingdone", refresh);
    };
  }, [fontUsage]);

  // Close the popover when clicking outside.
  useEffect(() => {
    if (!open) return;
    function onDocClick(ev: MouseEvent) {
      const root = containerRef.current;
      if (root && !root.contains(ev.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", onDocClick);
    return () => document.removeEventListener("mousedown", onDocClick);
  }, [open]);

  // Only typefaces that diverge from the authored name belong in the
  // popover — listing rows where `requested === effective` is noise
  // (and includes false positives from macOS CoreText's lenient
  // family-name aliasing of CJK fonts like "맑은 고딕"
  // → Apple SD Gothic Neo). A row qualifies when either:
  //   - the browser picked a different chain entry (`isSubstitute`), or
  //   - no chain entry is installed at all (`effective === null` →
  //     generic CSS fallback handles it, equally a substitution).
  const interestingRows = useMemo(
    () => rows.filter((r) => r.isSubstitute || r.effective === null),
    [rows],
  );

  const toggle = useCallback(() => setOpen((o) => !o), []);

  // No substitutions found — hide the indicator entirely so the
  // status bar stays uncluttered. Showing "all matched" is dishonest
  // because the browser-side check we rely on
  // (`document.fonts.check`) treats CoreText / Chrome alias matches
  // (e.g. "맑은 고딕" → Apple SD Gothic Neo) as legitimate matches —
  // we'd be lying about which fonts the user really has installed.
  if (interestingRows.length === 0) return null;

  const label = t("fontUsage.substituteCount", {
    count: interestingRows.length,
  });

  return (
    <div ref={containerRef} style={containerStyle}>
      <button
        style={open ? { ...buttonActiveStyle } : { ...buttonStyle }}
        onClick={toggle}
        title={label}
        aria-label={label}
        aria-pressed={open}
        aria-haspopup="dialog"
      >
        <Warning size={14} weight={open ? "fill" : "regular"} color="#e0a000" />
      </button>

      {open && (
        <div
          style={popoverStyle}
          role="dialog"
          aria-label={t("fontUsage.title")}
        >
          <header style={popoverHeaderStyle}>
            <span style={popoverTitleStyle}>{t("fontUsage.title")}</span>
            <button
              style={popoverCloseStyle}
              onClick={() => setOpen(false)}
              aria-label={t("fontUsage.close")}
              title={t("fontUsage.close")}
            >
              <X size={12} weight="bold" />
            </button>
          </header>
          <div style={popoverBodyStyle}>
            <table style={tableStyle}>
              <thead>
                <tr>
                  <th style={thStyle}>{t("fontUsage.headerRequested")}</th>
                  <th style={thStyle}>{t("fontUsage.headerEffective")}</th>
                </tr>
              </thead>
              <tbody>
                {interestingRows.map((row) => (
                  <tr key={row.requested}>
                    <td style={tdStyle}>{row.requested}</td>
                    <td style={tdStyle}>
                      {row.effective === null ? (
                        <span style={mutedStyle}>
                          {t("fontUsage.systemFallback")}
                        </span>
                      ) : (
                        <span style={substituteStyle}>{row.effective}</span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}

const containerStyle: CSSProperties = {
  position: "relative",
  display: "inline-flex",
  alignItems: "center",
};

const popoverStyle: CSSProperties = {
  position: "absolute",
  bottom: "calc(100% + 4px)",
  right: 0,
  minWidth: 320,
  maxWidth: 480,
  maxHeight: 360,
  overflow: "hidden",
  display: "flex",
  flexDirection: "column",
  background: "var(--pptx-shell-bg, #1f1f24)",
  color: "var(--pptx-shell-fg, #e6e6ea)",
  border: "1px solid var(--pptx-shell-border, #2c2c34)",
  borderRadius: 6,
  boxShadow: "0 8px 24px rgba(0, 0, 0, 0.4)",
  zIndex: 1000,
  fontSize: 12,
};

const popoverHeaderStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  padding: "8px 12px",
  borderBottom: "1px solid var(--pptx-shell-border, #2c2c34)",
};

const popoverTitleStyle: CSSProperties = {
  fontWeight: 600,
};

const popoverCloseStyle: CSSProperties = {
  width: 20,
  height: 20,
  display: "inline-flex",
  alignItems: "center",
  justifyContent: "center",
  background: "transparent",
  color: "inherit",
  border: 0,
  borderRadius: 3,
  cursor: "pointer",
  padding: 0,
};

const popoverBodyStyle: CSSProperties = {
  overflow: "auto",
  padding: "4px 0",
};

const tableStyle: CSSProperties = {
  width: "100%",
  borderCollapse: "collapse",
  fontSize: 12,
};

const thStyle: CSSProperties = {
  textAlign: "left",
  padding: "6px 12px",
  fontWeight: 500,
  color: "var(--pptx-shell-muted, #9a9aa3)",
  borderBottom: "1px solid var(--pptx-shell-border, #2c2c34)",
};

const tdStyle: CSSProperties = {
  padding: "5px 12px",
  borderBottom: "1px solid var(--pptx-shell-border-soft, #26262e)",
  whiteSpace: "nowrap",
  overflow: "hidden",
  textOverflow: "ellipsis",
};

const substituteStyle: CSSProperties = {
  color: "#e0a000",
};

const mutedStyle: CSSProperties = {
  color: "var(--pptx-shell-muted, #9a9aa3)",
  fontStyle: "italic",
};
