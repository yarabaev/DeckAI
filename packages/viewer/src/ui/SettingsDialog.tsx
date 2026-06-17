import {
  type CSSProperties,
  type ReactNode,
  useCallback,
  useEffect,
  useState,
} from "react";

import {
  loadSettings,
  saveSettings,
  type RulerUnit,
  type ThemeMode,
  type ViewerSettings,
} from "./settings.js";
import {
  getDetectedLocale,
  subscribeLocale,
  SUPPORTED_LOCALES,
  t,
  type Locale,
  type ResolvedLocale,
} from "./i18n.js";

const LOCALE_DISPLAY_NAMES: Record<ResolvedLocale, string> = {
  en: "English",
  ko: "한국어",
  ja: "日本語",
  "zh-CN": "简体中文",
  "zh-TW": "繁體中文",
  es: "Español",
  fr: "Français",
  de: "Deutsch",
};

export interface SettingsDialogProps {
  open: boolean;
  onClose: () => void;
  /**
   * Display name of the consuming app — usually the brand identifier
   * shown in the host's window title. Defaults to "SlideGlance".
   */
  appName?: string;
  /** Version string surfaced in the About panel. */
  appVersion?: string;
  /**
   * npm package name the host is built on top of. Defaults to
   * `@slideglance/viewer`. Hosts that wrap the viewer in their own
   * brand pass their own package name here.
   */
  npmPackage?: string;
  /**
   * Rust crate compiled to WebAssembly that powers conversion. Defaults
   * to the umbrella `slideglance-wasm` — the Rust → Wasm boundary the
   * viewer actually runs against.
   */
  engineCrate?: string;
  /**
   * URL of the project repository (typically GitHub). Rendered as a
   * clickable link in the About panel.
   */
  repositoryUrl?: string;
  /**
   * Copyright holder displayed in the About panel. Defaults to the
   * SlideGlance project's corporate owner.
   */
  copyrightHolder?: string;
  /**
   * Copyright year shown next to the holder. Defaults to the current
   * UTC year so first-time embedders don't see a stale value.
   */
  copyrightYear?: string;
  /**
   * Lead developer attribution. Defaults to the SlideGlance maintainer.
   */
  developer?: string;
  /**
   * Brand icon shown at the top of the About panel. Hosts that wrap
   * the viewer can pass their own logo here (typically as an inline
   * `<svg>` element to keep the bundle offline-first). Falls back to
   * the bundled SlideGlance mark when omitted.
   */
  appIcon?: ReactNode;
  onSettingsChange?: (settings: ViewerSettings) => void;
}

/**
 * Modal settings panel — React port of `<pptx-settings-dialog>`.
 *
 * Reads/writes to the persistent settings store so changes survive
 * across sessions. Every interaction commits immediately (no
 * separate Save button), matching macOS / Windows preferences
 * conventions.
 */
export function SettingsDialog(props: SettingsDialogProps): JSX.Element | null {
  const {
    open,
    onClose,
    appName = "SlideGlance",
    appVersion = __PPTX_VIEWER_VERSION__,
    npmPackage = "@slideglance/viewer",
    engineCrate = "slideglance-wasm",
    repositoryUrl = "https://github.com/SlideGlance/slideglance",
    copyrightHolder = "SimpleCORE Inc.",
    copyrightYear = String(new Date().getUTCFullYear()),
    developer = "Dmitry Yarabaev",
    appIcon,
    onSettingsChange,
  } = props;
  const [settings, setSettings] = useState<ViewerSettings>(() =>
    loadSettings(),
  );
  const [, setLocaleTick] = useState<number>(0);

  // Subscribe to locale changes so the dialog re-renders when the
  // user picks a new language.
  useEffect(() => {
    const unsub = subscribeLocale(() => setLocaleTick((n) => n + 1));
    return unsub;
  }, []);

  // Esc to close.
  useEffect(() => {
    if (!open) return;
    const onKey = (ev: KeyboardEvent): void => {
      if (ev.key === "Escape") {
        ev.preventDefault();
        onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  const updateSetting = useCallback(
    <K extends keyof ViewerSettings>(key: K, value: ViewerSettings[K]) => {
      const next = saveSettings({ [key]: value } as Partial<ViewerSettings>);
      setSettings(next);
      onSettingsChange?.(next);
    },
    [onSettingsChange],
  );

  if (!open) return null;

  const themeOptions: Array<{
    value: ThemeMode;
    label: string;
    desc: string;
  }> = [
    {
      value: "auto",
      label: t("dialog.themeAuto"),
      desc: t("dialog.themeAutoDesc"),
    },
    {
      value: "light",
      label: t("dialog.themeLight"),
      desc: t("dialog.themeLightDesc"),
    },
    {
      value: "dark",
      label: t("dialog.themeDark"),
      desc: t("dialog.themeDarkDesc"),
    },
    {
      value: "high-contrast",
      label: t("dialog.themeHighContrast"),
      desc: t("dialog.themeHighContrastDesc"),
    },
  ];
  const detected = getDetectedLocale();
  const localeOptions: Array<{ value: Locale; label: string }> = [
    {
      value: "auto",
      label: t("dialog.languageAuto", {
        detected: LOCALE_DISPLAY_NAMES[detected],
      }),
    },
    ...SUPPORTED_LOCALES.map((code) => ({
      value: code as Locale,
      label: LOCALE_DISPLAY_NAMES[code],
    })),
  ];

  return (
    <div style={hostStyle} role="presentation">
      <div style={backdropStyle} onClick={onClose} />
      <div
        style={panelStyle}
        role="dialog"
        aria-modal="true"
        aria-label={t("dialog.viewerSettingsAriaLabel")}
      >
        <header style={headerStyle}>
          <h2 style={titleStyle}>{t("dialog.title")}</h2>
          <div style={{ flex: 1 }} />
          <button
            style={closeButtonStyle}
            onClick={onClose}
            title={t("common.close")}
          >
            ✕
          </button>
        </header>
        <div style={bodyStyle}>
          <Section label={t("dialog.appearance")}>
            <Row label={t("dialog.theme")} desc={t("dialog.themeDesc")} />
            <RadioGrid
              ariaLabel={t("dialog.theme")}
              options={themeOptions.map((opt) => ({
                value: opt.value,
                label: opt.label,
                desc: opt.desc,
              }))}
              selected={settings.themeMode}
              onSelect={(value) =>
                updateSetting("themeMode", value as ThemeMode)
              }
            />
          </Section>

          <Section label={t("dialog.language")}>
            <Row label={t("dialog.language")} desc={t("dialog.languageDesc")} />
            <RadioGrid
              ariaLabel={t("dialog.language")}
              options={localeOptions.map((opt) => ({
                value: opt.value,
                label: opt.label,
              }))}
              selected={settings.locale}
              onSelect={(value) => updateSetting("locale", value as Locale)}
            />
          </Section>

          <Section label={t("dialog.ruler")}>
            <label style={checkboxRowStyle}>
              <input
                type="checkbox"
                checked={settings.showRuler}
                onChange={(e) => updateSetting("showRuler", e.target.checked)}
              />
              {t("dialog.rulerShow")}
            </label>
            <Row
              label={t("dialog.rulerUnitLabel")}
              desc={t("dialog.rulerUnitDesc")}
              style={{ marginTop: 6 }}
            />
            <RadioGrid
              ariaLabel={t("dialog.rulerUnitLabel")}
              disabled={!settings.showRuler}
              options={(["cm", "px"] as RulerUnit[]).map((u) => ({
                value: u,
                label:
                  u === "cm"
                    ? t("dialog.rulerUnitCm")
                    : t("dialog.rulerUnitPx"),
                desc:
                  u === "cm"
                    ? t("dialog.rulerUnitCmDesc")
                    : t("dialog.rulerUnitPxDesc"),
              }))}
              selected={settings.rulerUnit}
              onSelect={(value) =>
                updateSetting("rulerUnit", value as RulerUnit)
              }
            />
          </Section>

          <Section label={t("dialog.about")}>
            {/* Brand header — large icon + app name + version. Pulled out
                of the <dl> so the icon can sit alongside two stacked
                text rows without breaking the dt/dd pairing below. */}
            <div style={aboutBrandStyle}>
              <div style={aboutBrandIconStyle} aria-hidden="true">
                {appIcon ?? <SlideGlanceMark />}
              </div>
              <div style={aboutBrandTextStyle}>
                <div style={aboutBrandNameStyle}>{appName}</div>
                <div style={aboutBrandVersionStyle}>v{appVersion}</div>
              </div>
            </div>
            <dl style={aboutStyle}>
              {/* Runtime block — appName / version are shown in the brand
                  header above, so the dl picks up at the rendering row. */}
              <dt style={aboutDtStyle}>{t("dialog.aboutRendering")}</dt>
              <dd style={aboutDdStyle}>{t("dialog.aboutRenderingValue")}</dd>

              {/* Distribution surfaces — explicitly labelled by ecosystem
                  so the user can tell at a glance which package manager
                  the identifier belongs to. */}
              <dt style={aboutDtStyle}>{t("dialog.aboutNpmPackage")}</dt>
              <dd style={aboutDdStyle}>
                <code style={aboutCodeStyle}>{npmPackage}</code>
              </dd>
              <dt style={aboutDtStyle}>{t("dialog.aboutEngine")}</dt>
              <dd style={aboutDdStyle}>
                <code style={aboutCodeStyle}>{engineCrate}</code>
                <span style={aboutHintStyle}> (Rust crate → WebAssembly)</span>
              </dd>

              {/* Legal block */}
              <dt style={aboutDtStyle}>{t("dialog.aboutLicense")}</dt>
              <dd style={aboutDdStyle}>MIT</dd>
              <dt style={aboutDtStyle}>{t("dialog.aboutCopyright")}</dt>
              <dd style={aboutDdStyle}>
                © {copyrightYear} {copyrightHolder}
              </dd>
              <dt style={aboutDtStyle}>{t("dialog.aboutDeveloper")}</dt>
              <dd style={aboutDdStyle}>{developer}</dd>

              {/* Repository link — last so it visually anchors the
                  block as the canonical source of truth. */}
              <dt style={aboutDtStyle}>{t("dialog.aboutRepository")}</dt>
              <dd style={aboutDdStyle}>
                <a
                  href={repositoryUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  style={aboutLinkStyle}
                >
                  {repositoryUrl.replace(/^https?:\/\//, "")}
                </a>
              </dd>
            </dl>
          </Section>
        </div>
        <footer style={footerStyle}>
          <button style={primaryButtonStyle} onClick={onClose}>
            {t("common.close")}
          </button>
        </footer>
      </div>
    </div>
  );
}

interface SectionProps {
  label: string;
  children: React.ReactNode;
}

function Section(props: SectionProps): JSX.Element {
  return (
    <section style={sectionStyle} aria-label={props.label}>
      <h3 style={sectionTitleStyle}>{props.label}</h3>
      {props.children}
    </section>
  );
}

interface RowProps {
  label: string;
  desc?: string;
  style?: CSSProperties;
}

function Row(props: RowProps): JSX.Element {
  return (
    <div style={{ ...rowStyle, ...props.style }}>
      <div>
        <label style={rowLabelStyle}>{props.label}</label>
        {props.desc && <span style={rowDescStyle}>{props.desc}</span>}
      </div>
    </div>
  );
}

interface RadioGridProps {
  ariaLabel: string;
  disabled?: boolean;
  options: Array<{ value: string; label: string; desc?: string }>;
  selected: string;
  onSelect: (value: string) => void;
}

function RadioGrid(props: RadioGridProps): JSX.Element {
  return (
    <div style={radioGridStyle} role="radiogroup" aria-label={props.ariaLabel}>
      {props.options.map((opt) => {
        const isSelected = props.selected === opt.value;
        return (
          <button
            key={opt.value}
            style={
              isSelected
                ? { ...radioButtonStyle, ...radioButtonSelectedStyle }
                : radioButtonStyle
            }
            role="radio"
            aria-checked={isSelected}
            disabled={props.disabled}
            onClick={() => props.onSelect(opt.value)}
          >
            <div>{opt.label}</div>
            {opt.desc && <div style={radioDescStyle}>{opt.desc}</div>}
          </button>
        );
      })}
    </div>
  );
}

const hostStyle: CSSProperties = {
  position: "fixed",
  inset: 0,
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  zIndex: 1000,
  font: "13px system-ui, -apple-system, sans-serif",
};

const backdropStyle: CSSProperties = {
  position: "absolute",
  inset: 0,
  background: "var(--pptx-shell-dialog-overlay, rgba(0, 0, 0, 0.55))",
};

const panelStyle: CSSProperties = {
  position: "relative",
  // Wide enough to fit 4 radio cells per row in the language /
  // theme grids — `radioGridStyle` uses `minmax(160px, 1fr)` and
  // 8 px gaps, so 4 cells need at least 4 * 160 + 3 * 8 = 664 px of
  // grid track. Panel chrome (20 px padding on each side, scrollbar
  // gutter) lifts that to ~720 px; 820 px gives the cells room to
  // breathe so longer labels like "繁體中文" / "자동 (한국어)" don't
  // crowd the second-row description text.
  width: "min(820px, 92vw)",
  maxHeight: "86vh",
  display: "flex",
  flexDirection: "column",
  background: "var(--pptx-shell-dialog-bg, #1f1f23)",
  color: "var(--pptx-shell-dialog-fg, #ececec)",
  border: "1px solid var(--pptx-shell-border, #2a2a30)",
  borderRadius: 8,
  boxShadow: "0 16px 48px var(--pptx-shell-shadow, rgba(0, 0, 0, 0.5))",
  overflow: "hidden",
};

const headerStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 8,
  padding: "12px 16px",
  borderBottom: "1px solid var(--pptx-shell-border, #2a2a30)",
};

const titleStyle: CSSProperties = {
  margin: 0,
  fontSize: 14,
  fontWeight: 600,
};

const closeButtonStyle: CSSProperties = {
  background: "transparent",
  color: "inherit",
  border: "1px solid transparent",
  borderRadius: 4,
  padding: "4px 8px",
  cursor: "pointer",
  font: "inherit",
};

const bodyStyle: CSSProperties = {
  flex: "1 1 auto",
  overflowY: "auto",
  padding: "16px 20px",
};

const sectionStyle: CSSProperties = {
  marginBottom: 24,
};

const sectionTitleStyle: CSSProperties = {
  margin: "0 0 12px",
  fontSize: 12,
  letterSpacing: "0.05em",
  textTransform: "uppercase",
  color: "var(--pptx-shell-status, #aaa)",
};

const rowStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  marginBottom: 8,
};

const rowLabelStyle: CSSProperties = {
  display: "block",
  fontWeight: 600,
  fontSize: 13,
  marginBottom: 2,
};

const rowDescStyle: CSSProperties = {
  display: "block",
  fontSize: 11,
  color: "var(--pptx-shell-status, #888)",
};

const checkboxRowStyle: CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  gap: 8,
  fontSize: 13,
  cursor: "pointer",
};

const radioGridStyle: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))",
  gap: 8,
  marginTop: 6,
};

const radioButtonStyle: CSSProperties = {
  background: "transparent",
  color: "inherit",
  border: "1px solid var(--pptx-shell-border, #2a2a30)",
  borderRadius: 6,
  padding: "10px 12px",
  cursor: "pointer",
  font: "inherit",
  textAlign: "left",
};

const radioButtonSelectedStyle: CSSProperties = {
  // Selected state is signalled via background tint only — earlier
  // versions also bumped `borderColor` to the accent, but the 1px
  // accent stroke desaturates to off-white on dark themes / JPEG
  // captures and was perceived as a stray outline. Background tint
  // is sufficient and matches the toolbar's active-icon convention.
  background: "var(--pptx-shell-accent-soft, #1d2738)",
};

const radioDescStyle: CSSProperties = {
  fontSize: 11,
  color: "var(--pptx-shell-status, #888)",
  marginTop: 4,
};

const aboutBrandStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 14,
  marginBottom: 14,
};

const aboutBrandIconStyle: CSSProperties = {
  width: 56,
  height: 56,
  flex: "0 0 56px",
  borderRadius: 12,
  overflow: "hidden",
  background: "var(--pptx-shell-code-bg, rgba(255, 255, 255, 0.06))",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
};

const aboutBrandTextStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 2,
  minWidth: 0,
};

const aboutBrandNameStyle: CSSProperties = {
  fontSize: 16,
  fontWeight: 600,
  letterSpacing: 0.1,
  overflow: "hidden",
  textOverflow: "ellipsis",
  whiteSpace: "nowrap",
};

const aboutBrandVersionStyle: CSSProperties = {
  fontSize: 12,
  color: "var(--pptx-shell-status, #888)",
  fontVariantNumeric: "tabular-nums",
};

const aboutStyle: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "auto 1fr",
  gap: "4px 16px",
  margin: 0,
  fontSize: 12,
};

const aboutDtStyle: CSSProperties = {
  color: "var(--pptx-shell-status, #888)",
  margin: 0,
  whiteSpace: "nowrap",
};
const aboutDdStyle: CSSProperties = {
  margin: 0,
  fontVariantNumeric: "tabular-nums",
  // Long values (npm package paths, repo URLs, developer email) wrap
  // gracefully instead of overflowing the dialog width.
  overflowWrap: "anywhere",
};
const aboutCodeStyle: CSSProperties = {
  fontFamily:
    "ui-monospace, SFMono-Regular, Menlo, Monaco, 'Cascadia Code', 'Roboto Mono', Consolas, monospace",
  fontSize: 11,
  background: "var(--pptx-shell-code-bg, rgba(255, 255, 255, 0.06))",
  padding: "1px 6px",
  borderRadius: 3,
};
const aboutHintStyle: CSSProperties = {
  color: "var(--pptx-shell-status, #888)",
  fontSize: 11,
};
const aboutLinkStyle: CSSProperties = {
  color: "var(--pptx-shell-accent, #6aa3ff)",
  textDecoration: "none",
};

const footerStyle: CSSProperties = {
  display: "flex",
  justifyContent: "flex-end",
  padding: "12px 16px",
  borderTop: "1px solid var(--pptx-shell-border, #2a2a30)",
};

const primaryButtonStyle: CSSProperties = {
  background: "var(--pptx-shell-active, #3a3a44)",
  color: "var(--pptx-shell-fg, #ececec)",
  border: "1px solid var(--pptx-shell-accent, #6aa3ff)",
  borderRadius: 4,
  padding: "6px 16px",
  cursor: "pointer",
  font: "inherit",
};

/**
 * Default SlideGlance brand mark — an inline SVG so the bundle stays
 * offline-first (no external asset, no peer dependency, no loader
 * round-trip). Hosts that pass {@link SettingsDialogProps.appIcon}
 * override this with their own logo.
 */
function SlideGlanceMark(): JSX.Element {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 1024 1024"
      width="100%"
      height="100%"
      role="img"
    >
      <rect
        x="0"
        y="0"
        width="1024"
        height="1024"
        rx="232"
        ry="232"
        fill="#f3f4f6"
      />
      <g
        stroke="#6b7280"
        strokeWidth="40"
        strokeLinejoin="round"
        strokeLinecap="round"
        fill="none"
      >
        <rect x="124" y="184" width="776" height="656" rx="48" ry="48" />
        <line x1="144" y1="288" x2="880" y2="288" />
      </g>
      <g fill="#6b7280">
        <circle cx="208" cy="236" r="14" />
        <circle cx="256" cy="236" r="14" />
        <circle cx="304" cy="236" r="14" />
      </g>
      <g fill="#c43e1c">
        <rect x="232" y="568" width="96" height="160" rx="12" ry="12" />
        <rect x="372" y="464" width="96" height="264" rx="12" ry="12" />
        <rect x="512" y="380" width="96" height="348" rx="12" ry="12" />
        <rect x="652" y="512" width="96" height="216" rx="12" ry="12" />
      </g>
    </svg>
  );
}
