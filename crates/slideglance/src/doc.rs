//! Stateful document API for incremental, on-demand slide rendering.
//!
//! The classic [`crate::convert_to_svg`] pipeline parses the deck and
//! renders every slide in one shot. For decks with hundreds of slides
//! and tens of MB of inline media that path serializes hundreds of MB
//! across the WASM ↔ JS boundary in a single call and blocks the host
//! for several seconds.
//!
//! [`PptxDocument`] decouples parse from render:
//!
//! 1. [`PptxDocument::parse`] runs the archive open + theme + slide
//!    walk once and caches the resulting [`slideglance_model::Presentation`],
//!    embedded font face list, the deck-wide `@font-face` defs string,
//!    and an auto-built [`slideglance_font::OpentypeTextMeasurer`].
//! 2. [`PptxDocument::render_slide`] renders one 1-based slide on
//!    demand, reusing the cached state. Repeated calls with different
//!    slide numbers do not re-parse the archive.
//! 3. With [`SlideRenderOptions::external_media`] enabled, base64
//!    `data:` image URIs in the rendered SVG are post-processed into
//!    `pptx-media://{hash}` references and the raw bytes are returned
//!    in [`RenderedSlide::media`]. Hosts can build `Blob` URLs once
//!    per unique media item — repeated images across slides dedup
//!    automatically because the hash is content-derived.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use slideglance_font::{
    CjkPlatform, FontMapping, FontResolver, HeuristicTextMeasurer, OpentypeTextMeasurer,
    ScriptFontContext, TextMeasurer,
};
use slideglance_model::{Presentation, SlideElement};
use slideglance_parser::PptxArchive;
use slideglance_renderer::{render_slide_to_svg, Timestamp};

use crate::convert::ConvertError;
use crate::embedded_fonts::{
    additional_to_faces, render_embedded_font_defs, AdditionalFont, EmbeddedFontFace,
};
use crate::font_usage::{build_typeface_usage, TypefaceUsage};
use crate::{extract_referenced_font_families, parse_pptx, PptxError};

/// Per-slide render options. Mirrors a subset of [`crate::ConvertOptions`]
/// — the parts that legitimately vary between sibling render calls on the
/// same document. Deck-wide settings (embedded fonts, additional fonts,
/// measurement fonts) live on the [`PptxDocument`] itself.
pub struct SlideRenderOptions<'a> {
    /// PPTX font family → OSS substitute mapping.
    pub mapping: FontMapping,
    /// CJK platform for font-family list construction.
    pub cjk_platform: Option<CjkPlatform>,
    /// Path-mode font resolver. `None` keeps the renderer in text-mode.
    pub font_resolver: Option<&'a dyn FontResolver>,
    /// Override the auto-built measurer. `None` defers to whatever the
    /// document built at parse time (Opentype if any fonts were
    /// available, else [`HeuristicTextMeasurer`]).
    pub measurer: Option<&'a dyn TextMeasurer>,
    /// Wall-clock timestamp for `datetime{N}` field substitution.
    pub timestamp: Option<Timestamp>,
    /// When `true`, replace base64 `data:` image URIs in the SVG with
    /// `pptx-media://{hash}` references and return the raw bytes via
    /// [`RenderedSlide::media`]. Lets hosts build [`Blob`] URLs and
    /// keep transmitted SVG strings small.
    pub external_media: bool,
    /// When `true` (default), the deck-wide `@font-face` `<defs>` block
    /// is injected into this slide's SVG. Hosts that mount the font
    /// defs once at the document scope (e.g. shadow-root `<style>`)
    /// should set this to `false` and use [`PptxDocument::font_defs`].
    pub include_font_defs: bool,
}

impl Default for SlideRenderOptions<'_> {
    fn default() -> Self {
        Self {
            mapping: FontMapping::new(),
            cjk_platform: None,
            font_resolver: None,
            measurer: None,
            timestamp: None,
            external_media: false,
            include_font_defs: true,
        }
    }
}

/// One unique media blob referenced by a slide rendered with
/// [`SlideRenderOptions::external_media`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaBlob {
    /// MIME type (`image/png`, `image/jpeg`, …).
    pub mime: String,
    /// Raw decoded bytes — host wraps these in a `Blob` URL.
    pub bytes: Vec<u8>,
}

/// One slide's render result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedSlide {
    /// 1-based slide number.
    pub slide_number: u32,
    /// Self-contained SVG document.
    pub svg: String,
    /// Speaker notes from `notesSlide{N}.xml`, if present.
    pub notes: Option<String>,
    /// Layout name (`<p:cSld @name>` on the slide layout), if present.
    pub layout_name: Option<String>,
    /// Section name from `<p14:section>`, if the slide belongs to a
    /// section.
    pub section_name: Option<String>,
    /// Media blobs referenced by `pptx-media://{hash}` URIs in `svg`.
    /// Empty when [`SlideRenderOptions::external_media`] was `false`.
    /// Keys are stable hex hashes of the original base64 payload —
    /// identical content across slides yields the same key, so hosts
    /// can dedup blobs.
    pub media: HashMap<String, MediaBlob>,
}

/// Stateful PPTX document for incremental rendering.
pub struct PptxDocument {
    presentation: Presentation,
    /// Pre-rendered `<defs>…@font-face…</defs>` block for the deck.
    /// Empty when the deck has no embedded fonts and no caller-supplied
    /// additional fonts.
    embedded_defs: String,
    /// Auto-built measurer. Populated whenever any face was available
    /// at parse time. Falls through to [`HeuristicTextMeasurer`] in
    /// [`render_slide`] when this is `None`.
    auto_measurer: Option<OpentypeTextMeasurer>,
    /// Slide-id → section name lookup. Built once at parse time.
    section_by_slide_id: HashMap<i64, String>,
    /// Theme script-font lookup. Built once so per-slide renders don't
    /// re-clone the major / minor script-font `BTreeMap` every call.
    script_fonts: ScriptFontContext,
    /// `slide_number → index` cache for O(1) slide lookup. Slides are
    /// usually 1..=N consecutive, but the parser doesn't guarantee
    /// it, so we build the map explicitly.
    slide_index: HashMap<u32, usize>,
    /// Distinct typeface names referenced anywhere in the deck's XML
    /// (`typeface="…"` attrs on `<a:latin>`, `<a:ea>`, `<a:cs>`,
    /// `<a:font script="…">`). Populated once at parse time and reused
    /// by [`PptxDocument::font_usage`].
    referenced_typefaces: Vec<String>,
    /// `<p:embeddedFont>` payloads that are `MicroType` Express
    /// compressed (Agfa Monotype LZ77 + adaptive Huffman). The Rust
    /// pipeline can't decompress these, so they're held here as raw
    /// bytes for a JS-side decompressor to consume — see
    /// [`PptxDocument::compressed_embedded_fonts`].
    compressed_fonts: Vec<crate::embedded_fonts::CompressedEmbeddedFont>,
}

impl PptxDocument {
    /// Parse a PPTX byte stream and prepare it for incremental
    /// rendering.
    ///
    /// `additional_fonts` are inlined into the deck-wide `<defs>` and
    /// fed into the auto measurer alongside any `<p:embeddedFontLst>`
    /// faces extracted from the archive. Pass an empty `Vec` to keep
    /// only the deck's own embedded faces.
    ///
    /// `measurement_fonts` are extra TTF / OTF / TTC byte buffers used
    /// **only** for wrap measurement — they are not inlined and not
    /// surfaced as media blobs. Use this to register host system fonts
    /// without bloating output.
    ///
    /// # Errors
    ///
    /// - [`ConvertError::Pptx`] if the archive can't be parsed.
    pub fn parse(
        bytes: impl Into<Vec<u8>>,
        additional_fonts: &[AdditionalFont],
        measurement_fonts: &[Vec<u8>],
        embed_fonts: bool,
    ) -> Result<Self, ConvertError> {
        let buffer: Vec<u8> = bytes.into();

        let mut all_faces: Vec<EmbeddedFontFace> = Vec::new();
        let mut compressed_fonts: Vec<crate::embedded_fonts::CompressedEmbeddedFont> = Vec::new();
        if embed_fonts {
            if let Ok((ready, compressed)) = extract_embedded_fonts_split_from_buffer(&buffer) {
                all_faces.extend(ready);
                compressed_fonts = compressed;
            }
        }
        if !additional_fonts.is_empty() {
            all_faces.extend(additional_to_faces(additional_fonts));
        }
        let embedded_defs = render_embedded_font_defs(&all_faces);

        // Collect deck-referenced typefaces from the raw bytes — same scan
        // `extract_referenced_font_families` performs for system-font
        // filtering, reused here so `font_usage` can expose the list
        // without re-opening the archive.
        let referenced_typefaces = extract_referenced_font_families(&buffer);

        let presentation = parse_pptx(buffer)?;

        let auto_measurer =
            crate::convert::build_auto_opentype_measurer_pub(&all_faces, measurement_fonts);

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

        let script_fonts = ScriptFontContext::new(
            presentation.theme.font_scheme.major_script_fonts.clone(),
            presentation.theme.font_scheme.minor_script_fonts.clone(),
        );

        let slide_index: HashMap<u32, usize> = presentation
            .slides
            .iter()
            .enumerate()
            .map(|(i, r)| (r.slide.slide_number, i))
            .collect();

        Ok(Self {
            presentation,
            embedded_defs,
            auto_measurer,
            section_by_slide_id,
            script_fonts,
            slide_index,
            referenced_typefaces,
            compressed_fonts,
        })
    }

    /// Embedded `<p:embeddedFont>` payloads that are `MicroType` Express
    /// compressed and therefore could not be turned into TTF/OTF
    /// `FontFaces` by the Rust pipeline. Each entry carries the family
    /// name plus the raw post-EOT-header payload bytes (XOR
    /// de-obfuscation already applied).
    ///
    /// Hosts that ship a JS-side MTX decompressor (e.g. the
    /// chrome-extension viewer's `mtx-decompressor` integration) read
    /// this list, decompress each payload, and register the resulting
    /// TTF binary as a `FontFace`. Hosts without that capability can
    /// safely ignore this list — the deck still renders, just with
    /// the family-name fallback chain instead of the authored face.
    #[must_use]
    pub fn compressed_embedded_fonts(&self) -> &[crate::embedded_fonts::CompressedEmbeddedFont] {
        &self.compressed_fonts
    }

    /// Distinct typeface names referenced by the deck. Populated at
    /// parse time from every `typeface="…"` attribute in the archive's
    /// XML parts. The list is sorted (`BTreeSet` insertion order in
    /// [`extract_referenced_font_families`]) and deduplicated.
    #[must_use]
    pub fn referenced_typefaces(&self) -> &[String] {
        &self.referenced_typefaces
    }

    /// Build a per-typeface usage report describing how each
    /// deck-referenced font flows through the SVG `font-family` chain
    /// and the optional path-mode `resolver`.
    ///
    /// `mapping` and `cjk_platform` should match the values passed to
    /// [`PptxDocument::render_slide`] so the chain entries reflect what
    /// the renderer actually emits. `resolver` may be `None` for
    /// text-mode hosts (browser viewer) — the report's `resolved_family`
    /// will be `None` for every entry, and the host should probe each
    /// chain entry via the platform's font-availability API
    /// (`document.fonts.check()`) to identify the actually-rendered
    /// font.
    #[must_use]
    pub fn font_usage(
        &self,
        mapping: &FontMapping,
        cjk_platform: CjkPlatform,
        resolver: Option<&dyn FontResolver>,
    ) -> Vec<TypefaceUsage> {
        build_typeface_usage(
            &self.referenced_typefaces,
            mapping,
            cjk_platform,
            &self.script_fonts,
            resolver,
        )
    }

    /// Number of slides in the deck.
    #[must_use]
    pub fn slide_count(&self) -> u32 {
        u32::try_from(self.presentation.slides.len()).unwrap_or(u32::MAX)
    }

    /// Deck-wide `@font-face` `<defs>` block. Empty when the deck has
    /// no embedded fonts and no caller-supplied additional fonts.
    /// Hosts can mount this once at document scope and pass
    /// [`SlideRenderOptions::include_font_defs`] = `false` to keep the
    /// per-slide SVG small.
    #[must_use]
    pub fn font_defs(&self) -> &str {
        &self.embedded_defs
    }

    /// Layout name (`<p:cSld @name>` on the slide layout) for the given
    /// 1-based slide. Returns `None` when the slide does not exist or
    /// when the layout omitted `@name`.
    ///
    /// Reads cached parse output — does not re-render the slide.
    #[must_use]
    pub fn slide_layout_name(&self, slide_number: u32) -> Option<&str> {
        let idx = *self.slide_index.get(&slide_number)?;
        self.presentation
            .slides
            .get(idx)?
            .slide
            .layout_name
            .as_deref()
    }

    /// Section name (`<p14:section @name>`) for the slide whose 1-based
    /// number is `slide_number`. Returns `None` when the slide is not in
    /// any section, or when the slide does not exist.
    ///
    /// Reads cached parse output — does not re-render the slide.
    #[must_use]
    pub fn slide_section_name(&self, slide_number: u32) -> Option<&str> {
        let idx = *self.slide_index.get(&slide_number)?;
        let sid = self.presentation.info.slide_id_values.get(idx)?;
        self.section_by_slide_id.get(sid).map(String::as_str)
    }

    /// Speaker notes for the given 1-based slide. Returns `None` when
    /// the slide does not exist or carries no notes.
    ///
    /// Reads cached parse output — does not re-render the slide.
    #[must_use]
    pub fn slide_notes(&self, slide_number: u32) -> Option<&str> {
        let idx = *self.slide_index.get(&slide_number)?;
        self.presentation.slides.get(idx)?.slide.notes.as_deref()
    }

    /// Section outline as `(name, first_slide_number)` pairs in the
    /// order they appear in `<p14:sectionLst>`. `first_slide_number`
    /// is 1-based and matches the slide's [`Self::slide_count`]
    /// numbering. Sections with zero slides are skipped, mirroring the
    /// host-side outline UI which can't navigate to an empty section.
    ///
    /// Returns an empty `Vec` when the deck has no sections. Owns the
    /// returned strings so callers do not see the internal
    /// `slideglance_model::PresentationSection` type.
    #[must_use]
    pub fn sections(&self) -> Vec<(String, u32)> {
        let Some(sections) = self.presentation.info.sections.as_ref() else {
            return Vec::new();
        };
        // Build an inverse map: slide_id -> 1-based slide_number, so we
        // can recover the position of `section.slide_ids[0]`.
        let mut id_to_number: HashMap<i64, u32> = HashMap::new();
        for (idx, sid) in self.presentation.info.slide_id_values.iter().enumerate() {
            if let Some(rendered) = self.presentation.slides.get(idx) {
                id_to_number.insert(*sid, rendered.slide.slide_number);
            }
        }
        sections
            .iter()
            .filter_map(|sec| {
                let first_id = sec.slide_ids.first()?;
                let first = *id_to_number.get(first_id)?;
                Some((sec.name.clone(), first))
            })
            .collect()
    }

    /// Render one 1-based slide.
    ///
    /// Returns [`None`] when `slide_number` is out of range.
    ///
    /// # Errors
    ///
    /// - [`ConvertError::Renderer`] if the renderer rejects the slide.
    pub fn render_slide(
        &self,
        slide_number: u32,
        options: &SlideRenderOptions<'_>,
    ) -> Result<Option<RenderedSlide>, ConvertError> {
        let Some(&idx) = self.slide_index.get(&slide_number) else {
            return Ok(None);
        };
        let rendered = &self.presentation.slides[idx];

        let total_slides = u32::try_from(self.presentation.slides.len()).ok();
        let measurer: &dyn TextMeasurer = if let Some(m) = options.measurer {
            m
        } else if let Some(ref m) = self.auto_measurer {
            m as &dyn TextMeasurer
        } else {
            &HeuristicTextMeasurer
        };

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
        let mut slide_for_render = rendered.slide.clone();
        slide_for_render.elements = merged;

        let cjk_platform = options.cjk_platform.unwrap_or_else(CjkPlatform::current);
        let svg = render_slide_to_svg(
            &slide_for_render,
            &self.presentation.info.slide_size,
            total_slides,
            &self.script_fonts,
            measurer,
            &options.mapping,
            cjk_platform,
            options.font_resolver,
            options.timestamp,
        )
        .map_err(|source| ConvertError::Renderer {
            slide_number,
            source,
        })?;

        let svg = if options.include_font_defs && !self.embedded_defs.is_empty() {
            inject_after_svg_open(&svg, &self.embedded_defs)
        } else {
            svg
        };

        let (svg, media) = if options.external_media {
            externalize_media(&svg)
        } else {
            (svg, HashMap::new())
        };

        let section_name = self
            .presentation
            .info
            .slide_id_values
            .get(idx)
            .and_then(|sid| self.section_by_slide_id.get(sid).cloned());

        Ok(Some(RenderedSlide {
            slide_number: rendered.slide.slide_number,
            svg,
            notes: rendered.slide.notes.clone(),
            layout_name: rendered.slide.layout_name.clone(),
            section_name,
            media,
        }))
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

/// Walk `<p:embeddedFontLst>` and split into ready-to-use TTF/OTF faces
/// (uncompressed EOT) plus MTX-compressed payloads. Mirrors
/// [`extract_embedded_face_list`] but keeps the compressed bucket so
/// the JS side can decompress on-demand.
fn extract_embedded_fonts_split_from_buffer(
    bytes: &[u8],
) -> Result<
    (
        Vec<EmbeddedFontFace>,
        Vec<crate::embedded_fonts::CompressedEmbeddedFont>,
    ),
    PptxError,
> {
    let Ok(mut archive) = PptxArchive::open(bytes.to_vec()) else {
        return Ok((Vec::new(), Vec::new()));
    };
    let presentation = parse_pptx(bytes.to_vec())?;
    let Some(fonts) = presentation.info.embedded_fonts.as_ref() else {
        return Ok((Vec::new(), Vec::new()));
    };
    Ok(
        crate::embedded_fonts::extract_embedded_fonts_split(&mut archive, fonts)
            .unwrap_or_default(),
    )
}

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

/// Walk the SVG once, replacing every `data:image/X;base64,YYY` URI
/// with `pptx-media://{hash}`. Returns the rewritten SVG plus a map of
/// hash → [`MediaBlob`] for every unique payload.
///
/// The hash is a hex-formatted [`std::collections::hash_map::DefaultHasher`]
/// of the base64 string. Within a single render run the hash function is
/// deterministic, so identical media across slides resolve to the same
/// key and hosts can dedup `Blob` URLs.
fn externalize_media(svg: &str) -> (String, HashMap<String, MediaBlob>) {
    // Format constraints we rely on:
    // - URIs always start with `data:image/`
    // - The MIME continues until the first `;`
    // - `;base64,` separator is fixed
    // - Payload runs until the next `"` (we only ever emit URIs inside
    // double-quoted attributes — `href="…"` / `xlink:href="…"`).
    const PREFIX: &str = "data:image/";
    const B64_SEP: &str = ";base64,";

    if !svg.contains(PREFIX) {
        return (svg.to_owned(), HashMap::new());
    }

    let mut out = String::with_capacity(svg.len());
    let mut media: HashMap<String, MediaBlob> = HashMap::new();
    let mut seen_hashes: HashSet<u64> = HashSet::new();
    let mut cursor = 0usize;
    let bytes = svg.as_bytes();

    while let Some(rel) = svg[cursor..].find(PREFIX) {
        let start = cursor + rel;
        out.push_str(&svg[cursor..start]);

        // Find the `;base64,` separator after the MIME segment.
        let Some(sep_rel) = svg[start..].find(B64_SEP) else {
            // Malformed — emit the rest verbatim and stop.
            out.push_str(&svg[start..]);
            cursor = svg.len();
            break;
        };
        let mime_end = start + sep_rel;
        let mime = &svg[start + "data:".len()..mime_end];
        let payload_start = mime_end + B64_SEP.len();

        // Payload runs until the next `"` — every URI in our renderer is
        // inside a double-quoted attribute. If the SVG ever switches to
        // single-quoted attributes the renderer would have to update too.
        let mut payload_end = payload_start;
        while payload_end < bytes.len() && bytes[payload_end] != b'"' {
            payload_end += 1;
        }
        if payload_end == bytes.len() {
            out.push_str(&svg[start..]);
            cursor = svg.len();
            break;
        }
        let payload = &svg[payload_start..payload_end];

        let hash = hash_str(payload);
        let key = format!("{hash:016x}");
        if seen_hashes.insert(hash) {
            // Only decode once per unique payload.
            let decoded = BASE64_STANDARD
                .decode(payload.as_bytes())
                .unwrap_or_default();
            media.insert(
                key.clone(),
                MediaBlob {
                    mime: mime.to_owned(),
                    bytes: decoded,
                },
            );
        }
        out.push_str("pptx-media://");
        out.push_str(&key);
        cursor = payload_end;
    }
    if cursor < svg.len() {
        out.push_str(&svg[cursor..]);
    }
    (out, media)
}

fn hash_str(s: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn externalize_replaces_data_uri_with_pptx_media() {
        let svg = r#"<svg><image href="data:image/png;base64,AAAA" width="10"/></svg>"#;
        let (out, media) = externalize_media(svg);
        assert!(out.starts_with("<svg><image href=\"pptx-media://"));
        assert!(out.ends_with(r#"" width="10"/></svg>"#));
        assert_eq!(media.len(), 1);
        let only = media.values().next().unwrap();
        assert_eq!(only.mime, "image/png");
        assert_eq!(only.bytes, vec![0, 0, 0]);
    }

    #[test]
    fn externalize_dedups_repeated_media() {
        let svg = concat!(
            r#"<svg><image href="data:image/png;base64,AAAA"/>"#,
            r#"<image href="data:image/png;base64,AAAA"/>"#,
            r#"<image href="data:image/jpeg;base64,BBBB"/></svg>"#
        );
        let (_out, media) = externalize_media(svg);
        // Same payload -> same hash key, jpeg one is distinct.
        assert_eq!(media.len(), 2);
        assert!(media.values().any(|m| m.mime == "image/png"));
        assert!(media.values().any(|m| m.mime == "image/jpeg"));
    }

    #[test]
    fn externalize_passthrough_when_no_data_uri() {
        let svg = r#"<svg><rect width="10"/></svg>"#;
        let (out, media) = externalize_media(svg);
        assert_eq!(out, svg);
        assert!(media.is_empty());
    }
}
