---
title: slideglance-font — Reference
lang: en
kind: reference
crate: slideglance-font
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-font/src/lib.rs
  - crates/slideglance-font/src/cjk_fallback.rs
  - crates/slideglance-font/src/collector.rs
  - crates/slideglance-font/src/fallback_metrics.rs
  - crates/slideglance-font/src/font_fetcher.rs
  - crates/slideglance-font/src/font_metric.rs
  - crates/slideglance-font/src/font_resolver.rs
  - crates/slideglance-font/src/latin_defaults.rs
  - crates/slideglance-font/src/mapping.rs
  - crates/slideglance-font/src/opentype.rs
  - crates/slideglance-font/src/script_context.rs
  - crates/slideglance-font/src/system_fonts.rs
  - crates/slideglance-font/src/text_engine.rs
  - crates/slideglance-font/src/text_measure.rs
  - crates/slideglance-font/src/text_measurer.rs
  - crates/slideglance-font/src/text_path.rs
  - crates/slideglance-font/src/text_wrap.rs
  - crates/slideglance-font/src/ttc.rs
---

# slideglance-font — Reference

## Module map

| Module | What lives here |
|---|---|
| `opentype` | `FontFace`, `FontError`, `all_face_family_names` |
| `ttc` | `parse_font_data`, `extract_ttc_faces`, `is_ttc`, … |
| `font_resolver` | `FontResolver` trait + the standard chain builders |
| `mapping` | `FontMapping` — authored-name → physical-name table |
| `cjk_fallback` | `get_cjk_fallback_fonts`, `CjkPlatform` |
| `latin_defaults` | `get_latin_os_defaults` |
| `system_fonts` | Native system-font discovery (cfg-gated) |
| `font_fetcher` | `FontFetcher`, `FetcherFontResolver` (Google Fonts) |
| `script_context` | `ScriptFontContext`, `CJK_SCRIPT_CODES` |
| `font_metric` | `FontMetrics` access |
| `fallback_metrics` | OSS metric-compatible fallback table |
| `collector` | `collect_used_fonts`, `ThemeFonts`, `UsedFonts` |
| `text_engine` | `TextEngine`, `TextEngineBuilder`, `RenderMode` |
| `text_measurer` | `TextMeasurer` trait, `OpentypeTextMeasurer`, `HeuristicTextMeasurer`, `FontStyle` |
| `text_measure` | Re-exports of the high-level measurement entry points |
| `text_path` | SVG path conversion for path-mode rendering |
| `text_wrap` | Line-break / wrap algorithm |

## Top-level re-exports

### Font discovery and resolution

- `FontResolver` (trait) — `resolve(name, style) -> Option<Arc<FontFace>>`
- `BufferFontResolver` — owns a pool of raw font byte buffers
- `MappingFontResolver`, `CjkFallbackFontResolver` — chain links
- `standard_resolver_chain(inner, platform) -> Arc<dyn FontResolver>`
- `FontMapping` — authored-name → physical-name table
- `FontFetcher`, `FetcherFontResolver` — Google Fonts pull (Node-only)

### Faces

- `FontFace`, `FontError`, `all_face_family_names`
- `is_ttc`, `extract_first_ttc_face`, `extract_ttc_faces`, `parse_font_data`, `ttc_face_count`

### CJK and Latin fallbacks

- `get_cjk_fallback_fonts`, `CjkPlatform { Windows, Mac, Generic, … }`
- `get_latin_os_defaults`
- `ScriptFontContext`, `CJK_SCRIPT_CODES`

### Metrics

- `FontMetrics`, `get_font_metrics`, `get_metrics_fallback_font`

### Engine

- `TextEngine`, `TextEngineBuilder`, `RenderMode { Text, Path }`

### Measurement

- `TextMeasurer` (trait)
- `OpentypeTextMeasurer` — precise, uses parsed tables
- `HeuristicTextMeasurer` — last-resort, no font data required
- `FontStyle`

### Collectors

- `collect_used_fonts(presentation) -> UsedFonts`
- `ThemeFonts`, `UsedFonts`

## CJK script equality

The CJK fallback selector treats `Jpan`, `Hang`, `Hans`, `Hant`
equally — no Japanese-only special-case. Any change to fallback
policy must update all four scripts atomically. The full list of
script codes is exported as `CJK_SCRIPT_CODES`.

For the complete export list (about 50 items), see
[`src/lib.rs`](../../src/lib.rs).
