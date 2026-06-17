//! Ported from
//! (color / font scheme — fmt scheme is intentionally unimplemented in this
//! sub-phase, see Phase 3-3 follow-ups in plan.md).

use slideglance_color::Rgb;
use slideglance_model::{DashStyle, Fill};
use slideglance_parser::parse_theme;

#[test]
fn returns_default_color_scheme_when_root_missing() {
    let theme = parse_theme("<other/>").unwrap();
    assert_eq!(theme.color_scheme.dk1, Rgb::new(0x00, 0x00, 0x00));
    assert_eq!(theme.color_scheme.lt1, Rgb::new(0xFF, 0xFF, 0xFF));
    assert_eq!(theme.color_scheme.accent1, Rgb::new(0x44, 0x72, 0xC4));
    assert_eq!(theme.font_scheme.major_font, "Calibri");
    assert!(theme.fmt_scheme.is_none());
}

#[test]
fn parses_explicit_srgb_color_scheme() {
    let xml = r#"
        <a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:themeElements>
                <a:clrScheme>
                    <a:dk1><a:srgbClr val="111111"/></a:dk1>
                    <a:lt1><a:srgbClr val="EEEEEE"/></a:lt1>
                    <a:dk2><a:srgbClr val="222222"/></a:dk2>
                    <a:lt2><a:srgbClr val="DDDDDD"/></a:lt2>
                    <a:accent1><a:srgbClr val="FF0000"/></a:accent1>
                    <a:accent2><a:srgbClr val="00FF00"/></a:accent2>
                    <a:accent3><a:srgbClr val="0000FF"/></a:accent3>
                    <a:accent4><a:srgbClr val="FFFF00"/></a:accent4>
                    <a:accent5><a:srgbClr val="00FFFF"/></a:accent5>
                    <a:accent6><a:srgbClr val="FF00FF"/></a:accent6>
                    <a:hlink><a:srgbClr val="123456"/></a:hlink>
                    <a:folHlink><a:srgbClr val="654321"/></a:folHlink>
                </a:clrScheme>
            </a:themeElements>
        </a:theme>
    "#;
    let theme = parse_theme(xml).unwrap();
    assert_eq!(theme.color_scheme.dk1, Rgb::new(0x11, 0x11, 0x11));
    assert_eq!(theme.color_scheme.accent1, Rgb::new(0xFF, 0x00, 0x00));
    assert_eq!(theme.color_scheme.fol_hlink, Rgb::new(0x65, 0x43, 0x21));
}

#[test]
fn extracts_sys_clr_last_value() {
    let xml = r#"
        <a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:themeElements>
                <a:clrScheme>
                    <a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1>
                    <a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1>
                </a:clrScheme>
            </a:themeElements>
        </a:theme>
    "#;
    let theme = parse_theme(xml).unwrap();
    assert_eq!(theme.color_scheme.dk1, Rgb::new(0, 0, 0));
    assert_eq!(theme.color_scheme.lt1, Rgb::new(0xFF, 0xFF, 0xFF));
}

#[test]
fn parses_font_scheme_with_latin_ea_cs() {
    let xml = r#"
        <a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:themeElements>
                <a:fontScheme name="Office">
                    <a:majorFont>
                        <a:latin typeface="Calibri Light"/>
                        <a:ea typeface="Yu Gothic Light"/>
                        <a:cs typeface="Times New Roman"/>
                    </a:majorFont>
                    <a:minorFont>
                        <a:latin typeface="Calibri"/>
                        <a:ea typeface="Yu Gothic"/>
                        <a:cs typeface="Arial"/>
                    </a:minorFont>
                </a:fontScheme>
            </a:themeElements>
        </a:theme>
    "#;
    let theme = parse_theme(xml).unwrap();
    assert_eq!(theme.font_scheme.major_font, "Calibri Light");
    assert_eq!(theme.font_scheme.minor_font, "Calibri");
    assert_eq!(
        theme.font_scheme.major_font_ea.as_deref(),
        Some("Yu Gothic Light")
    );
    assert_eq!(
        theme.font_scheme.minor_font_ea.as_deref(),
        Some("Yu Gothic")
    );
    assert_eq!(
        theme.font_scheme.major_font_cs.as_deref(),
        Some("Times New Roman"),
    );
}

#[test]
fn falls_back_to_jpan_script_when_ea_empty() {
    let xml = r#"
        <a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:themeElements>
                <a:fontScheme>
                    <a:majorFont>
                        <a:latin typeface="Calibri"/>
                        <a:ea typeface=""/>
                        <a:font script="Jpan" typeface="Yu Mincho Demibold"/>
                    </a:majorFont>
                    <a:minorFont>
                        <a:latin typeface="Calibri"/>
                    </a:minorFont>
                </a:fontScheme>
            </a:themeElements>
        </a:theme>
    "#;
    let theme = parse_theme(xml).unwrap();
    assert_eq!(
        theme.font_scheme.major_font_ea.as_deref(),
        Some("Yu Mincho Demibold"),
    );
    assert_eq!(
        theme.font_scheme.major_script_font("Jpan"),
        Some("Yu Mincho Demibold"),
    );
}

// --- CJK Script Equality (project rule, see CLAUDE.md) ---
//
// The spec (the spec) only extracts script="Jpan", silently
// dropping Hang / Hans / Hant entries. The Rust port treats every CJK script
// equally: all four codes round-trip through `*_script_fonts`.

#[test]
fn collects_korean_hang_script_font() {
    let xml = r#"
        <a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:themeElements>
                <a:fontScheme>
                    <a:majorFont>
                        <a:latin typeface="Calibri"/>
                        <a:font script="Hang" typeface="맑은 고딕"/>
                    </a:majorFont>
                    <a:minorFont>
                        <a:latin typeface="Calibri"/>
                        <a:font script="Hang" typeface="맑은 고딕"/>
                    </a:minorFont>
                </a:fontScheme>
            </a:themeElements>
        </a:theme>
    "#;
    let theme = parse_theme(xml).unwrap();
    assert_eq!(
        theme.font_scheme.major_script_font("Hang"),
        Some("맑은 고딕")
    );
    assert_eq!(
        theme.font_scheme.minor_script_font("Hang"),
        Some("맑은 고딕")
    );
}

#[test]
fn collects_simplified_chinese_hans_script_font() {
    let xml = r#"
        <a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:themeElements>
                <a:fontScheme>
                    <a:majorFont>
                        <a:latin typeface="Calibri"/>
                        <a:font script="Hans" typeface="宋体"/>
                    </a:majorFont>
                    <a:minorFont>
                        <a:latin typeface="Calibri"/>
                        <a:font script="Hans" typeface="宋体"/>
                    </a:minorFont>
                </a:fontScheme>
            </a:themeElements>
        </a:theme>
    "#;
    let theme = parse_theme(xml).unwrap();
    assert_eq!(theme.font_scheme.major_script_font("Hans"), Some("宋体"));
    assert_eq!(theme.font_scheme.minor_script_font("Hans"), Some("宋体"));
}

#[test]
fn collects_traditional_chinese_hant_script_font() {
    let xml = r#"
        <a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:themeElements>
                <a:fontScheme>
                    <a:majorFont>
                        <a:latin typeface="Calibri"/>
                        <a:font script="Hant" typeface="新細明體"/>
                    </a:majorFont>
                    <a:minorFont>
                        <a:latin typeface="Calibri"/>
                        <a:font script="Hant" typeface="新細明體"/>
                    </a:minorFont>
                </a:fontScheme>
            </a:themeElements>
        </a:theme>
    "#;
    let theme = parse_theme(xml).unwrap();
    assert_eq!(
        theme.font_scheme.major_script_font("Hant"),
        Some("新細明體")
    );
    assert_eq!(
        theme.font_scheme.minor_script_font("Hant"),
        Some("新細明體")
    );
}

#[test]
fn collects_all_cjk_scripts_simultaneously() {
    // PowerPoint themes commonly carry every CJK script; the parser must
    // expose every one of them, not just the first or just Japanese.
    let xml = r#"
        <a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:themeElements>
                <a:fontScheme>
                    <a:majorFont>
                        <a:latin typeface="Calibri Light"/>
                        <a:font script="Jpan" typeface="Yu Gothic Light"/>
                        <a:font script="Hang" typeface="맑은 고딕"/>
                        <a:font script="Hans" typeface="等线 Light"/>
                        <a:font script="Hant" typeface="新細明體"/>
                    </a:majorFont>
                    <a:minorFont>
                        <a:latin typeface="Calibri"/>
                        <a:font script="Jpan" typeface="Yu Gothic"/>
                        <a:font script="Hang" typeface="맑은 고딕"/>
                        <a:font script="Hans" typeface="等线"/>
                        <a:font script="Hant" typeface="新細明體"/>
                    </a:minorFont>
                </a:fontScheme>
            </a:themeElements>
        </a:theme>
    "#;
    let theme = parse_theme(xml).unwrap();
    let major = &theme.font_scheme.major_script_fonts;
    assert_eq!(
        major.get("Jpan").map(String::as_str),
        Some("Yu Gothic Light")
    );
    assert_eq!(major.get("Hang").map(String::as_str), Some("맑은 고딕"));
    assert_eq!(major.get("Hans").map(String::as_str), Some("等线 Light"));
    assert_eq!(major.get("Hant").map(String::as_str), Some("新細明體"));
    assert_eq!(major.len(), 4, "every CJK script must round-trip");

    let minor = &theme.font_scheme.minor_script_fonts;
    assert_eq!(minor.get("Jpan").map(String::as_str), Some("Yu Gothic"));
    assert_eq!(minor.get("Hang").map(String::as_str), Some("맑은 고딕"));
    assert_eq!(minor.get("Hans").map(String::as_str), Some("等线"));
    assert_eq!(minor.get("Hant").map(String::as_str), Some("新細明體"));
}

#[test]
fn ea_falls_back_to_korean_when_only_hang_present() {
    // CJK equality: the ea fallback is no longer Japanese-only. When a
    // theme has no <a:ea> but has a Korean script font, that becomes the EA
    // typeface. Mirrors the equivalent Japanese-only test above.
    let xml = r#"
        <a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:themeElements>
                <a:fontScheme>
                    <a:majorFont>
                        <a:latin typeface="Calibri"/>
                        <a:ea typeface=""/>
                        <a:font script="Hang" typeface="맑은 고딕"/>
                    </a:majorFont>
                    <a:minorFont>
                        <a:latin typeface="Calibri"/>
                    </a:minorFont>
                </a:fontScheme>
            </a:themeElements>
        </a:theme>
    "#;
    let theme = parse_theme(xml).unwrap();
    assert_eq!(
        theme.font_scheme.major_font_ea.as_deref(),
        Some("맑은 고딕")
    );
    assert_eq!(
        theme.font_scheme.major_script_font("Hang"),
        Some("맑은 고딕"),
    );
}

// --- fmtScheme ---

#[test]
fn fmt_scheme_is_none_when_block_absent() {
    let xml = r#"
        <a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:themeElements>
                <a:clrScheme><a:dk1><a:srgbClr val="000000"/></a:dk1></a:clrScheme>
            </a:themeElements>
        </a:theme>
    "#;
    let theme = parse_theme(xml).unwrap();
    assert!(theme.fmt_scheme.is_none());
}

#[test]
fn parses_fill_style_lst_in_order() {
    let xml = r#"
        <a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:themeElements>
                <a:clrScheme>
                    <a:dk1><a:srgbClr val="000000"/></a:dk1>
                    <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
                    <a:dk2><a:srgbClr val="44546A"/></a:dk2>
                    <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
                    <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
                    <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
                    <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
                    <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
                    <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
                    <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
                    <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
                    <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
                </a:clrScheme>
                <a:fontScheme/>
                <a:fmtScheme>
                    <a:fillStyleLst>
                        <a:solidFill><a:schemeClr val="accent1"/></a:solidFill>
                        <a:gradFill>
                            <a:gsLst>
                                <a:gs pos="0"><a:srgbClr val="FF0000"/></a:gs>
                                <a:gs pos="100000"><a:srgbClr val="0000FF"/></a:gs>
                            </a:gsLst>
                            <a:lin ang="0"/>
                        </a:gradFill>
                        <a:solidFill><a:srgbClr val="00FF00"/></a:solidFill>
                    </a:fillStyleLst>
                </a:fmtScheme>
            </a:themeElements>
        </a:theme>
    "#;
    let theme = parse_theme(xml).unwrap();
    let fmt = theme.fmt_scheme.unwrap();
    assert_eq!(fmt.fill_styles.len(), 3, "source order must be preserved");
    match &fmt.fill_styles[0] {
        Fill::Solid(s) => assert_eq!(s.color.rgb, Rgb::new(0x44, 0x72, 0xC4)),
        other => panic!("expected accent1 solid first, got {other:?}"),
    }
    match &fmt.fill_styles[1] {
        Fill::Gradient(_) => {}
        other => panic!("expected gradient second, got {other:?}"),
    }
    match &fmt.fill_styles[2] {
        Fill::Solid(s) => assert_eq!(s.color.rgb, Rgb::new(0, 0xFF, 0)),
        other => panic!("expected green solid third, got {other:?}"),
    }
}

#[test]
fn parses_ln_style_lst_with_outlines() {
    let xml = r#"
        <a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:themeElements>
                <a:clrScheme/>
                <a:fontScheme/>
                <a:fmtScheme>
                    <a:lnStyleLst>
                        <a:ln w="6350" cap="flat">
                            <a:solidFill><a:srgbClr val="000000"/></a:solidFill>
                        </a:ln>
                        <a:ln w="12700">
                            <a:solidFill><a:srgbClr val="FF0000"/></a:solidFill>
                            <a:prstDash val="dash"/>
                        </a:ln>
                    </a:lnStyleLst>
                </a:fmtScheme>
            </a:themeElements>
        </a:theme>
    "#;
    let theme = parse_theme(xml).unwrap();
    let fmt = theme.fmt_scheme.unwrap();
    assert_eq!(fmt.ln_styles.len(), 2);
    assert_eq!(fmt.ln_styles[0].width.raw(), 6_350);
    assert_eq!(fmt.ln_styles[1].width.raw(), 12_700);
    assert!(matches!(fmt.ln_styles[1].dash_style, DashStyle::Dash));
}

#[test]
fn parses_effect_style_lst_with_some_empty_entries() {
    // Mirrors the typical Office theme: 3 effectStyle entries, the first two
    // empty (no shadows / glows) and the third with an outerShadow.
    let xml = r#"
        <a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:themeElements>
                <a:clrScheme/>
                <a:fontScheme/>
                <a:fmtScheme>
                    <a:effectStyleLst>
                        <a:effectStyle><a:effectLst/></a:effectStyle>
                        <a:effectStyle><a:effectLst/></a:effectStyle>
                        <a:effectStyle>
                            <a:effectLst>
                                <a:outerShdw blurRad="50800" dist="38100">
                                    <a:srgbClr val="000000"/>
                                </a:outerShdw>
                            </a:effectLst>
                        </a:effectStyle>
                    </a:effectStyleLst>
                </a:fmtScheme>
            </a:themeElements>
        </a:theme>
    "#;
    let theme = parse_theme(xml).unwrap();
    let fmt = theme.fmt_scheme.unwrap();
    assert_eq!(fmt.effect_styles.len(), 3);
    assert!(fmt.effect_styles[0].is_none());
    assert!(fmt.effect_styles[1].is_none());
    let third = fmt.effect_styles[2].as_ref().unwrap();
    assert!(third.outer_shadow.is_some());
}

#[test]
fn parses_bg_fill_style_lst() {
    let xml = r#"
        <a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:themeElements>
                <a:clrScheme>
                    <a:dk1><a:srgbClr val="000000"/></a:dk1>
                    <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
                </a:clrScheme>
                <a:fontScheme/>
                <a:fmtScheme>
                    <a:bgFillStyleLst>
                        <a:solidFill><a:schemeClr val="bg1"/></a:solidFill>
                        <a:noFill/>
                    </a:bgFillStyleLst>
                </a:fmtScheme>
            </a:themeElements>
        </a:theme>
    "#;
    let theme = parse_theme(xml).unwrap();
    let fmt = theme.fmt_scheme.unwrap();
    assert_eq!(fmt.bg_fill_styles.len(), 2);
    match &fmt.bg_fill_styles[0] {
        // bg1 maps to lt1 in the default ColorMap → white.
        Fill::Solid(s) => assert_eq!(s.color.rgb, Rgb::new(0xFF, 0xFF, 0xFF)),
        other => panic!("expected solid white from bg1, got {other:?}"),
    }
    assert!(matches!(fmt.bg_fill_styles[1], Fill::None(_)));
}

#[test]
fn empty_fmt_scheme_returns_none() {
    let xml = r#"
        <a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:themeElements>
                <a:clrScheme/>
                <a:fontScheme/>
                <a:fmtScheme/>
            </a:themeElements>
        </a:theme>
    "#;
    let theme = parse_theme(xml).unwrap();
    assert!(
        theme.fmt_scheme.is_none(),
        "empty fmtScheme block should collapse to None"
    );
}

#[test]
fn defaults_font_scheme_when_missing() {
    let xml = r#"
        <a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:themeElements/>
        </a:theme>
    "#;
    let theme = parse_theme(xml).unwrap();
    assert_eq!(theme.font_scheme.major_font, "Calibri");
    assert_eq!(theme.font_scheme.minor_font, "Calibri");
    assert!(theme.font_scheme.major_font_ea.is_none());
    assert!(theme.font_scheme.major_script_fonts.is_empty());
    assert!(theme.font_scheme.minor_script_fonts.is_empty());
}
