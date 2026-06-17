import type { DefaultTextStyle } from "./types.ts";

export const DEFAULT_FONT_FAMILY = "Noto Sans JP";

export interface ResolvedDefaultTextStyle extends DefaultTextStyle {
  fontFamily: string;
}

export function normalizeDefaultTextStyle(
  style?: DefaultTextStyle,
): ResolvedDefaultTextStyle {
  const fontFamily = style?.fontFamily?.trim();
  return {
    ...style,
    fontFamily:
      fontFamily && fontFamily.length > 0 ? fontFamily : DEFAULT_FONT_FAMILY,
  };
}

export function mergeDefaultTextStyles(
  base?: DefaultTextStyle,
  override?: DefaultTextStyle,
): DefaultTextStyle | undefined {
  const merged = {
    ...base,
    ...override,
  };

  return Object.values(merged).some((value) => value !== undefined)
    ? merged
    : undefined;
}

export function resolveFontFamily(
  fontFamily: string | undefined,
  defaultTextStyle?: DefaultTextStyle,
): string {
  const resolved = fontFamily?.trim();
  if (resolved) return resolved;

  const defaultFontFamily = defaultTextStyle?.fontFamily?.trim();
  if (defaultFontFamily) return defaultFontFamily;

  return DEFAULT_FONT_FAMILY;
}

export function resolveTextStyleValue<T>(
  value: T | undefined,
  defaultValue: T | undefined,
  fallback: T,
): T {
  return value ?? defaultValue ?? fallback;
}
