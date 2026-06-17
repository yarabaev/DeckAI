//! End-to-end PPTX -> SVG / PNG pipelines.
//!
//! Composes [`crate::parse_pptx`] with [`slideglance_renderer::render_slide_to_svg`]
//! (and [`slideglance_png::svg_to_png`] for the PNG variant).
//!
//! Both functions:
//! - filter slides by 1-based number when `slides` is set,
//! - merge master / layout / slide elements (placeholder shapes from
//!   master and layout are dropped — they are templates, not visible
//!   content), and
//! - thread font / measurement context through to the renderer.
//!
//! `convert_to_png` requires a caller-supplied [`slideglance_font::FontResolver`]
//! so resvg can rasterize glyph outlines; without one the call returns
//! [`ConvertError::FontResolverRequiredForPng`].
//!
//! Notes / layout name / section name are surfaced on each result.

use std::collections::HashMap;

use std::sync::Arc;

use slideglance_font::{
    BufferFontResolver, CjkPlatform, FontFace, FontMapping, FontResolver, HeuristicTextMeasurer,
    OpentypeTextMeasurer, RenderMode, ScriptFontContext, TextMeasurer,
};
use slideglance_model::{Presentation, SlideElement};
use slideglance_parser::PptxArchive;
use slideglance_png::{svg_to_png, FontData, PngOptions};
use slideglance_renderer::render_slide_to_svg;

use crate::embedded_fonts::{
    additional_to_faces, extract_embedded_faces, render_embedded_font_defs, EmbedFormat,
    FontBytesCache, UsedCodepoints,
};
use crate::parse_pptx;

mod types;
pub use types::{ConvertError, ConvertOptions, FontConfig, SlideImage, SlideSvg};

/// Parse a PPTX byte stream and render every (or `options.slides`)
/// slide to a self-contained SVG document.
///
/// # Errors
///
/// - [`ConvertError::Pptx`] if the archive can't be parsed.
/// - [`ConvertError::Renderer`] if the renderer rejects any slide.
pub fn convert_to_svg(
    bytes: impl Into<Vec<u8>>,
    options: &ConvertOptions<'_>,
) -> Result<Vec<SlideSvg>, ConvertError> {
    // Materialize the byte buffer so we can both parse the model and
    // re-open the archive for embedded font extraction without
    // requiring the caller to hand us the bytes twice.
    let buffer: Vec<u8> = bytes.into();

    // Parse the model first so we can walk all slide runs and collect the
    // codepoints used per typeface before subsetting the embedded fonts.
    let presentation = parse_pptx(buffer.clone())?;

    // R-C4: walk every slide run and accumulate codepoints per typeface.
    // This enables subset_font_bytes to remove glyphs not referenced by
    // the deck, reducing the base64 payload for each @font-face rule.
    let used_codepoints = collect_used_codepoints(&presentation);

    // Collect every face that should land in the SVG `<defs>`:
    // (1) the deck's own embedded fonts and (2) caller-supplied
    // `inline_fonts`. Either source alone is sufficient — having
    // both is fine; faces with the same typeface×variant simply emit
    // duplicate `@font-face` rules (the last one wins per CSS spec).
    let mut all_faces: Vec<crate::embedded_fonts::EmbeddedFontFace> = Vec::new();
    if options.fonts.embed_deck_fonts {
        if let Ok(faces) =
            build_embedded_face_list(&buffer, &used_codepoints, options.fonts.embed_format)
        {
            all_faces.extend(faces);
        }
    }
    if !options.fonts.inline_fonts.is_empty() {
        all_faces.extend(additional_to_faces(&options.fonts.inline_fonts));
    }
    let embedded_defs = render_embedded_font_defs(&all_faces);
    // When the caller leaves `measurer` unset, build an Opentype
    // measurer from real font byte buffers (PPTX embedded fonts +
    // caller-supplied `inline_fonts`) so wrap decisions use actual
    // glyph advances instead of the heuristic 0.95em CJK proxy. The
    // heuristic over-measures condensed Korean / Japanese sans-serifs
    // by ~7-10 %, splitting tight one-line labels at the wrong
    // codepoint (test deck slide 10: "한국농어촌공사 스마트농업처"
    // forced to break at "농업/처"). The measurer also covers caller-
    // supplied measurement-only buffers, so the same accuracy applies
    // to host system fonts the CLI / `dir_to_png` example pre-loads.
    let auto_measurer = if options.measurer.is_none() {
        build_auto_opentype_measurer(&all_faces, &options.fonts.measurement_only_fonts)
    } else {
        None
    };
    // Build an embedded-fonts BufferFontResolver from the same face byte
    // buffers we ship in `<defs>` so path-mode glyph extraction picks them
    // up exactly. Without this, the renderer measures with embedded fonts
    // (via the OpentypeTextMeasurer above) but draws glyphs from whatever
    // host fallback the user passes in `fonts.resolver` — a 1-2 px offset
    // on bold East-Asian text in cells with anchor=ctr (slide 24 side
    // labels were the canary).
    let embedded_resolver = build_embedded_buffer_resolver(&all_faces);
    let caller_resolver: Option<&dyn FontResolver> = options
        .fonts
        .resolver
        .as_deref()
        .map(|r| r as &dyn FontResolver);
    let augmented_resolver: Option<EmbeddedAwareResolver<'_>> = if embedded_resolver.is_empty() {
        None
    } else {
        Some(EmbeddedAwareResolver {
            embedded: embedded_resolver,
            fallback: caller_resolver,
        })
    };
    let resolver_for_render: Option<&dyn FontResolver> = augmented_resolver
        .as_ref()
        .map(|r| r as &dyn FontResolver)
        .or(caller_resolver);
    let measurer_for_render: Option<&dyn TextMeasurer> = auto_measurer
        .as_ref()
        .map(|m| m as &dyn TextMeasurer)
        .or(options.measurer);
    let mut slides = render_all_slides(
        &presentation,
        options,
        resolver_for_render,
        measurer_for_render,
    )?;
    if !embedded_defs.is_empty() {
        // Embedded font `<defs>` can be tens of megabytes (the deck's
        // bundled TTF blobs are inlined as base64). Replicating the
        // same block in every slide multiplies the output by N — for a
        // 132-slide deck with 27 MB of fonts that's ~3.6 GB of pure
        // duplication, enough to blow past the wasm32 4 GB linear
        // memory ceiling and crash the conversion with `unreachable`.
        //
        // Inject the defs only into the first returned slide. Hosts
        // that render slides into a shared DOM scope (the
        // `pptx-viewer` Web Component, an SSR document, etc.) can
        // hoist that `<style>` once and every subsequent slide's text
        // will pick the same `@font-face` family up. Single-slide
        // conversions still get their defs because `slides[0]` is
        // always present.
        if let Some(first) = slides.first_mut() {
            first.svg = inject_after_svg_open(&first.svg, &embedded_defs);
        }
    }
    Ok(slides)
}

/// Extract `<p:embeddedFontLst>` faces from the archive, optionally
/// subsetting each face to the codepoints recorded in `used`.
///
/// When `used` is non-empty, each face's bytes are subsetted to only the
/// glyphs needed by slide runs (retaining `cmap` so browsers can resolve
/// the `@font-face`). When `used` is empty or the face isn't in the map,
/// the original bytes are kept unchanged.
fn build_embedded_face_list(
    bytes: &[u8],
    used: &UsedCodepoints,
    embed_format: EmbedFormat,
) -> Result<Vec<crate::embedded_fonts::EmbeddedFontFace>, ConvertError> {
    use crate::embedded_fonts::{resolve_embed_mime, subset_font_bytes};
    let Ok(mut archive) = PptxArchive::open(bytes.to_vec()) else {
        return Ok(Vec::new());
    };
    let presentation = parse_pptx(bytes.to_vec())?;
    let Some(fonts) = presentation.info.embedded_fonts.as_ref() else {
        return Ok(Vec::new());
    };
    let mut faces = extract_embedded_faces(&mut archive, fonts).unwrap_or_default();
    if faces.is_empty() {
        return Ok(Vec::new());
    }
    let mut cache = FontBytesCache::new();
    for face in &mut faces {
        // Subset to the codepoints seen for this typeface (pass-through when empty).
        // `ttf_bytes` is always TTF/OTF — never WOFF2 — so FontFace::from_bytes /
        // ttf_parser callers remain valid regardless of the chosen embed_format.
        let codepoints = used.get(&face.typeface);
        let ttf_bytes: Vec<u8> = if let Some(cp) = codepoints {
            subset_font_bytes(&face.bytes, cp).unwrap_or_else(|_| face.bytes.clone())
        } else {
            face.bytes.clone()
        };
        // Optionally compress to WOFF2 for the SVG @font-face payload only.
        // `bytes_for_svg` may be WOFF2; `face.bytes` stays TTF (Fix 5).
        #[allow(clippy::match_same_arms)]
        let svg_bytes = match embed_format {
            EmbedFormat::Woff2 | EmbedFormat::Auto => {
                #[cfg(feature = "woff2")]
                {
                    use crate::embedded_fonts::compress_font_to_woff2;
                    compress_font_to_woff2(&ttf_bytes).unwrap_or_else(|_| ttf_bytes.clone())
                }
                #[cfg(not(feature = "woff2"))]
                ttf_bytes.clone()
            }
            EmbedFormat::Ttf => ttf_bytes.clone(),
        };
        // Update mime to reflect bytes_for_svg format.
        face.mime = resolve_embed_mime(embed_format);
        // Dedup via xxHash3 cache keyed on bytes_for_svg — identical encoded
        // payloads share one Arc<String> (KDD-21, Fix 4).
        face.base64_cache = Some(cache.get_or_insert(&svg_bytes));
        // Collect aliases from the font's name table (Fix 7).
        face.aliases = slideglance_font::all_face_family_names(&ttf_bytes)
            .into_iter()
            .filter(|a| a != &face.typeface)
            .collect();
        face.bytes = ttf_bytes;
        face.bytes_for_svg = svg_bytes;
    }
    Ok(faces)
}

/// Insert `defs` immediately after the `<svg …>` opening tag of an
/// SVG document. Falls back to prepending when the document doesn't
/// start with `<svg `.
fn inject_after_svg_open(svg: &str, defs: &str) -> String {
    let Some(open_end) = svg.find('>') else {
        return format!("{defs}{svg}");
    };
    let mut out = String::with_capacity(svg.len() + defs.len());
    out.push_str(&svg[..=open_end]);
    out.push_str(defs);
    out.push_str(&svg[open_end + 1..]);
    out
}

/// Parse a PPTX byte stream and render every (or `options.slides`)
/// slide to PNG bytes.
///
/// Forces the renderer into path-mode by requiring `options.font_resolver`
/// to be `Some(_)`. Uses the caller-supplied `options.png_fonts` to
/// populate the rasterizer's fontdb.
///
/// # Errors
///
/// - [`ConvertError::FontResolverRequiredForPng`] if no font resolver
///   was supplied.
/// - [`ConvertError::Pptx`] if the archive can't be parsed.
/// - [`ConvertError::Renderer`] if the renderer rejects any slide.
/// - [`ConvertError::Png`] if rasterization of any slide fails.
pub fn convert_to_png(
    bytes: impl Into<Vec<u8>>,
    options: &ConvertOptions<'_>,
) -> Result<Vec<SlideImage>, ConvertError> {
    if options.fonts.resolver.is_none() {
        return Err(ConvertError::FontResolverRequiredForPng);
    }
    let svgs = convert_to_svg(bytes, options)?;
    // KDD-22: derive the rasterizer fontdb from `inline_fonts` so the
    // SVG `<defs>@font-face</defs>` advertisements and the bytes resvg
    // sees at rasterization time always agree. The legacy
    // `png_fonts` field has been removed; callers that previously
    // pre-built `FontData` values should populate `fonts.inline_fonts`
    // (an `AdditionalFont` list) instead.
    let png_fonts: Vec<FontData> = options
        .fonts
        .inline_fonts
        .iter()
        .map(|af| FontData::new(af.bytes.clone()))
        .collect();
    let png_options = PngOptions {
        width: options.width,
        height: options.height,
        fonts: png_fonts,
    };
    let mut out = Vec::with_capacity(svgs.len());
    for s in svgs {
        let png = svg_to_png(&s.svg, &png_options).map_err(|source| ConvertError::Png {
            slide_number: s.slide_number,
            source,
        })?;
        out.push(SlideImage {
            slide_number: s.slide_number,
            png: png.png,
            width: png.width,
            height: png.height,
            notes: s.notes,
            layout_name: s.layout_name,
            section_name: s.section_name,
        });
    }
    Ok(out)
}

/// Construct an [`OpentypeTextMeasurer`] from PPTX-extracted /
/// caller-supplied font byte buffers. Returns `None` when no faces
/// parse successfully — callers should fall back to
/// [`HeuristicTextMeasurer`] in that case.
///
/// Each `EmbeddedFontFace` is registered under both its declared
/// typeface and every additional family-name alias the font file's
/// `name` table exposes (id 1 / 4 / 16 / 21, all platforms /
/// languages). PPTX `<a:latin typeface="…"/>` strings vary by author
/// OS — Korean `PowerPoint` typically writes the localized id-1 family
/// (`프리젠테이션 7 Bold`), Windows en-US writes the English id-1
/// (`Freesentation 7 Bold`), and `<a:font script="Hang">` carries the
/// typographic family (`Freesentation`). Indexing every alias makes
/// the measurer pick up the same physical face regardless of which
/// Resolver that consults an embedded-font buffer first and falls back
/// to the caller-supplied resolver for anything not in the deck. Used
/// by [`convert_to_svg`] / [`convert_to_png`] so path-mode glyph
/// extraction reads the deck's own TTFs (matching what the SVG
/// `@font-face` defs already advertise) instead of a host substitute
/// that may differ in stem-thickness / advance widths by 1-2 px.
struct EmbeddedAwareResolver<'a> {
    embedded: BufferFontResolver,
    fallback: Option<&'a dyn FontResolver>,
}

impl FontResolver for EmbeddedAwareResolver<'_> {
    fn resolve(&self, name: &str) -> Option<Arc<FontFace>> {
        if let Some(face) = self.embedded.resolve(name) {
            return Some(face);
        }
        self.fallback.and_then(|r| r.resolve(name))
    }
}

/// Parse every embedded face into the buffer resolver under its
/// declared typeface AND each alias the font's `name` table exposes.
/// Bold faces are also registered under `"{family} Bold"` so the
/// path-mode `resolve_face` lookup chain matches them when a run has
/// `<a:rPr b="1">` even if the typeface attribute carries the bold
/// suffix already.
fn build_embedded_buffer_resolver(
    embedded_faces: &[crate::embedded_fonts::EmbeddedFontFace],
) -> BufferFontResolver {
    let mut buffer = BufferFontResolver::new();
    for face in embedded_faces {
        let Ok(parsed) = FontFace::from_bytes(face.bytes.clone(), 0) else {
            continue;
        };
        let arc = Arc::new(parsed);
        // Primary typeface name as declared in the PPTX.
        buffer.insert_arc(face.typeface.clone(), Arc::clone(&arc));
        // Aliases the font itself reports (e.g. `Freesentation 7 Bold`
        // ↔ `프리젠테이션 7 Bold`). Register every variant so deck-side
        // text using either spelling resolves to the same face.
        for alias in slideglance_font::all_face_family_names(&face.bytes) {
            buffer.insert_arc(alias, Arc::clone(&arc));
        }
        // path-mode `resolve_face` tries `"{family} Bold"` first when
        // the run requests bold. Even if the typeface name already
        // ends in "Bold", a deck might also reference the bare family
        // for a different typeface, so always register the suffixed
        // alias for bold faces too.
        if face.weight == "bold" {
            let suffixed = format!("{} Bold", face.typeface);
            if !suffixed.ends_with(" Bold Bold") {
                buffer.insert_arc(suffixed, Arc::clone(&arc));
            }
        }
    }
    buffer
}

/// Public re-export shim for [`PptxDocument`] — same body as the
/// private helper used internally by [`convert_to_svg`].
pub(crate) fn build_auto_opentype_measurer_pub(
    embedded_faces: &[crate::embedded_fonts::EmbeddedFontFace],
    measurement_fonts: &[Vec<u8>],
) -> Option<OpentypeTextMeasurer> {
    build_auto_opentype_measurer(embedded_faces, measurement_fonts)
}

fn build_auto_opentype_measurer(
    embedded_faces: &[crate::embedded_fonts::EmbeddedFontFace],
    measurement_fonts: &[Vec<u8>],
) -> Option<OpentypeTextMeasurer> {
    use std::collections::BTreeMap;
    let mut map: BTreeMap<String, FontFace> = BTreeMap::new();
    let mut default_face: Option<FontFace> = None;
    // 1. PPTX-declared embedded faces — register under the declared
    // typeface AND every alias the font's `name` table exposes.
    for face in embedded_faces {
        let Ok(parsed) = FontFace::from_bytes(face.bytes.clone(), 0) else {
            continue;
        };
        if !map.contains_key(&face.typeface) {
            map.insert(face.typeface.clone(), parsed.clone());
        }
        for alias in slideglance_font::all_face_family_names(&face.bytes) {
            map.entry(alias).or_insert_with(|| parsed.clone());
        }
        if default_face.is_none() {
            default_face = Some(parsed);
        }
    }
    // 2. Caller-supplied measurement-only buffers (e.g. host system
    // fonts injected by the CLI). Walk every face in TTC bundles.
    //
    // Use each face's *own* `all_family_names()` rather than the file-
    // wide `all_face_family_names(bytes)` union — the union pulls every
    // face's aliases (Light / Regular / Bold / …) into the alias list
    // returned for face 0, and `map.entry(...).or_insert_with(...)`
    // would then pin ALL aliases to face 0. A request for plain
    // "Apple SD Gothic Neo" would then return whichever weight
    // happens to be index 0 of the TTC (often Heavy on Apple's macOS
    // bundles), not the Regular weight the WebView paints with by
    // default — that width mismatch is what produced the "browser
    // fits the line, app wraps it" symptom on Korean decks.
    //
    // Within a TTC, sort faces by `|weight − 400|` ascending so the
    // bare typographic-family alias (e.g. "Apple SD Gothic Neo") is
    // claimed by the Regular face — matching CSS's default weight 400
    // resolution. Subfamily-qualified aliases ("…Light", "…Bold", …)
    // still pin to their own faces because they're unique strings.
    for bytes in measurement_fonts {
        let face_count = ttf_parser::fonts_in_collection(bytes).unwrap_or(1);
        let mut faces: Vec<FontFace> = (0..face_count)
            .filter_map(|i| FontFace::from_bytes(bytes.clone(), i).ok())
            .collect();
        faces.sort_by_key(|f| (i32::from(f.weight()) - 400).abs());
        for parsed in faces {
            for alias in parsed.all_family_names() {
                map.entry(alias).or_insert_with(|| parsed.clone());
            }
            if default_face.is_none() {
                default_face = Some(parsed);
            }
        }
    }
    if map.is_empty() {
        return None;
    }
    Some(OpentypeTextMeasurer::from_fonts(
        map,
        default_face,
        FontMapping::new(),
        CjkPlatform::current(),
    ))
}

fn render_all_slides(
    presentation: &Presentation,
    options: &ConvertOptions<'_>,
    resolver_override: Option<&dyn FontResolver>,
    measurer_override: Option<&dyn TextMeasurer>,
) -> Result<Vec<SlideSvg>, ConvertError> {
    let script_fonts = ScriptFontContext::new(
        presentation.theme.font_scheme.major_script_fonts.clone(),
        presentation.theme.font_scheme.minor_script_fonts.clone(),
    );

    // Section lookup: numeric slide id -> section name.
    // PresentationInfo holds sections + slide_id_values aligned with the
    // slide_r_ids ordering; we build a map once and consult it per
    // slide. `sections` is None when the deck has no <p14:sectionLst>.
    let section_by_slide_id: HashMap<i64, String> = presentation
        .info
        .sections
        .iter()
        .flat_map(|secs| secs.iter())
        .flat_map(|sec| {
            sec.slide_ids
                .iter()
                .map(move |sid| (*sid, sec.name.clone()))
        })
        .collect();

    let total_slides = u32::try_from(presentation.slides.len()).ok();
    // Override slot wins (auto-built measurer / embedded-aware resolver
    // were assembled in `convert_to_svg`); fall back to the caller-set
    // values for either field, then to the heuristic measurer / no
    // resolver as final defaults.
    let measurer: &dyn TextMeasurer = measurer_override
        .or(options.measurer)
        .unwrap_or(&HeuristicTextMeasurer);
    let resolver: Option<&dyn FontResolver> = resolver_override.or_else(|| {
        options
            .fonts
            .resolver
            .as_deref()
            .map(|r| r as &dyn FontResolver)
    });
    // KDD-11: `PathMode` whenever a resolver is active; `TextMode`
    // otherwise. Same classification as `TextEngineBuilder::build`.
    let render_mode = if resolver.is_some() {
        RenderMode::PathMode
    } else {
        RenderMode::TextMode
    };

    let slides_filter: Option<&[u32]> = options.slides.as_deref();

    let mut out: Vec<SlideSvg> = Vec::new();
    for (idx, rendered) in presentation.slides.iter().enumerate() {
        if let Some(filter) = slides_filter {
            if !filter.contains(&rendered.slide.slide_number) {
                continue;
            }
        }

        // master / layout / slide composition mirrors TS
        // mergeElements — drop placeholder shapes from master and
        // layout (those are templates), keep their decorative shapes,
        // then layer slide elements on top.
        let effective_master = if rendered.slide.show_master_sp && rendered.layout_show_master_sp {
            rendered.master_elements.as_slice()
        } else {
            &[]
        };
        let merged = merge_elements(
            effective_master,
            &rendered.layout_elements,
            &rendered.slide.elements,
        );

        // Build a fresh Slide with merged elements but everything else
        // preserved. Cheaper than cloning the entire RenderedSlide.
        let mut slide_for_render = rendered.slide.clone();
        slide_for_render.elements = merged;

        let svg = render_slide_to_svg(
            &slide_for_render,
            &presentation.info.slide_size,
            total_slides,
            &script_fonts,
            measurer,
            &options.mapping,
            options.cjk_platform,
            resolver,
            options.timestamp,
        )
        .map_err(|source| ConvertError::Renderer {
            slide_number: rendered.slide.slide_number,
            source,
        })?;

        // Section name lookup: align 1-based slide index with
        // slide_id_values entries.
        let section_name = presentation
            .info
            .slide_id_values
            .get(idx)
            .and_then(|sid| section_by_slide_id.get(sid).cloned());

        out.push(SlideSvg {
            slide_number: rendered.slide.slide_number,
            svg,
            notes: rendered.slide.notes.clone(),
            layout_name: rendered.slide.layout_name.clone(),
            section_name,
            render_mode,
            // R-C2: D3-T13 will set this to `true` when the renderer
            // detects FSP step 4 (host system fallback) firing during
            // glyph extraction. D0 always reports `false` because the
            // renderer cannot yet signal that condition back up.
            fallback_used: false,
        });
    }
    Ok(out)
}

/// Concat master / layout / slide elements while filtering placeholders
/// out of master and layout. Mirrors TS `mergeElements`
/// (): placeholder shapes on master and layout
/// describe geometry only — their text is never visible content. Only
/// non-placeholder shapes (decorative elements, logos, …) survive.
/// Walk all slide runs across every slide in the presentation and collect
/// the Unicode codepoints used per typeface.
///
/// The Latin typeface (`font_family`), EA typeface (`font_family_ea`), and
/// CS typeface (`font_family_cs`) are each credited with the full run text
/// so that all three font buckets are subsetted correctly.  An empty or
/// absent typeface name is skipped.
fn collect_used_codepoints(presentation: &slideglance_model::Presentation) -> UsedCodepoints {
    let mut used = UsedCodepoints::default();
    for rendered in &presentation.slides {
        collect_codepoints_from_elements(&rendered.slide.elements, &mut used);
        collect_codepoints_from_elements(&rendered.layout_elements, &mut used);
        collect_codepoints_from_elements(&rendered.master_elements, &mut used);
    }
    used
}

fn collect_codepoints_from_elements(elements: &[SlideElement], used: &mut UsedCodepoints) {
    for el in elements {
        match el {
            SlideElement::Shape(s) => {
                if let Some(tb) = &s.text_body {
                    collect_codepoints_from_text_body(tb, used);
                }
            }
            SlideElement::Group(g) => {
                collect_codepoints_from_elements(&g.children, used);
            }
            SlideElement::Table(t) => {
                for row in &t.table.rows {
                    for cell in &row.cells {
                        if let Some(tb) = &cell.text_body {
                            collect_codepoints_from_text_body(tb, used);
                        }
                    }
                }
            }
            SlideElement::Chart(_) | SlideElement::Connector(_) | SlideElement::Image(_) => {}
        }
    }
}

fn collect_codepoints_from_text_body(tb: &slideglance_model::TextBody, used: &mut UsedCodepoints) {
    for para in &tb.paragraphs {
        for run in &para.runs {
            let text = &run.text;
            if text.is_empty() {
                continue;
            }
            let props = &run.properties;
            // Credit each declared typeface with the full run text.
            if let Some(family) = &props.font_family {
                used.add_text_for(family, text);
            }
            if let Some(family) = &props.font_family_ea {
                used.add_text_for(family, text);
            }
            if let Some(family) = &props.font_family_cs {
                used.add_text_for(family, text);
            }
            // Sym font (Wingdings / Symbol): PUA codepoints U+F000-F0FF.
            // Must be credited so subsetting retains them (Fix 6).
            if let Some(family) = &props.font_family_sym {
                used.add_text_for(family, text);
            }
        }
    }
}

fn merge_elements(
    master: &[SlideElement],
    layout: &[SlideElement],
    slide: &[SlideElement],
) -> Vec<SlideElement> {
    let filter_placeholders = |elements: &[SlideElement]| -> Vec<SlideElement> {
        elements
            .iter()
            .filter(|el| match el {
                SlideElement::Shape(s) => s.placeholder_type.is_none(),
                _ => true,
            })
            .cloned()
            .collect()
    };
    let mut out = Vec::with_capacity(master.len() + layout.len() + slide.len());
    out.extend(filter_placeholders(master));
    out.extend(filter_placeholders(layout));
    out.extend_from_slice(slide);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Read one of the shared fixture PPTX files into bytes for
    /// integration tests.
    fn fixture_bytes(name: &str) -> Vec<u8> {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path = Path::new(manifest_dir)
            .join("..")
            .join("..")
            .join("testing/fixtures")
            .join(name);
        std::fs::read(&path).unwrap_or_else(|_| panic!("fixture exists at {path:?}"))
    }

    #[test]
    fn font_config_default_keeps_legacy_behavior() {
        // IC-5: `embed_deck_fonts` defaults to `true` to match the old
        // `ConvertOptions::embed_fonts` default.
        let cfg = FontConfig::default();
        assert!(cfg.resolver.is_none());
        assert!(cfg.embed_deck_fonts);
        assert!(cfg.inline_fonts.is_empty());
        assert!(cfg.measurement_only_fonts.is_empty());
        assert_eq!(cfg.embed_format, EmbedFormat::Auto);
    }

    #[test]
    fn convert_options_carries_fonts_field() {
        let opts = ConvertOptions::default();
        // Compile-time access to the new consolidated field.
        let _ = &opts.fonts;
        assert!(opts.fonts.embed_deck_fonts);
    }

    #[test]
    fn slide_svg_carries_render_mode_and_fallback_used_fields() {
        // Compile-time + default-value test for the new fields added by
        // T7 / R-C2. `RenderMode::TextMode` is the default; D0 always
        // reports `fallback_used = false`.
        let svg = SlideSvg {
            slide_number: 1,
            svg: String::new(),
            notes: None,
            layout_name: None,
            section_name: None,
            render_mode: RenderMode::TextMode,
            fallback_used: false,
        };
        assert_eq!(svg.render_mode, RenderMode::TextMode);
        assert!(!svg.fallback_used);
    }

    #[test]
    #[ignore = "needs testing/fixtures/* — drop fixtures into testing/fixtures/ to enable"]
    fn convert_to_svg_text_mode_sets_render_mode_text_mode() {
        // Default options (no font resolver) must yield TextMode and
        // fallback_used = false on every slide.
        let bytes = fixture_bytes("sample.pptx");
        let out = convert_to_svg(bytes, &ConvertOptions::default()).expect("converted");
        for s in &out {
            assert_eq!(s.render_mode, RenderMode::TextMode);
            assert!(!s.fallback_used);
        }
    }

    #[test]
    #[ignore = "needs testing/fixtures/* — drop fixtures into testing/fixtures/ to enable"]
    fn convert_sample_pptx_to_svg_returns_at_least_one_slide() {
        let bytes = fixture_bytes("sample.pptx");
        let out = convert_to_svg(bytes, &ConvertOptions::default()).expect("converted");
        assert!(
            !out.is_empty(),
            "sample.pptx should yield at least one slide"
        );
        for s in &out {
            assert!(
                s.svg.starts_with("<svg "),
                "slide {} svg malformed",
                s.slide_number
            );
            assert!(
                s.svg.ends_with("</svg>"),
                "slide {} svg unterminated",
                s.slide_number
            );
        }
    }

    #[test]
    #[ignore = "needs testing/fixtures/* — drop fixtures into testing/fixtures/ to enable"]
    fn convert_to_svg_filters_slides_by_number() {
        let bytes = fixture_bytes("sample.pptx");
        let opts = ConvertOptions {
            slides: Some(vec![1]),
            ..ConvertOptions::default()
        };
        let out = convert_to_svg(bytes, &opts).expect("converted");
        assert!(out.iter().all(|s| s.slide_number == 1));
    }

    #[test]
    #[ignore = "needs testing/fixtures/* — drop fixtures into testing/fixtures/ to enable"]
    fn convert_to_svg_filters_to_no_match_returns_empty() {
        let bytes = fixture_bytes("sample.pptx");
        let opts = ConvertOptions {
            slides: Some(vec![9999]),
            ..ConvertOptions::default()
        };
        let out = convert_to_svg(bytes, &opts).expect("converted");
        assert!(out.is_empty());
    }

    #[test]
    #[ignore = "needs testing/fixtures/* — drop fixtures into testing/fixtures/ to enable"]
    fn convert_to_png_without_font_resolver_errors() {
        let bytes = fixture_bytes("sample.pptx");
        let err = convert_to_png(bytes, &ConvertOptions::default())
            .expect_err("must require font_resolver");
        assert!(matches!(err, ConvertError::FontResolverRequiredForPng));
    }

    #[test]
    fn merge_elements_drops_master_and_layout_placeholders() {
        use slideglance_model::{Geometry, PresetGeometry, ShapeElement, Transform};
        use slideglance_utils::Emu;
        use std::collections::BTreeMap;

        fn shape(placeholder: Option<&str>) -> SlideElement {
            SlideElement::Shape(ShapeElement {
                sp_id: None,
                transform: Transform {
                    offset_x: Emu::new(0),
                    offset_y: Emu::new(0),
                    extent_width: Emu::new(914_400),
                    extent_height: Emu::new(914_400),
                    ..Transform::default()
                },
                geometry: Geometry::Preset(PresetGeometry {
                    preset: "rect".to_string(),
                    adjust_values: BTreeMap::new(),
                }),
                fill: None,
                outline: None,
                text_body: None,
                effects: None,
                placeholder_type: placeholder.map(str::to_string),
                placeholder_idx: None,
                alt_text: None,
                object_name: None,
                hidden: false,
                hyperlink: None,
            })
        }

        let master = vec![shape(Some("title")), shape(None)]; // logo decoration kept
        let layout = vec![shape(Some("body")), shape(None)];
        let slide = vec![shape(Some("title")), shape(None)]; // slide placeholders kept

        let merged = merge_elements(&master, &layout, &slide);
        // Filtered master: 1 (decoration only)
        // Filtered layout: 1 (decoration only)
        // Slide: 2 (both kept)
        assert_eq!(merged.len(), 4);
    }
}
