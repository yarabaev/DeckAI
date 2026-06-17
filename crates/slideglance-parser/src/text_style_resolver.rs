//! Text-style inheritance walker.
//!
//! Mirrors.
//! Walks already-parsed slide elements and fills in `None` paragraph and run
//! properties from the layout-placeholder → master-placeholder → txStyle →
//! defaultTextStyle chain. The mutation is in-place because the chain is
//! resolved once per slide after layout/master are known, and the resolved
//! values then feed straight into the renderer.

use slideglance_model::{
    BulletType, DefaultParagraphLevelProperties, DefaultRunProperties, DefaultTextStyle,
    FontScheme, ParagraphAlignment, ParagraphProperties, PlaceholderStyleInfo, ShapeElement,
    SlideElement, TxStyles,
};

use crate::text_style::resolve_theme_font;

/// Inputs for one inheritance pass.
pub struct TextStyleContext<'a> {
    /// Layout-level placeholder styles (highest priority after the shape's
    /// own `<p:txBody>`).
    pub layout_placeholder_styles: &'a [PlaceholderStyleInfo],
    /// Master-level placeholder styles.
    pub master_placeholder_styles: &'a [PlaceholderStyleInfo],
    /// Master `<p:txStyles>`.
    pub tx_styles: Option<&'a TxStyles>,
    /// Presentation-level `<p:defaultTextStyle>`.
    pub default_text_style: Option<&'a DefaultTextStyle>,
    /// Theme font scheme — needed because `lst_style` entries can reference
    /// theme-font tokens (`+mj-lt` etc.) that have to be resolved against
    /// the current theme.
    pub font_scheme: Option<&'a FontScheme>,
}

/// Apply the inheritance chain to every shape under `elements` (recursing
/// into groups). Mutates [`slideglance_model::TextRun::properties`] and
/// [`slideglance_model::Paragraph::properties`] in place.
pub fn apply_text_style_inheritance(elements: &mut [SlideElement], ctx: &TextStyleContext<'_>) {
    for element in elements {
        match element {
            SlideElement::Shape(shape) => resolve_shape_text(shape, ctx),
            SlideElement::Group(group) => {
                apply_text_style_inheritance(&mut group.children, ctx);
            }
            _ => {}
        }
    }
}

fn resolve_shape_text(shape: &mut ShapeElement, ctx: &TextStyleContext<'_>) {
    let Some(text_body) = shape.text_body.as_mut() else {
        return;
    };

    let layout_lst_style = find_matching_placeholder_style(
        shape.placeholder_type.as_deref(),
        shape.placeholder_idx,
        ctx.layout_placeholder_styles,
    );
    let master_lst_style = find_matching_placeholder_style(
        shape.placeholder_type.as_deref(),
        shape.placeholder_idx,
        ctx.master_placeholder_styles,
    );
    let tx_style = get_tx_style_for_placeholder(shape.placeholder_type.as_deref(), ctx.tx_styles);

    // Fixed-length array of style sources, in inheritance priority order.
    // `None` slots are skipped during chain iteration.
    let chain: [Option<&DefaultTextStyle>; 4] = [
        layout_lst_style,
        master_lst_style,
        tx_style,
        ctx.default_text_style,
    ];

    // Pre-seed each run's `color` with the shape's `<p:style><a:fontRef>`
    // colour (resolved into `text_body.default_text_color`) before the
    // chain cascades. PowerPoint's inheritance order treats fontRef as
    // a stronger source than the layout/master textStyles' default
    // run-property colour, so without this seed the chain cascades a
    // theme `dk1` (= black) into colour-less runs and the renderer
    // never falls back to fontRef. Slide 56 has a `lt1` (white)
    // fontRef on a "law-text-on-white-card" shape; without this fix
    // the word "법령표준의…" was inherited black and showed through
    // the white card instead of disappearing.
    if let Some(default_color) = text_body.default_text_color {
        for paragraph in &mut text_body.paragraphs {
            for run in &mut paragraph.runs {
                if run.properties.color.is_none() {
                    run.properties.color = Some(default_color);
                }
            }
        }
    }

    for paragraph in &mut text_body.paragraphs {
        resolve_paragraph(&mut paragraph.properties, &chain);
        for run in &mut paragraph.runs {
            resolve_run(
                &mut run.properties,
                paragraph.properties.level,
                &chain,
                ctx.font_scheme,
            );
        }
    }
}

#[allow(clippy::too_many_lines)]
fn resolve_paragraph(props: &mut ParagraphProperties, chain: &[Option<&DefaultTextStyle>; 4]) {
    let level = props.level;

    // Alignment.
    if props.alignment.is_none() {
        for source in chain.iter().flatten() {
            if let Some(found) = level_props(source, level)
                .and_then(|p| p.alignment)
                .or_else(|| source.default_paragraph.as_ref().and_then(|p| p.alignment))
            {
                props.alignment = Some(found);
                break;
            }
        }
        // OOXML default is left-aligned when no source supplies an alignment.
        if props.alignment.is_none() {
            props.alignment = Some(ParagraphAlignment::L);
        }
    }

    // Bullet.
    if props.bullet.is_none() {
        for source in chain.iter().flatten() {
            let lp = level_props(source, level).or(source.default_paragraph.as_ref());
            if let Some(bullet) = lp.and_then(|p| p.bullet.clone()) {
                props.bullet = Some(bullet);
                break;
            }
        }
    }

    // Bullet auxiliary attributes are inherited only when the resolved bullet
    // is something visible (not `BulletType::None`).
    let has_bullet = !matches!(props.bullet, None | Some(BulletType::None));
    if has_bullet {
        for source in chain.iter().flatten() {
            let lp = level_props(source, level).or(source.default_paragraph.as_ref());
            let Some(lp) = lp else { continue };
            if props.bullet_font.is_none() {
                if let Some(font) = lp.bullet_font.clone() {
                    props.bullet_font = Some(font);
                }
            }
            if props.bullet_color.is_none() {
                if let Some(color) = lp.bullet_color {
                    props.bullet_color = Some(color);
                }
            }
            if props.bullet_size_pct.is_none() {
                if let Some(pct) = lp.bullet_size_pct {
                    props.bullet_size_pct = Some(pct);
                }
            }
            if props.bullet_font.is_some()
                && props.bullet_color.is_some()
                && props.bullet_size_pct.is_some()
            {
                break;
            }
        }
    }

    // marginLeft / indent.
    if props.margin_left.is_none() || props.indent.is_none() {
        for source in chain.iter().flatten() {
            let lp = level_props(source, level).or(source.default_paragraph.as_ref());
            let Some(lp) = lp else { continue };
            if props.margin_left.is_none() {
                if let Some(m) = lp.margin_left {
                    props.margin_left = Some(m);
                }
            }
            if props.indent.is_none() {
                if let Some(i) = lp.indent {
                    props.indent = Some(i);
                }
            }
            if props.margin_left.is_some() && props.indent.is_some() {
                break;
            }
        }
    }

    // line_spacing / space_before / space_after — same chain shape; the
    // resolver fills `None` slots with the first source that provides a
    // value, then leaves the renderer's `unwrap_or(0)` to drive the
    // ultimate default. Without this, master-level `<a:bodyStyle>`
    // line-spacing / paragraph-before defaults silently dropped on all
    // bulleted body lists, packing them ~2× tighter than PowerPoint
    // (visible on the slide-105 ToC and several layout-driven decks).
    if props.line_spacing.is_none() || props.space_before.is_none() || props.space_after.is_none() {
        for source in chain.iter().flatten() {
            let lp = level_props(source, level).or(source.default_paragraph.as_ref());
            let Some(lp) = lp else { continue };
            if props.line_spacing.is_none() {
                if let Some(v) = lp.line_spacing {
                    props.line_spacing = Some(v);
                }
            }
            if props.space_before.is_none() {
                if let Some(v) = lp.space_before {
                    props.space_before = Some(v);
                }
            }
            if props.space_after.is_none() {
                if let Some(v) = lp.space_after {
                    props.space_after = Some(v);
                }
            }
            if props.line_spacing.is_some()
                && props.space_before.is_some()
                && props.space_after.is_some()
            {
                break;
            }
        }
    }
}

fn resolve_run(
    props: &mut slideglance_model::RunProperties,
    level: u8,
    chain: &[Option<&DefaultTextStyle>; 4],
    font_scheme: Option<&FontScheme>,
) {
    for source in chain.iter().flatten() {
        if props.font_size.is_some()
            && props.font_family.is_some()
            && props.font_family_ea.is_some()
            && props.font_family_cs.is_some()
            && props.color.is_some()
        {
            return;
        }
        let Some(def_r_pr) = def_r_pr_from_style(source, level) else {
            continue;
        };
        if props.font_size.is_none() {
            if let Some(sz) = def_r_pr.font_size {
                props.font_size = Some(sz);
            }
        }
        if props.font_family.is_none() {
            if let Some(font) = def_r_pr.font_family.as_deref() {
                props.font_family = resolve_theme_font(Some(font), font_scheme);
            }
        }
        if props.font_family_ea.is_none() {
            if let Some(font) = def_r_pr.font_family_ea.as_deref() {
                props.font_family_ea = resolve_theme_font(Some(font), font_scheme);
            }
        }
        if props.font_family_cs.is_none() {
            if let Some(font) = def_r_pr.font_family_cs.as_deref() {
                props.font_family_cs = resolve_theme_font(Some(font), font_scheme);
            }
        }
        if props.color.is_none() {
            if let Some(color) = def_r_pr.color {
                props.color = Some(color);
            }
        }
    }
}

fn level_props(style: &DefaultTextStyle, level: u8) -> Option<&DefaultParagraphLevelProperties> {
    style.levels.get(level as usize)?.as_ref()
}

fn find_matching_placeholder_style<'a>(
    placeholder_type: Option<&str>,
    placeholder_idx: Option<u32>,
    styles: &'a [PlaceholderStyleInfo],
) -> Option<&'a DefaultTextStyle> {
    let ph_type = placeholder_type?;

    // idx match takes priority.
    if let Some(idx) = placeholder_idx {
        if let Some(style) = styles
            .iter()
            .find(|s| s.placeholder_idx == Some(idx))
            .and_then(|s| s.lst_style.as_ref())
        {
            return Some(style);
        }
    }

    // Type match.
    if let Some(style) = styles
        .iter()
        .find(|s| s.placeholder_type == ph_type)
        .and_then(|s| s.lst_style.as_ref())
    {
        return Some(style);
    }

    // OOXML fallback types: ctrTitle → title, subTitle → body.
    let fallback = match ph_type {
        "ctrTitle" => Some("title"),
        "subTitle" => Some("body"),
        _ => None,
    };
    if let Some(ft) = fallback {
        return styles
            .iter()
            .find(|s| s.placeholder_type == ft)
            .and_then(|s| s.lst_style.as_ref());
    }
    None
}

fn get_tx_style_for_placeholder<'a>(
    placeholder_type: Option<&str>,
    tx_styles: Option<&'a TxStyles>,
) -> Option<&'a DefaultTextStyle> {
    let tx = tx_styles?;
    let Some(ty) = placeholder_type else {
        return tx.other_style.as_ref();
    };
    match ty {
        "title" | "ctrTitle" => tx.title_style.as_ref(),
        "body" | "subTitle" | "obj" => tx.body_style.as_ref(),
        _ => tx.other_style.as_ref(),
    }
}

fn def_r_pr_from_style(style: &DefaultTextStyle, level: u8) -> Option<&DefaultRunProperties> {
    if let Some(level_props) = level_props(style, level) {
        if let Some(def_r_pr) = level_props.default_run_properties.as_ref() {
            return Some(def_r_pr);
        }
    }
    style
        .default_paragraph
        .as_ref()?
        .default_run_properties
        .as_ref()
}
