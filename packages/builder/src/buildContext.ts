import type { TextMeasurementMode } from "./calcYogaLayout/measureText.ts";
import type { DefaultTextStyle } from "./types.ts";
import {
  normalizeDefaultTextStyle,
  type ResolvedDefaultTextStyle,
} from "./defaultTextStyle.ts";
import { DiagnosticCollector } from "./diagnostics.ts";
import type { ImageSrcGuardOptions } from "./options.ts";
import {
  createMeasurer,
  type TextMeasurer,
} from "./calcYogaLayout/fontLoader.ts";

/** Resolved security configuration threaded through the build pipeline. */
export interface SecurityConfig {
  /** Allowed URL schemes for <A href>. Always populated (defaults applied). */
  allowedHrefSchemes: ReadonlySet<string>;
  /** Optional guard for <Image src> / <Master backgroundPath>. Undefined = disabled (OD3). */
  imageSrcGuard: ImageSrcGuardOptions | undefined;
}

export interface BuildContext {
  textMeasurementMode: TextMeasurementMode;
  defaultTextStyle: ResolvedDefaultTextStyle;
  imageSizeCache: Map<string, { widthPx: number; heightPx: number }>;
  imageDataCache: Map<string, string>;
  iconRasterCache: Map<string, string>;
  diagnostics: DiagnosticCollector;
  security: SecurityConfig;
  /** Fallback BCP 47 lang tag for text runs without an explicit lang. Undefined = no fallback. */
  defaultLang: string | undefined;
  /**
   * Per-build text measurer. Carries the bundled fonts (Noto Sans JP,
   * Pretendard — both weights) plus any caller-supplied TTF/OTF buffers
   * passed via the `fonts` build option. Shared by `calcYogaLayout`
   * (wrap decisions) and the lint pass (overflow verification) so both
   * layers measure with the same glyph metrics the renderer will paint.
   */
  measurer: TextMeasurer;
}

const DEFAULT_HREF_SCHEMES = ["https:", "http:", "mailto:", "tel:"];

export interface CreateBuildContextOptions {
  textMeasurementMode?: TextMeasurementMode;
  defaultTextStyle?: DefaultTextStyle;
  allowedHrefSchemes?: string[];
  imageSrcGuard?: ImageSrcGuardOptions;
  defaultLang?: string;
  /**
   * Extra TTF/OTF buffers to register with the per-build text measurer
   * alongside the bundled fonts. When omitted, the measurer is the
   * bundled-only singleton and unknown families fall through to
   * Pretendard / Noto Sans JP via {@link pickBundledFontForText}.
   */
  fonts?: Uint8Array[];
}

export function createBuildContext(
  options: CreateBuildContextOptions = {},
): BuildContext {
  const mergedSchemes = options.allowedHrefSchemes
    ? [...DEFAULT_HREF_SCHEMES, ...options.allowedHrefSchemes]
    : DEFAULT_HREF_SCHEMES;
  return {
    textMeasurementMode: options.textMeasurementMode ?? "auto",
    defaultTextStyle: normalizeDefaultTextStyle(options.defaultTextStyle),
    imageSizeCache: new Map(),
    imageDataCache: new Map(),
    iconRasterCache: new Map(),
    diagnostics: new DiagnosticCollector(),
    security: {
      allowedHrefSchemes: new Set(mergedSchemes),
      imageSrcGuard: options.imageSrcGuard,
    },
    defaultLang: options.defaultLang,
    measurer: createMeasurer(options.fonts),
  };
}
