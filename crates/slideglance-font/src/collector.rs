//! Used-font collection from a parsed [`Presentation`].
//!
//! Mirrors. Walks the theme,
//! every slide's elements, and (de-duplicated) layout / master shapes,
//! emitting a sorted distinct list of font family names that appear in
//! the document. Used for two things in the renderer pipeline:
//!
//! 1. The `collect_used_fonts` public API (so a host can pre-fetch
//!    Google Fonts before invoking conversion).
//! 2. The font-resolver's "what do we actually need" question — the
//!    resolver can avoid loading every system font and instead resolve
//!    only the families this presentation references.
//!
//! Pure walker — no I/O, no dependencies beyond `slideglance-model`. The
//! orchestrator (in `slideglance`) hands the parsed [`Presentation`] in
//! and gets back a [`UsedFonts`] back.

use std::collections::BTreeSet;

use slideglance_model::{
    shape::SlideElement, table::TableElement, text::TextBody, theme::FontScheme, Presentation,
};

/// Result of [`collect_used_fonts`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsedFonts {
    /// Theme-defined fonts pulled out of the presentation's font scheme.
    /// Mirrors the TS `theme` object 1:1.
    pub theme: ThemeFonts,
    /// Sorted, de-duplicated list of every font family name that
    /// appears in the document — theme entries plus run / bullet
    /// references in slide / layout / master text bodies.
    pub fonts: Vec<String>,
}

/// Theme font names extracted from `<a:fontScheme>`.
///
/// Matches the spec's `theme` field exactly. Plus `script_fonts`
/// per CJK Script Equality (TS only ships major / minor latin / EA / CS;
/// see [`crate::script_context`] for the rationale).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeFonts {
    /// `<a:majorFont><a:latin>` typeface.
    pub major_font: String,
    /// `<a:minorFont><a:latin>` typeface.
    pub minor_font: String,
    /// `<a:majorFont><a:ea>` typeface.
    pub major_font_ea: Option<String>,
    /// `<a:minorFont><a:ea>` typeface.
    pub minor_font_ea: Option<String>,
    /// `<a:majorFont><a:cs>` typeface.
    pub major_font_cs: Option<String>,
    /// `<a:minorFont><a:cs>` typeface.
    pub minor_font_cs: Option<String>,
    /// Major-font script-keyed fallbacks (`<a:font script="...">`).
    /// Per CJK Script Equality every script the source XML declares is
    /// included; the spec omits this field.
    pub major_script_fonts: std::collections::BTreeMap<String, String>,
    /// Minor-font script-keyed fallbacks.
    pub minor_script_fonts: std::collections::BTreeMap<String, String>,
}

/// Walks `presentation` and collects every font family name referenced
/// by either the theme or any text run / bullet glyph.
///
/// The result's `fonts` is alphabetically sorted (TS uses
/// `[...set].sort()`); identical input always yields identical output
/// for snapshot stability.
#[must_use]
pub fn collect_used_fonts(presentation: &Presentation) -> UsedFonts {
    let mut fonts: BTreeSet<String> = BTreeSet::new();

    let scheme = &presentation.theme.font_scheme;
    collect_theme_fonts(scheme, &mut fonts);

    // De-duplicate master / layout walks: the same master can back many
    // slides; walking its elements once per slide is wasteful and the
    // spec uses a `Set` of element references for the same
    // reason. Rust has no reference identity so we hash the master /
    // layout `Vec<SlideElement>`'s `as_ptr()` value.
    let mut visited_layout_ptrs: std::collections::HashSet<usize> =
        std::collections::HashSet::new();
    let mut visited_master_ptrs: std::collections::HashSet<usize> =
        std::collections::HashSet::new();

    for rendered in &presentation.slides {
        collect_fonts_from_elements(&rendered.slide.elements, &mut fonts);

        let layout_ptr = rendered.layout_elements.as_ptr() as usize;
        if visited_layout_ptrs.insert(layout_ptr) {
            collect_fonts_from_elements(&rendered.layout_elements, &mut fonts);
        }

        let master_ptr = rendered.master_elements.as_ptr() as usize;
        if visited_master_ptrs.insert(master_ptr) {
            collect_fonts_from_elements(&rendered.master_elements, &mut fonts);
        }

        let _ = rendered; // keep clippy::unused_results quiet
    }

    UsedFonts {
        theme: ThemeFonts {
            major_font: scheme.major_font.clone(),
            minor_font: scheme.minor_font.clone(),
            major_font_ea: scheme.major_font_ea.clone(),
            minor_font_ea: scheme.minor_font_ea.clone(),
            major_font_cs: scheme.major_font_cs.clone(),
            minor_font_cs: scheme.minor_font_cs.clone(),
            major_script_fonts: scheme.major_script_fonts.clone(),
            minor_script_fonts: scheme.minor_script_fonts.clone(),
        },
        fonts: fonts.into_iter().collect(),
    }
}

fn collect_theme_fonts(scheme: &FontScheme, fonts: &mut BTreeSet<String>) {
    fonts.insert(scheme.major_font.clone());
    fonts.insert(scheme.minor_font.clone());
    if let Some(name) = &scheme.major_font_ea {
        fonts.insert(name.clone());
    }
    if let Some(name) = &scheme.minor_font_ea {
        fonts.insert(name.clone());
    }
    if let Some(name) = &scheme.major_font_cs {
        fonts.insert(name.clone());
    }
    if let Some(name) = &scheme.minor_font_cs {
        fonts.insert(name.clone());
    }
    // CJK Script Equality additions — spec omits these.
    for typeface in scheme.major_script_fonts.values() {
        fonts.insert(typeface.clone());
    }
    for typeface in scheme.minor_script_fonts.values() {
        fonts.insert(typeface.clone());
    }
}

#[allow(unused_results)] // BTreeSet::insert returns bool we discard
pub(crate) fn collect_fonts_from_elements(elements: &[SlideElement], fonts: &mut BTreeSet<String>) {
    for el in elements {
        match el {
            SlideElement::Shape(shape) => {
                if let Some(body) = &shape.text_body {
                    collect_fonts_from_text_body(body, fonts);
                }
            }
            SlideElement::Group(group) => {
                collect_fonts_from_elements(&group.children, fonts);
            }
            SlideElement::Table(table) => {
                collect_fonts_from_table(table, fonts);
            }
            // Image / Connector / Chart carry no text in the model
            // surface this function operates on. Charts have axis /
            // legend text but those are encoded in `chart.xml` which
            // ships its own text-body trees; revisit if the chart
            // renderer surfaces those.
            SlideElement::Image(_) | SlideElement::Connector(_) | SlideElement::Chart(_) => {}
        }
    }
}

fn collect_fonts_from_table(table: &TableElement, fonts: &mut BTreeSet<String>) {
    for row in &table.table.rows {
        for cell in &row.cells {
            if let Some(body) = &cell.text_body {
                collect_fonts_from_text_body(body, fonts);
            }
        }
    }
}

pub(crate) fn collect_fonts_from_text_body(body: &TextBody, fonts: &mut BTreeSet<String>) {
    for paragraph in &body.paragraphs {
        if let Some(name) = &paragraph.properties.bullet_font {
            fonts.insert(name.clone());
        }
        for run in &paragraph.runs {
            if let Some(name) = &run.properties.font_family {
                fonts.insert(name.clone());
            }
            if let Some(name) = &run.properties.font_family_ea {
                fonts.insert(name.clone());
            }
            if let Some(name) = &run.properties.font_family_cs {
                fonts.insert(name.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use slideglance_model::text::{
        AutoFit, BodyProperties, Paragraph, ParagraphAlignment, ParagraphProperties, RunProperties,
        TextBody, TextRun, TextVerticalType, VerticalAnchor, WrapMode,
    };
    use slideglance_utils::{Emu, Pt};

    use super::*;

    // Test strategy: top-level `collect_used_fonts` is exercised at the
    // slideglance integration-test level with real fixtures (where building a
    // `Presentation` is straightforward via the parser). Here we test
    // only the data-walking primitives (`collect_fonts_from_text_body`,
    // `collect_fonts_from_elements` on empty input, ThemeFonts shape).

    fn body_props() -> BodyProperties {
        BodyProperties {
            anchor: VerticalAnchor::T,
            margin_left: Emu(0),
            margin_right: Emu(0),
            margin_top: Emu(0),
            margin_bottom: Emu(0),
            wrap: WrapMode::Square,
            auto_fit: AutoFit::NoAutofit,
            font_scale: 1.0,
            ln_spc_reduction: 0.0,
            num_col: 1,
            vert: TextVerticalType::Horz,
            spc_first_last_para: false,
            compat_ln_spc: false,
            prst_tx_warp: None,
        }
    }

    fn paragraph_with_runs(runs: Vec<TextRun>) -> Paragraph {
        Paragraph {
            runs,
            properties: ParagraphProperties {
                alignment: Some(ParagraphAlignment::L),
                line_spacing: None,
                space_before: None,
                space_after: None,
                level: 0,
                bullet: None,
                bullet_font: None,
                bullet_color: None,
                bullet_size_pct: None,
                margin_left: None,
                indent: None,
                tab_stops: Vec::new(),
            },
            end_para_run_properties: None,
        }
    }

    fn run_with_font(font_family: &str) -> TextRun {
        TextRun {
            text: "x".to_string(),
            properties: RunProperties {
                font_size: Some(Pt(18.0)),
                font_family: Some(font_family.to_string()),
                ..RunProperties::default()
            },
            field_type: None,
        }
    }

    fn text_body_with(paragraphs: Vec<Paragraph>) -> TextBody {
        TextBody {
            default_text_color: None,
            paragraphs,
            body_properties: body_props(),
        }
    }

    // -- collect_fonts_from_text_body internals ------------------------------

    #[test]
    fn run_font_family_collected() {
        let body = text_body_with(vec![paragraph_with_runs(vec![run_with_font(
            "Comic Sans MS",
        )])]);
        let mut fonts = BTreeSet::new();
        collect_fonts_from_text_body(&body, &mut fonts);
        assert!(fonts.contains("Comic Sans MS"));
    }

    #[test]
    fn run_ea_and_cs_fonts_collected() {
        let mut run = run_with_font("Calibri");
        run.properties.font_family_ea = Some("メイリオ".to_string());
        run.properties.font_family_cs = Some("Arial".to_string());
        let body = text_body_with(vec![paragraph_with_runs(vec![run])]);
        let mut fonts = BTreeSet::new();
        collect_fonts_from_text_body(&body, &mut fonts);
        assert!(fonts.contains("Calibri"));
        assert!(fonts.contains("メイリオ"));
        assert!(fonts.contains("Arial"));
    }

    #[test]
    fn bullet_font_collected() {
        let mut paragraph = paragraph_with_runs(vec![run_with_font("Calibri")]);
        paragraph.properties.bullet_font = Some("Wingdings".to_string());
        let body = text_body_with(vec![paragraph]);
        let mut fonts = BTreeSet::new();
        collect_fonts_from_text_body(&body, &mut fonts);
        assert!(fonts.contains("Wingdings"));
        assert!(fonts.contains("Calibri"));
    }

    #[test]
    fn empty_text_body_collects_nothing() {
        let body = text_body_with(Vec::new());
        let mut fonts = BTreeSet::new();
        collect_fonts_from_text_body(&body, &mut fonts);
        assert!(fonts.is_empty());
    }

    #[test]
    fn run_with_no_font_family_collects_nothing() {
        let run = TextRun {
            text: "x".to_string(),
            properties: RunProperties::default(),
            field_type: None,
        };
        let body = text_body_with(vec![paragraph_with_runs(vec![run])]);
        let mut fonts = BTreeSet::new();
        collect_fonts_from_text_body(&body, &mut fonts);
        assert!(fonts.is_empty());
    }

    // -- collect_fonts_from_elements: empty / no-text variants --------------

    #[test]
    fn empty_elements_slice_collects_nothing() {
        let mut fonts = BTreeSet::new();
        collect_fonts_from_elements(&[], &mut fonts);
        assert!(fonts.is_empty());
    }

    // -- collect_theme_fonts -------------------------------------------------

    #[test]
    fn theme_fonts_collected() {
        let scheme = FontScheme {
            major_font: "Calibri Light".to_string(),
            minor_font: "Calibri".to_string(),
            major_font_ea: Some("Yu Gothic".to_string()),
            minor_font_ea: Some("Yu Mincho".to_string()),
            major_font_cs: Some("Arial".to_string()),
            minor_font_cs: Some("Arial".to_string()),
            major_script_fonts: BTreeMap::from_iter([
                ("Jpan".to_string(), "Yu Gothic".to_string()),
                ("Hang".to_string(), "맑은 고딕".to_string()),
            ]),
            minor_script_fonts: BTreeMap::from_iter([(
                "Hans".to_string(),
                "Microsoft YaHei".to_string(),
            )]),
        };
        let mut fonts = BTreeSet::new();
        collect_theme_fonts(&scheme, &mut fonts);
        assert!(fonts.contains("Calibri"));
        assert!(fonts.contains("Calibri Light"));
        assert!(fonts.contains("Yu Gothic"));
        assert!(fonts.contains("Yu Mincho"));
        assert!(fonts.contains("Arial"));
        assert!(fonts.contains("맑은 고딕"));
        assert!(fonts.contains("Microsoft YaHei"));
    }

    #[test]
    fn cjk_script_equality_includes_all_scripts() {
        // Project rule verification: Hang / Hans / Hant entries in the
        // script-fonts map are collected just like Jpan.
        let scheme = FontScheme {
            major_font: "Roboto".to_string(),
            minor_font: "Roboto".to_string(),
            major_font_ea: None,
            minor_font_ea: None,
            major_font_cs: None,
            minor_font_cs: None,
            major_script_fonts: BTreeMap::from_iter([
                ("Jpan".to_string(), "JPFont".to_string()),
                ("Hang".to_string(), "KRFont".to_string()),
                ("Hans".to_string(), "SCFont".to_string()),
                ("Hant".to_string(), "TCFont".to_string()),
            ]),
            minor_script_fonts: BTreeMap::new(),
        };
        let mut fonts = BTreeSet::new();
        collect_theme_fonts(&scheme, &mut fonts);
        for cjk_font in ["JPFont", "KRFont", "SCFont", "TCFont"] {
            assert!(fonts.contains(cjk_font), "missing {cjk_font}");
        }
    }
}
