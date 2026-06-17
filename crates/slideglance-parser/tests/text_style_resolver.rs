//! Text-style inheritance resolver integration tests. Mirrors representative
//! cases from.

use slideglance_color::{ResolvedColor, Rgb};
use slideglance_model::{
    BodyProperties, BulletType, DefaultParagraphLevelProperties, DefaultRunProperties,
    DefaultTextStyle, Paragraph, ParagraphAlignment, ParagraphProperties, PlaceholderStyleInfo,
    PresetGeometry, RunProperties, ShapeElement, SlideElement, SpacingValue, TextBody, TextRun,
    Transform, TxStyles, WrapMode,
};
use slideglance_parser::{apply_text_style_inheritance, TextStyleContext};
use slideglance_utils::{Emu, HundredthPt, Pt};

fn empty_run_props() -> RunProperties {
    RunProperties {
        font_size: None,
        font_family: None,
        font_family_ea: None,
        font_family_cs: None,
        bold: false,
        italic: false,
        underline: false,
        strikethrough: false,
        color: None,
        baseline: 0.0,
        hyperlink: None,
        outline: None,
        highlight: None,
        font_family_sym: None,
        kern: None,
        char_spacing: None,
    }
}

fn empty_paragraph_props() -> ParagraphProperties {
    ParagraphProperties {
        alignment: None,
        line_spacing: None,
        space_before: Some(SpacingValue::Pts {
            value: HundredthPt::new(0),
        }),
        space_after: Some(SpacingValue::Pts {
            value: HundredthPt::new(0),
        }),
        level: 0,
        bullet: None,
        bullet_font: None,
        bullet_color: None,
        bullet_size_pct: None,
        margin_left: None,
        indent: None,
        tab_stops: Vec::new(),
    }
}

fn shape_with_text(text_body: TextBody) -> SlideElement {
    SlideElement::Shape(ShapeElement {
        sp_id: None,
        transform: Transform::default(),
        geometry: slideglance_model::Geometry::Preset(PresetGeometry {
            preset: "rect".to_owned(),
            adjust_values: std::collections::BTreeMap::new(),
        }),
        fill: None,
        outline: None,
        text_body: Some(text_body),
        effects: None,
        placeholder_type: None,
        placeholder_idx: None,
        alt_text: None,
        object_name: None,
        hidden: false,
        hyperlink: None,
    })
}

fn shape_for_placeholder(text_body: TextBody, ph_type: &str) -> SlideElement {
    let mut element = shape_with_text(text_body);
    if let SlideElement::Shape(s) = &mut element {
        s.placeholder_type = Some(ph_type.to_owned());
    }
    element
}

fn body_with_one_paragraph(props: ParagraphProperties, runs: Vec<TextRun>) -> TextBody {
    TextBody {
        default_text_color: None,
        paragraphs: vec![Paragraph {
            runs,
            properties: props,
            end_para_run_properties: None,
        }],
        body_properties: BodyProperties {
            anchor: slideglance_model::VerticalAnchor::T,
            margin_left: Emu::new(0),
            margin_right: Emu::new(0),
            margin_top: Emu::new(0),
            margin_bottom: Emu::new(0),
            wrap: WrapMode::Square,
            auto_fit: slideglance_model::AutoFit::NoAutofit,
            font_scale: 1.0,
            ln_spc_reduction: 0.0,
            num_col: 1,
            vert: slideglance_model::TextVerticalType::Horz,
            spc_first_last_para: false,
            compat_ln_spc: false,
            prst_tx_warp: None,
        },
    }
}

#[test]
fn alignment_falls_back_to_left_when_chain_empty() {
    let mut elements = vec![shape_with_text(body_with_one_paragraph(
        empty_paragraph_props(),
        Vec::new(),
    ))];
    apply_text_style_inheritance(
        &mut elements,
        &TextStyleContext {
            layout_placeholder_styles: &[],
            master_placeholder_styles: &[],
            tx_styles: None,
            default_text_style: None,
            font_scheme: None,
        },
    );
    let SlideElement::Shape(shape) = &elements[0] else {
        panic!("expected shape");
    };
    let p = &shape.text_body.as_ref().unwrap().paragraphs[0];
    assert_eq!(p.properties.alignment, Some(ParagraphAlignment::L));
}

#[test]
fn run_font_size_inherits_from_master_placeholder() {
    let mut runs = vec![TextRun {
        text: "hello".into(),
        properties: empty_run_props(),
        field_type: None,
    }];
    runs[0].properties.font_size = None;
    let mut elements = vec![shape_for_placeholder(
        body_with_one_paragraph(empty_paragraph_props(), runs),
        "title",
    )];

    let master_placeholder = PlaceholderStyleInfo {
        placeholder_type: "title".to_owned(),
        placeholder_idx: None,
        lst_style: Some(DefaultTextStyle {
            default_paragraph: Some(DefaultParagraphLevelProperties {
                alignment: None,
                margin_left: None,
                indent: None,
                bullet: None,
                bullet_font: None,
                bullet_color: None,
                bullet_size_pct: None,
                line_spacing: None,
                space_before: None,
                space_after: None,
                default_run_properties: Some(DefaultRunProperties {
                    font_size: Some(Pt(44.0)),
                    font_family: None,
                    font_family_ea: None,
                    font_family_cs: None,
                    bold: None,
                    italic: None,
                    underline: None,
                    strikethrough: None,
                    color: None,
                    highlight: None,
                }),
            }),
            levels: vec![None; 9],
        }),
        body_properties: None,
        transform: None,
        geometry: None,
    };

    apply_text_style_inheritance(
        &mut elements,
        &TextStyleContext {
            layout_placeholder_styles: &[],
            master_placeholder_styles: std::slice::from_ref(&master_placeholder),
            tx_styles: None,
            default_text_style: None,
            font_scheme: None,
        },
    );

    let SlideElement::Shape(shape) = &elements[0] else {
        panic!("expected shape");
    };
    let run = &shape.text_body.as_ref().unwrap().paragraphs[0].runs[0];
    assert_eq!(run.properties.font_size, Some(Pt(44.0)));
}

#[test]
fn layout_overrides_master_when_both_supply_color() {
    let mut runs = vec![TextRun {
        text: "x".into(),
        properties: empty_run_props(),
        field_type: None,
    }];
    runs[0].properties.color = None;
    let mut elements = vec![shape_for_placeholder(
        body_with_one_paragraph(empty_paragraph_props(), runs),
        "body",
    )];

    let make_color_style = |rgb: Rgb| DefaultTextStyle {
        default_paragraph: Some(DefaultParagraphLevelProperties {
            alignment: None,
            margin_left: None,
            indent: None,
            bullet: None,
            bullet_font: None,
            bullet_color: None,
            bullet_size_pct: None,
            line_spacing: None,
            space_before: None,
            space_after: None,
            default_run_properties: Some(DefaultRunProperties {
                font_size: None,
                font_family: None,
                font_family_ea: None,
                font_family_cs: None,
                bold: None,
                italic: None,
                underline: None,
                strikethrough: None,
                color: Some(ResolvedColor { rgb, alpha: 1.0 }),
                highlight: None,
            }),
        }),
        levels: vec![None; 9],
    };

    let layout = PlaceholderStyleInfo {
        placeholder_type: "body".to_owned(),
        placeholder_idx: None,
        lst_style: Some(make_color_style(Rgb::new(0xFF, 0, 0))),
        body_properties: None,
        transform: None,
        geometry: None,
    };
    let master = PlaceholderStyleInfo {
        placeholder_type: "body".to_owned(),
        placeholder_idx: None,
        lst_style: Some(make_color_style(Rgb::new(0, 0xFF, 0))),
        body_properties: None,
        transform: None,
        geometry: None,
    };

    apply_text_style_inheritance(
        &mut elements,
        &TextStyleContext {
            layout_placeholder_styles: std::slice::from_ref(&layout),
            master_placeholder_styles: std::slice::from_ref(&master),
            tx_styles: None,
            default_text_style: None,
            font_scheme: None,
        },
    );

    let SlideElement::Shape(shape) = &elements[0] else {
        panic!("expected shape");
    };
    let run = &shape.text_body.as_ref().unwrap().paragraphs[0].runs[0];
    let color = run.properties.color.expect("color resolved");
    assert_eq!(color.rgb, Rgb::new(0xFF, 0, 0)); // layout wins
}

#[test]
fn ctr_title_falls_back_to_title_style() {
    let mut runs = vec![TextRun {
        text: "x".into(),
        properties: empty_run_props(),
        field_type: None,
    }];
    runs[0].properties.font_size = None;
    let mut elements = vec![shape_for_placeholder(
        body_with_one_paragraph(empty_paragraph_props(), runs),
        "ctrTitle",
    )];

    let master_title = PlaceholderStyleInfo {
        placeholder_type: "title".to_owned(),
        placeholder_idx: None,
        lst_style: Some(DefaultTextStyle {
            default_paragraph: Some(DefaultParagraphLevelProperties {
                alignment: None,
                margin_left: None,
                indent: None,
                bullet: None,
                bullet_font: None,
                bullet_color: None,
                bullet_size_pct: None,
                line_spacing: None,
                space_before: None,
                space_after: None,
                default_run_properties: Some(DefaultRunProperties {
                    font_size: Some(Pt(60.0)),
                    font_family: None,
                    font_family_ea: None,
                    font_family_cs: None,
                    bold: None,
                    italic: None,
                    underline: None,
                    strikethrough: None,
                    color: None,
                    highlight: None,
                }),
            }),
            levels: vec![None; 9],
        }),
        body_properties: None,
        transform: None,
        geometry: None,
    };

    apply_text_style_inheritance(
        &mut elements,
        &TextStyleContext {
            layout_placeholder_styles: &[],
            master_placeholder_styles: std::slice::from_ref(&master_title),
            tx_styles: None,
            default_text_style: None,
            font_scheme: None,
        },
    );

    let SlideElement::Shape(shape) = &elements[0] else {
        panic!("expected shape");
    };
    let run = &shape.text_body.as_ref().unwrap().paragraphs[0].runs[0];
    assert_eq!(run.properties.font_size, Some(Pt(60.0)));
}

#[test]
fn tx_styles_supply_alignment_when_no_placeholder_match() {
    let mut elements = vec![shape_for_placeholder(
        body_with_one_paragraph(empty_paragraph_props(), Vec::new()),
        "title",
    )];

    let tx_styles = TxStyles {
        title_style: Some(DefaultTextStyle {
            default_paragraph: Some(DefaultParagraphLevelProperties {
                alignment: Some(ParagraphAlignment::Ctr),
                margin_left: None,
                indent: None,
                bullet: None,
                bullet_font: None,
                bullet_color: None,
                bullet_size_pct: None,
                line_spacing: None,
                space_before: None,
                space_after: None,
                default_run_properties: None,
            }),
            levels: vec![None; 9],
        }),
        body_style: None,
        other_style: None,
    };

    apply_text_style_inheritance(
        &mut elements,
        &TextStyleContext {
            layout_placeholder_styles: &[],
            master_placeholder_styles: &[],
            tx_styles: Some(&tx_styles),
            default_text_style: None,
            font_scheme: None,
        },
    );

    let SlideElement::Shape(shape) = &elements[0] else {
        panic!("expected shape");
    };
    let p = &shape.text_body.as_ref().unwrap().paragraphs[0];
    assert_eq!(p.properties.alignment, Some(ParagraphAlignment::Ctr));
}

#[test]
fn bullet_aux_attrs_inherited_only_when_visible_bullet() {
    let mut props = empty_paragraph_props();
    props.bullet = Some(BulletType::Char {
        char: "\u{2022}".to_owned(),
    });
    let mut elements = vec![shape_with_text(body_with_one_paragraph(props, Vec::new()))];

    let default_text_style = DefaultTextStyle {
        default_paragraph: Some(DefaultParagraphLevelProperties {
            alignment: None,
            margin_left: None,
            indent: None,
            bullet: None,
            bullet_font: Some("Arial".to_owned()),
            bullet_color: Some(ResolvedColor {
                rgb: Rgb::new(0, 0, 0),
                alpha: 1.0,
            }),
            bullet_size_pct: Some(75.0),
            line_spacing: None,
            space_before: None,
            space_after: None,
            default_run_properties: None,
        }),
        levels: vec![None; 9],
    };

    apply_text_style_inheritance(
        &mut elements,
        &TextStyleContext {
            layout_placeholder_styles: &[],
            master_placeholder_styles: &[],
            tx_styles: None,
            default_text_style: Some(&default_text_style),
            font_scheme: None,
        },
    );

    let SlideElement::Shape(shape) = &elements[0] else {
        panic!("expected shape");
    };
    let p = &shape.text_body.as_ref().unwrap().paragraphs[0];
    assert_eq!(p.properties.bullet_font.as_deref(), Some("Arial"));
    assert_eq!(p.properties.bullet_size_pct, Some(75.0));
}

#[test]
fn bullet_none_does_not_inherit_aux_attrs() {
    let mut props = empty_paragraph_props();
    props.bullet = Some(BulletType::None);
    let mut elements = vec![shape_with_text(body_with_one_paragraph(props, Vec::new()))];

    let default_text_style = DefaultTextStyle {
        default_paragraph: Some(DefaultParagraphLevelProperties {
            alignment: None,
            margin_left: None,
            indent: None,
            bullet: None,
            bullet_font: Some("Arial".to_owned()),
            bullet_color: None,
            bullet_size_pct: Some(50.0),
            line_spacing: None,
            space_before: None,
            space_after: None,
            default_run_properties: None,
        }),
        levels: vec![None; 9],
    };

    apply_text_style_inheritance(
        &mut elements,
        &TextStyleContext {
            layout_placeholder_styles: &[],
            master_placeholder_styles: &[],
            tx_styles: None,
            default_text_style: Some(&default_text_style),
            font_scheme: None,
        },
    );

    let SlideElement::Shape(shape) = &elements[0] else {
        panic!("expected shape");
    };
    let p = &shape.text_body.as_ref().unwrap().paragraphs[0];
    assert!(p.properties.bullet_font.is_none());
    assert!(p.properties.bullet_size_pct.is_none());
}

#[test]
fn group_descent_resolves_inner_shapes() {
    use slideglance_model::GroupElement;

    let mut runs = vec![TextRun {
        text: "x".into(),
        properties: empty_run_props(),
        field_type: None,
    }];
    runs[0].properties.font_size = None;
    let inner = shape_for_placeholder(
        body_with_one_paragraph(empty_paragraph_props(), runs),
        "body",
    );

    let mut elements = vec![SlideElement::Group(GroupElement {
        sp_id: None,
        transform: Transform::default(),
        child_transform: Transform::default(),
        children: vec![inner],
        effects: None,
        alt_text: None,
        object_name: None,
        hidden: false,
    })];

    let master = PlaceholderStyleInfo {
        placeholder_type: "body".to_owned(),
        placeholder_idx: None,
        lst_style: Some(DefaultTextStyle {
            default_paragraph: Some(DefaultParagraphLevelProperties {
                alignment: None,
                margin_left: None,
                indent: None,
                bullet: None,
                bullet_font: None,
                bullet_color: None,
                bullet_size_pct: None,
                line_spacing: None,
                space_before: None,
                space_after: None,
                default_run_properties: Some(DefaultRunProperties {
                    font_size: Some(Pt(18.0)),
                    font_family: None,
                    font_family_ea: None,
                    font_family_cs: None,
                    bold: None,
                    italic: None,
                    underline: None,
                    strikethrough: None,
                    color: None,
                    highlight: None,
                }),
            }),
            levels: vec![None; 9],
        }),
        body_properties: None,
        transform: None,
        geometry: None,
    };

    apply_text_style_inheritance(
        &mut elements,
        &TextStyleContext {
            layout_placeholder_styles: &[],
            master_placeholder_styles: std::slice::from_ref(&master),
            tx_styles: None,
            default_text_style: None,
            font_scheme: None,
        },
    );

    let SlideElement::Group(g) = &elements[0] else {
        panic!("expected group");
    };
    let SlideElement::Shape(inner) = &g.children[0] else {
        panic!("expected inner shape");
    };
    let run = &inner.text_body.as_ref().unwrap().paragraphs[0].runs[0];
    assert_eq!(run.properties.font_size, Some(Pt(18.0)));
}
