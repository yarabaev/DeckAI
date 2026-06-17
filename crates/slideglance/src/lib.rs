//! End-to-end PPTX parsing orchestrator.
//!
//! Mirrors (`parsePptxData` +
//! `parseSlideWithLayout`). Takes a PPTX byte stream and returns a fully
//! resolved [`Presentation`] with every slide already merged with its
//! layout / master inheritance and text-style chain.

#![deny(missing_docs)]

use std::collections::BTreeMap;
use std::sync::OnceLock;

use regex::Regex;
use slideglance_color::{ColorMap, ColorResolver, ColorScheme, Rgb};
use slideglance_model::{FontScheme, PlaceholderStyleInfo, Presentation, RenderedSlide, Theme};
use slideglance_parser::{
    apply_text_style_inheritance, build_rels_path, parse_notes_text, parse_presentation,
    parse_relationships, parse_slide, parse_theme, resolve_relationship_target, ArchiveError,
    PptxArchive, Relationship, TextStyleContext, XmlError,
};

mod cache;
mod clr_map_override;
mod convert;
mod doc;
mod embedded_fonts;
mod font_usage;

pub use embedded_fonts::{
    additional_to_faces, extract_embedded_faces, extract_embedded_fonts_split,
    render_embedded_font_defs, AdditionalFont, CompressedEmbeddedFont, EmbedFormat,
    EmbeddedFontError, EmbeddedFontFace,
};

pub use convert::{
    convert_to_png, convert_to_svg, ConvertError, ConvertOptions, FontConfig, SlideImage, SlideSvg,
};
// `RenderMode` is canonical to `slideglance_font::text_engine` per R-C3.
// Re-export here so `slideglance::RenderMode` is the same type as
// `slideglance_font::RenderMode` — no new definition site.
pub use slideglance_font::RenderMode;

pub use doc::{MediaBlob, PptxDocument, RenderedSlide as DocRenderedSlide, SlideRenderOptions};

pub use font_usage::{build_typeface_usage, TypefaceUsage};

/// Re-exports for host-system font discovery — only available when the
/// `system-fonts` cargo feature is enabled. Prefer
/// [`load_system_font_bytes_for_families`] (filtered by the typeface
/// names the deck actually references — extract them with
/// [`extract_referenced_font_families`]) over [`load_system_font_bytes`]
/// (slurps the entire OS font collection, hundreds of MB of disk I/O on
/// a fresh process).
#[cfg(feature = "system-fonts")]
pub use slideglance_font::{
    load_system_font_bytes, load_system_font_bytes_for_families, load_system_fonts,
};

/// Re-exports for CJK fallback expansion. Callers feeding
/// [`PptxDocument::parse`]'s `measurement_fonts` argument typically
/// want host-system bytes for the typefaces a deck literally
/// references **plus** the per-OS fallback chain those names resolve
/// through.
#[cfg(feature = "system-fonts")]
pub use slideglance_font::{
    default_font_mapping, get_cjk_fallback_fonts, get_mapped_font, CjkPlatform, FontFace,
    FontMapping,
};

use cache::{LayoutCache, MasterCache};

const PRES_PATH: &str = "ppt/presentation.xml";
const PRES_RELS_PATH: &str = "ppt/_rels/presentation.xml.rels";

/// Walk every XML part inside a PPTX archive and return the unique set
/// of typeface family names referenced by `<a:font script="…" typeface="…"/>`
/// (theme major / minor + Latin / EA / CS) and inline `<a:latin>`,
/// `<a:ea>`, `<a:cs>`, `<a:sym>` `typeface="…"` attrs on text-run
/// properties. The scan is regex-only — it never builds a parser tree —
/// so its cost is bounded by the total size of the deck's XML, not by
/// the slide / shape graph depth.
///
/// Designed to feed [`load_system_font_bytes_for_families`] when the
/// `system-fonts` feature is in play: extract the small set of names a
/// deck actually uses and ask fontdb for only those, instead of
/// reading every system font from disk on every open.
///
/// Returns an empty `Vec` on archive / read errors — callers fall back
/// to whatever measurer the renderer can build without host fonts.
#[must_use]
pub fn extract_referenced_font_families(bytes: &[u8]) -> Vec<String> {
    let archive = match PptxArchive::open(bytes.to_vec()) {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };
    let typeface_re = Regex::new(r#"typeface\s*=\s*"([^"]+)""#).expect("static regex");
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for (path, content) in archive.xml_files() {
        if !path.starts_with("ppt/") {
            continue;
        }
        for cap in typeface_re.captures_iter(content) {
            let typeface = cap[1].trim();
            if typeface.is_empty() {
                continue;
            }
            // PPTX uses `+mj-lt` / `+mn-ea` / etc. as theme-pointer
            // shorthands rather than literal family names.
            if typeface.starts_with('+') {
                continue;
            }
            seen.insert(typeface.to_owned());
        }
    }
    seen.into_iter().collect()
}

/// Errors surfaced by [`parse_pptx`].
#[derive(Debug)]
pub enum PptxError {
    /// Underlying ZIP archive failure.
    Archive(ArchiveError),
    /// XML parsing failure (malformed or schema-incompatible content).
    Xml(XmlError),
    /// `ppt/presentation.xml` is missing — not a valid PPTX.
    MissingPresentation,
}

impl std::fmt::Display for PptxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Archive(e) => write!(f, "pptx archive error: {e}"),
            Self::Xml(e) => write!(f, "pptx xml error: {e}"),
            Self::MissingPresentation => write!(f, "missing ppt/presentation.xml in archive"),
        }
    }
}

impl std::error::Error for PptxError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Archive(e) => Some(e),
            Self::Xml(e) => Some(e),
            Self::MissingPresentation => None,
        }
    }
}

impl From<ArchiveError> for PptxError {
    fn from(e: ArchiveError) -> Self {
        Self::Archive(e)
    }
}

impl From<XmlError> for PptxError {
    fn from(e: XmlError) -> Self {
        Self::Xml(e)
    }
}

/// Parses a complete PPTX byte stream into a [`Presentation`] model.
///
/// Walks the document graph in a single pass:
/// 1. Loads `ppt/presentation.xml` + its `.rels`.
/// 2. Resolves the theme from the presentation's `theme` relationship.
/// 3. Resolves every slide → layout → master chain, caching layouts and
///    masters by archive path so siblings share a single parse.
/// 4. For each slide, applies clrMapOvr (slide → layout → master), merges
///    layout/master placeholder geometry, calls
///    [`slideglance_parser::parse_slide`], pulls in notes, and applies text-style
///    inheritance.
///
/// # Errors
///
/// Returns [`PptxError`] on archive read failure, XML malformation, or a
/// missing `ppt/presentation.xml` entry.
pub fn parse_pptx(bytes: impl Into<Vec<u8>>) -> Result<Presentation, PptxError> {
    let mut archive = PptxArchive::open(bytes)?;
    let pres_xml = archive
        .xml(PRES_PATH)
        .ok_or(PptxError::MissingPresentation)?
        .to_owned();
    let pres_info = parse_presentation(&pres_xml)?;

    let pres_rels = archive
        .xml(PRES_RELS_PATH)
        .map(parse_relationships)
        .transpose()?
        .unwrap_or_default();

    let theme = resolve_theme(&pres_rels, &mut archive)?;

    // Slide path resolution: slide_r_ids[i] gives the rId for slide i+1.
    let slide_paths: Vec<(u32, String)> = pres_info
        .slide_r_ids
        .iter()
        .enumerate()
        .filter_map(|(i, r_id)| {
            let rel = pres_rels.get(r_id)?;
            let path = resolve_relationship_target(PRES_PATH, &rel.target);
            Some((u32::try_from(i).ok()? + 1, path))
        })
        .collect();

    let mut master_cache = MasterCache::default();
    let mut layout_cache = LayoutCache::default();
    let mut slides: Vec<RenderedSlide> = Vec::with_capacity(slide_paths.len());

    for (slide_number, slide_path) in slide_paths {
        if let Some(rendered) = parse_slide_with_layout(
            slide_number,
            &slide_path,
            &theme,
            pres_info.default_text_style.as_ref(),
            &mut archive,
            &mut master_cache,
            &mut layout_cache,
        )? {
            slides.push(rendered);
        }
    }

    Ok(Presentation {
        info: pres_info,
        theme,
        slides,
    })
}

fn resolve_theme(
    pres_rels: &BTreeMap<String, Relationship>,
    archive: &mut PptxArchive,
) -> Result<Theme, PptxError> {
    for rel in pres_rels.values() {
        if rel.ty.contains("theme") {
            let theme_path = resolve_relationship_target(PRES_PATH, &rel.target);
            if let Some(xml) = archive.xml(&theme_path) {
                let theme_xml = xml.to_owned();
                return Ok(parse_theme(&theme_xml)?);
            }
            break;
        }
    }
    Ok(default_theme())
}

fn default_theme() -> Theme {
    Theme {
        color_scheme: default_color_scheme(),
        font_scheme: default_font_scheme(),
        fmt_scheme: None,
        color_map: None,
    }
}

fn default_color_scheme() -> ColorScheme {
    ColorScheme {
        dk1: Rgb::new(0, 0, 0),
        lt1: Rgb::new(0xFF, 0xFF, 0xFF),
        dk2: Rgb::new(0x44, 0x54, 0x6A),
        lt2: Rgb::new(0xE7, 0xE6, 0xE6),
        accent1: Rgb::new(0x44, 0x72, 0xC4),
        accent2: Rgb::new(0xED, 0x7D, 0x31),
        accent3: Rgb::new(0xA5, 0xA5, 0xA5),
        accent4: Rgb::new(0xFF, 0xC0, 0x00),
        accent5: Rgb::new(0x5B, 0x9B, 0xD5),
        accent6: Rgb::new(0x70, 0xAD, 0x47),
        hlink: Rgb::new(0x05, 0x63, 0xC1),
        fol_hlink: Rgb::new(0x95, 0x4F, 0x72),
    }
}

fn default_font_scheme() -> FontScheme {
    FontScheme {
        major_font: "Calibri".to_owned(),
        minor_font: "Calibri".to_owned(),
        major_font_ea: None,
        minor_font_ea: None,
        major_font_cs: None,
        minor_font_cs: None,
        major_script_fonts: BTreeMap::new(),
        minor_script_fonts: BTreeMap::new(),
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn parse_slide_with_layout(
    slide_number: u32,
    slide_path: &str,
    theme: &Theme,
    default_text_style: Option<&slideglance_model::DefaultTextStyle>,
    archive: &mut PptxArchive,
    master_cache: &mut MasterCache,
    layout_cache: &mut LayoutCache,
) -> Result<Option<RenderedSlide>, PptxError> {
    if archive.xml(slide_path).is_none() {
        return Ok(None);
    }

    // Slide → layout → master path resolution.
    let slide_rels = load_rels(archive, &build_rels_path(slide_path))?;

    let layout_path = slide_rels
        .values()
        .find(|rel| rel.ty.contains("slideLayout"))
        .map(|rel| resolve_relationship_target(slide_path, &rel.target));

    let layout_rels = if let Some(lp) = layout_path.as_deref() {
        load_rels(archive, &build_rels_path(lp))?
    } else {
        BTreeMap::new()
    };

    let master_path = layout_path.as_deref().and_then(|lp| {
        layout_rels
            .values()
            .find(|rel| rel.ty.contains("slideMaster"))
            .map(|rel| resolve_relationship_target(lp, &rel.target))
    });

    // Master cache: parse-once per master path.
    if let Some(mp) = master_path.as_deref() {
        master_cache.ensure(mp, theme, archive)?;
    }
    let master = master_path.as_deref().and_then(|mp| master_cache.get(mp));

    // Layout cache: parse once per layout path. We pass the slide's color
    // resolver to the layout parse, but the resolver itself depends on
    // master's color_map + clrMapOvr. The spec re-parses layout
    // for each slide; we parse it once with the master's resolver and then
    // recompute per-slide overrides on top of that.
    let layout_color_resolver = master.map_or_else(
        || ColorResolver::new(theme.color_scheme, ColorMap::default()),
        |m| m.color_resolver,
    );

    if let Some(lp) = layout_path.as_deref() {
        layout_cache.ensure(lp, theme, &layout_color_resolver, archive)?;
    }
    let layout = layout_path.as_deref().and_then(|lp| layout_cache.get(lp));

    // Per-slide ColorResolver: starts from master, then layout clrMapOvr,
    // then slide clrMapOvr.
    let slide_xml_owned = archive
        .xml(slide_path)
        .map(str::to_owned)
        .ok_or(PptxError::MissingPresentation)?;
    let layout_xml_owned = layout_path
        .as_deref()
        .and_then(|lp| archive.xml(lp).map(str::to_owned));

    let slide_color_resolver =
        build_slide_color_resolver(&slide_xml_owned, layout_xml_owned.as_deref(), master, theme);

    // Merge layout + master placeholder styles (layout wins).
    let layout_placeholder_styles: &[PlaceholderStyleInfo] =
        layout.map_or(&[], |l| l.layout.placeholder_styles.as_slice());
    let master_placeholder_styles: &[PlaceholderStyleInfo] =
        master.map_or(&[], |m| m.master.placeholder_styles.as_slice());
    let merged_placeholders =
        merge_placeholder_geometry(layout_placeholder_styles, master_placeholder_styles);

    // Parse the slide proper.
    let mut slide = parse_slide(
        &slide_xml_owned,
        slide_path,
        slide_number,
        archive,
        &slide_color_resolver,
        Some(&theme.font_scheme),
        theme.fmt_scheme.as_ref(),
        &merged_placeholders,
    )?;

    // Background fallback chain: slide → layout → master.
    if slide.background.is_none() {
        slide.background = layout.and_then(|l| l.layout.background.clone());
    }
    if slide.background.is_none() {
        slide.background = master.and_then(|m| m.master.background.clone());
    }

    // Apply text-style inheritance using layout/master placeholder styles
    // (raw, not the merged version — spec passes them separately
    // because the resolver looks at lst_style independently from geometry).
    apply_text_style_inheritance(
        &mut slide.elements,
        &TextStyleContext {
            layout_placeholder_styles,
            master_placeholder_styles,
            tx_styles: master.and_then(|m| m.master.tx_styles.as_ref()),
            default_text_style,
            font_scheme: Some(&theme.font_scheme),
        },
    );

    // Notes integration.
    for rel in slide_rels.values() {
        if !rel.ty.contains("notesSlide") {
            continue;
        }
        let notes_path = resolve_relationship_target(slide_path, &rel.target);
        if let Some(notes_xml) = archive.xml(&notes_path) {
            let notes_xml_owned = notes_xml.to_owned();
            if let Some(notes_text) = parse_notes_text(&notes_xml_owned)? {
                slide.notes = Some(notes_text);
            }
        }
        break;
    }

    // Layout name (cSld @name) — surfaced as `data-layout-name` on the SVG.
    if let Some(lx) = layout_xml_owned.as_deref() {
        if let Some(name) = extract_layout_name(lx) {
            slide.layout_name = Some(name);
        }
    }

    let layout_show_master_sp = layout.is_none_or(|l| l.layout.show_master_sp);
    let layout_elements = layout
        .map(|l| l.layout.elements.clone())
        .unwrap_or_default();
    let master_elements = master
        .map(|m| m.master.elements.clone())
        .unwrap_or_default();

    Ok(Some(RenderedSlide {
        slide,
        layout_elements,
        master_elements,
        layout_show_master_sp,
    }))
}

fn load_rels(
    archive: &PptxArchive,
    path: &str,
) -> Result<BTreeMap<String, Relationship>, PptxError> {
    Ok(archive
        .xml(path)
        .map(parse_relationships)
        .transpose()?
        .unwrap_or_default())
}

fn build_slide_color_resolver(
    slide_xml: &str,
    layout_xml: Option<&str>,
    master: Option<&cache::CachedMaster>,
    theme: &Theme,
) -> ColorResolver {
    let base_color_map = master.map(|m| m.master.color_map).unwrap_or_default();

    let layout_override = layout_xml.and_then(clr_map_override::parse_clr_map_override);
    let slide_override = clr_map_override::parse_clr_map_override(slide_xml);

    if layout_override.is_none() && slide_override.is_none() {
        if let Some(m) = master {
            return m.color_resolver;
        }
        return ColorResolver::new(theme.color_scheme, base_color_map);
    }

    let mut effective = base_color_map;
    if let Some(o) = layout_override {
        apply_color_map_override(&mut effective, &o);
    }
    if let Some(o) = slide_override {
        apply_color_map_override(&mut effective, &o);
    }
    ColorResolver::new(theme.color_scheme, effective)
}

fn apply_color_map_override(dst: &mut ColorMap, src: &clr_map_override::ColorMapOverride) {
    macro_rules! merge {
        ($field:ident) => {
            if let Some(v) = src.$field {
                dst.$field = v;
            }
        };
    }
    merge!(bg1);
    merge!(tx1);
    merge!(bg2);
    merge!(tx2);
    merge!(accent1);
    merge!(accent2);
    merge!(accent3);
    merge!(accent4);
    merge!(accent5);
    merge!(accent6);
    merge!(hlink);
    merge!(fol_hlink);
}

/// Merge layout-level placeholder styles with master fallback. Layout wins
/// for transform/geometry; master-only placeholders are appended. Mirrors
/// .
fn merge_placeholder_geometry(
    layout: &[PlaceholderStyleInfo],
    master: &[PlaceholderStyleInfo],
) -> Vec<PlaceholderStyleInfo> {
    let mut merged: Vec<PlaceholderStyleInfo> = layout
        .iter()
        .map(|ls| {
            let master_match =
                find_placeholder_by_type_and_idx(&ls.placeholder_type, ls.placeholder_idx, master);
            let mut filled = ls.clone();
            if let Some(m) = master_match {
                if filled.transform.is_none() {
                    filled.transform = m.transform;
                }
                if filled.geometry.is_none() {
                    filled.geometry.clone_from(&m.geometry);
                }
                // Field-wise merge of explicit bodyPr attributes: layout
                // wins per attribute, falling through to master only for
                // attributes the layout left absent. Mirrors MS-OE376
                // §5.1.5.1.1 ancestor-walk semantics.
                let merged_pbp = merge_placeholder_body_pr(
                    filled.body_properties.as_ref(),
                    m.body_properties.as_ref(),
                );
                filled.body_properties = merged_pbp;
            }
            filled
        })
        .collect();

    for ms in master {
        let exists = merged.iter().any(|m| {
            m.placeholder_type == ms.placeholder_type && m.placeholder_idx == ms.placeholder_idx
        });
        if !exists {
            merged.push(ms.clone());
        }
    }
    merged
}

fn merge_placeholder_body_pr(
    layout: Option<&slideglance_model::PlaceholderBodyPr>,
    master: Option<&slideglance_model::PlaceholderBodyPr>,
) -> Option<slideglance_model::PlaceholderBodyPr> {
    match (layout, master) {
        (None, None) => None,
        (Some(l), None) => Some(l.clone()),
        (None, Some(m)) => Some(m.clone()),
        (Some(l), Some(m)) => Some(slideglance_model::PlaceholderBodyPr {
            anchor: l.anchor.or(m.anchor),
            margin_left: l.margin_left.or(m.margin_left),
            margin_right: l.margin_right.or(m.margin_right),
            margin_top: l.margin_top.or(m.margin_top),
            margin_bottom: l.margin_bottom.or(m.margin_bottom),
            wrap: l.wrap.or(m.wrap),
            vert: l.vert.or(m.vert),
            num_col: l.num_col.or(m.num_col),
            spc_first_last_para: l.spc_first_last_para.or(m.spc_first_last_para),
            compat_ln_spc: l.compat_ln_spc.or(m.compat_ln_spc),
            auto_fit: l.auto_fit.or(m.auto_fit),
            font_scale: l.font_scale.or(m.font_scale),
            ln_spc_reduction: l.ln_spc_reduction.or(m.ln_spc_reduction),
        }),
    }
}

fn find_placeholder_by_type_and_idx<'a>(
    ph_type: &str,
    ph_idx: Option<u32>,
    styles: &'a [PlaceholderStyleInfo],
) -> Option<&'a PlaceholderStyleInfo> {
    if let Some(idx) = ph_idx {
        if let Some(found) = styles.iter().find(|s| s.placeholder_idx == Some(idx)) {
            return Some(found);
        }
    }
    styles.iter().find(|s| s.placeholder_type == ph_type)
}

/// Extract `cSld @name` from a slide-layout XML using a cheap regex. Avoids
/// a full XML parse since the value is only used as an output attribute.
fn extract_layout_name(layout_xml: &str) -> Option<String> {
    static LAYOUT_NAME_RE: OnceLock<Regex> = OnceLock::new();
    let re = LAYOUT_NAME_RE
        .get_or_init(|| Regex::new(r#"<p:cSld[^>]*\bname="([^"]*)""#).expect("valid"));
    re.captures(layout_xml)?
        .get(1)
        .map(|m| m.as_str().to_owned())
}

// Re-export the model's Presentation so consumers have a single entry
// point for the parsed result.
pub use slideglance_model::Presentation as ParsedPresentation;
