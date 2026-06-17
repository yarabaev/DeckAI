//! Ported from
//! (representative cases — full TS file has 30+ assertions).

use std::collections::BTreeMap;
use std::io::{Cursor, Write};

use slideglance_color::{ColorMap, ColorResolver, ColorScheme, Rgb, SchemeColorKey};
use slideglance_model::{
    ArrowSize, ArrowType, DashStyle, Fill, GradientType, ImageFlip, LineCap, LineJoin, OutlineFill,
};
use slideglance_parser::{parse_fill, parse_outline, FillParseContext, PptxArchive, Relationship};
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

fn test_resolver() -> ColorResolver {
    let scheme = ColorScheme {
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
    };
    let map = ColorMap {
        bg1: SchemeColorKey::Lt1,
        tx1: SchemeColorKey::Dk1,
        bg2: SchemeColorKey::Lt2,
        tx2: SchemeColorKey::Dk2,
        accent1: SchemeColorKey::Accent1,
        accent2: SchemeColorKey::Accent2,
        accent3: SchemeColorKey::Accent3,
        accent4: SchemeColorKey::Accent4,
        accent5: SchemeColorKey::Accent5,
        accent6: SchemeColorKey::Accent6,
        hlink: SchemeColorKey::Hlink,
        fol_hlink: SchemeColorKey::FolHlink,
    };
    ColorResolver::new(scheme, map)
}

fn build_test_archive(entries: &[(&str, &[u8])]) -> PptxArchive {
    let buf = Cursor::new(Vec::<u8>::new());
    let mut writer = ZipWriter::new(buf);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, data) in entries {
        writer.start_file(*name, opts).unwrap();
        writer.write_all(data).unwrap();
    }
    let bytes = writer.finish().unwrap().into_inner();
    PptxArchive::open(bytes).unwrap()
}

// --- noFill ---

#[test]
fn parses_no_fill() {
    let result = parse_fill("<root><a:noFill/></root>", &test_resolver(), None)
        .unwrap()
        .unwrap();
    assert!(matches!(result, Fill::None(_)));
}

// --- solidFill ---

#[test]
fn parses_solid_fill_with_srgb() {
    let xml = r#"<root xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:solidFill><a:srgbClr val="FF0000"/></a:solidFill>
    </root>"#;
    let result = parse_fill(xml, &test_resolver(), None).unwrap().unwrap();
    match result {
        Fill::Solid(s) => {
            assert_eq!(s.color.rgb, Rgb::new(0xFF, 0, 0));
            assert_eq!(s.color.alpha, 1.0);
        }
        _ => panic!("expected solid fill, got {result:?}"),
    }
}

#[test]
fn parses_solid_fill_with_scheme_color() {
    let xml = r#"<root><a:solidFill><a:schemeClr val="accent1"/></a:solidFill></root>"#;
    let result = parse_fill(xml, &test_resolver(), None).unwrap().unwrap();
    match result {
        Fill::Solid(s) => assert_eq!(s.color.rgb, Rgb::new(0x44, 0x72, 0xC4)),
        _ => panic!("expected solid fill"),
    }
}

#[test]
fn parses_solid_fill_with_alpha_transform() {
    let xml = r#"<root><a:solidFill>
        <a:srgbClr val="FF0000"><a:alpha val="50000"/></a:srgbClr>
    </a:solidFill></root>"#;
    let result = parse_fill(xml, &test_resolver(), None).unwrap().unwrap();
    match result {
        Fill::Solid(s) => assert!((s.color.alpha - 0.5).abs() < 1e-9),
        _ => panic!("expected solid fill"),
    }
}

// --- gradFill ---

#[test]
fn parses_linear_gradient_fill() {
    let xml = r#"<root><a:gradFill>
        <a:gsLst>
            <a:gs pos="0"><a:srgbClr val="FF0000"/></a:gs>
            <a:gs pos="100000"><a:srgbClr val="0000FF"/></a:gs>
        </a:gsLst>
        <a:lin ang="5400000"/>
    </a:gradFill></root>"#;
    let result = parse_fill(xml, &test_resolver(), None).unwrap().unwrap();
    match result {
        Fill::Gradient(g) => {
            assert!(matches!(g.gradient_type, GradientType::Linear));
            assert!((g.angle - 90.0).abs() < 1e-9);
            assert_eq!(g.stops.len(), 2);
            assert_eq!(g.stops[0].color.rgb, Rgb::new(0xFF, 0, 0));
            assert!((g.stops[0].position - 0.0).abs() < 1e-9);
            assert_eq!(g.stops[1].color.rgb, Rgb::new(0, 0, 0xFF));
            assert!((g.stops[1].position - 1.0).abs() < 1e-9);
        }
        _ => panic!("expected gradient fill"),
    }
}

#[test]
fn parses_radial_gradient_with_path_centered() {
    let xml = r#"<root><a:gradFill>
        <a:gsLst>
            <a:gs pos="0"><a:srgbClr val="FFFFFF"/></a:gs>
            <a:gs pos="100000"><a:srgbClr val="000000"/></a:gs>
        </a:gsLst>
        <a:path path="circle"/>
    </a:gradFill></root>"#;
    let result = parse_fill(xml, &test_resolver(), None).unwrap().unwrap();
    match result {
        Fill::Gradient(g) => {
            assert!(matches!(g.gradient_type, GradientType::Radial));
            assert!((g.center_x.unwrap() - 0.5).abs() < 1e-9);
            assert!((g.center_y.unwrap() - 0.5).abs() < 1e-9);
        }
        _ => panic!("expected gradient fill"),
    }
}

#[test]
fn parses_radial_gradient_with_explicit_fill_to_rect() {
    let xml = r#"<root><a:gradFill>
        <a:gsLst>
            <a:gs pos="0"><a:srgbClr val="FFFFFF"/></a:gs>
            <a:gs pos="100000"><a:srgbClr val="000000"/></a:gs>
        </a:gsLst>
        <a:path path="circle">
            <a:fillToRect l="20000" t="30000" r="20000" b="30000"/>
        </a:path>
    </a:gradFill></root>"#;
    let result = parse_fill(xml, &test_resolver(), None).unwrap().unwrap();
    match result {
        Fill::Gradient(g) => {
            // (l + (100000 - r)) / 2 / 100000 = (20000 + 80000) / 2 / 100000 = 0.5
            assert!((g.center_x.unwrap() - 0.5).abs() < 1e-9);
        }
        _ => panic!("expected gradient fill"),
    }
}

// --- pattFill ---

#[test]
fn parses_pattern_fill() {
    let xml = r#"<root><a:pattFill prst="pct50">
        <a:fgClr><a:srgbClr val="000000"/></a:fgClr>
        <a:bgClr><a:srgbClr val="FFFFFF"/></a:bgClr>
    </a:pattFill></root>"#;
    let result = parse_fill(xml, &test_resolver(), None).unwrap().unwrap();
    match result {
        Fill::Pattern(p) => {
            assert_eq!(p.preset, "pct50");
            assert_eq!(p.foreground_color.rgb, Rgb::new(0, 0, 0));
            assert_eq!(p.background_color.rgb, Rgb::new(0xFF, 0xFF, 0xFF));
        }
        _ => panic!("expected pattern fill"),
    }
}

#[test]
fn pattern_fill_with_no_colors_returns_none() {
    let xml = r#"<root><a:pattFill prst="pct50"/></root>"#;
    let result = parse_fill(xml, &test_resolver(), None).unwrap();
    assert!(result.is_none());
}

// --- blipFill ---

#[test]
fn parses_blip_fill_with_archive_media() {
    let png_bytes = b"\x89PNG\r\n\x1a\nfake-image-data";
    let mut archive = build_test_archive(&[("ppt/media/image1.png", png_bytes)]);

    let mut rels = BTreeMap::new();
    rels.insert(
        "rId7".to_owned(),
        Relationship {
            id: "rId7".to_owned(),
            ty: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
                .to_owned(),
            target: "../media/image1.png".to_owned(),
            target_mode: None,
        },
    );

    let xml = r#"<root><a:blipFill>
        <a:blip r:embed="rId7"/>
        <a:stretch><a:fillRect/></a:stretch>
    </a:blipFill></root>"#;

    let mut ctx = FillParseContext {
        rels: &rels,
        archive: &mut archive,
        base_path: "ppt/slides/slide1.xml",
        group_fill: None,
    };

    let result = parse_fill(xml, &test_resolver(), Some(&mut ctx))
        .unwrap()
        .unwrap();
    match result {
        Fill::Image(img) => {
            assert_eq!(img.mime_type, "image/png");
            assert!(
                !img.image_data.is_empty(),
                "image_data should be base64-encoded"
            );
            let stretch = img.stretch.unwrap();
            assert_eq!(stretch.left, 0.0);
            assert_eq!(stretch.right, 0.0);
        }
        _ => panic!("expected image fill, got {result:?}"),
    }
}

#[test]
fn blip_fill_without_context_returns_none() {
    let xml = r#"<root><a:blipFill><a:blip r:embed="rId7"/></a:blipFill></root>"#;
    let result = parse_fill(xml, &test_resolver(), None).unwrap();
    assert!(result.is_none(), "blipFill without ctx must produce None");
}

#[test]
fn blip_fill_with_unknown_rid_returns_none() {
    let mut archive = build_test_archive(&[("ppt/media/image1.png", b"data")]);
    let rels: BTreeMap<String, Relationship> = BTreeMap::new();
    let mut ctx = FillParseContext {
        rels: &rels,
        archive: &mut archive,
        base_path: "ppt/slides/slide1.xml",
        group_fill: None,
    };
    let xml = r#"<root><a:blipFill><a:blip r:embed="rIdMissing"/></a:blipFill></root>"#;
    let result = parse_fill(xml, &test_resolver(), Some(&mut ctx)).unwrap();
    assert!(result.is_none());
}

#[test]
fn parses_blip_fill_with_tile() {
    let png_bytes = b"\x89PNGdata";
    let mut archive = build_test_archive(&[("ppt/media/image1.png", png_bytes)]);
    let mut rels = BTreeMap::new();
    rels.insert(
        "rId1".to_owned(),
        Relationship {
            id: "rId1".to_owned(),
            ty: "image".to_owned(),
            target: "../media/image1.png".to_owned(),
            target_mode: None,
        },
    );
    let xml = r#"<root><a:blipFill>
        <a:blip r:embed="rId1"/>
        <a:tile tx="100" ty="200" sx="50000" sy="50000" flip="xy" algn="ctr"/>
    </a:blipFill></root>"#;
    let mut ctx = FillParseContext {
        rels: &rels,
        archive: &mut archive,
        base_path: "ppt/slides/slide1.xml",
        group_fill: None,
    };
    let result = parse_fill(xml, &test_resolver(), Some(&mut ctx))
        .unwrap()
        .unwrap();
    match result {
        Fill::Image(img) => {
            let tile = img.tile.unwrap();
            assert_eq!(tile.tx.raw(), 100);
            assert_eq!(tile.ty.raw(), 200);
            assert!((tile.sx - 0.5).abs() < 1e-9);
            assert!(matches!(tile.flip, ImageFlip::Xy));
            assert_eq!(tile.align, "ctr");
        }
        _ => panic!("expected image fill"),
    }
}

#[test]
fn parses_blip_fill_with_src_rect() {
    let mut archive = build_test_archive(&[("ppt/media/image1.png", b"data")]);
    let mut rels = BTreeMap::new();
    rels.insert(
        "rId1".to_owned(),
        Relationship {
            id: "rId1".to_owned(),
            ty: "image".to_owned(),
            target: "../media/image1.png".to_owned(),
            target_mode: None,
        },
    );
    let xml = r#"<root><a:blipFill>
        <a:blip r:embed="rId1"/>
        <a:srcRect l="5000" t="10000" r="5000" b="10000"/>
    </a:blipFill></root>"#;
    let mut ctx = FillParseContext {
        rels: &rels,
        archive: &mut archive,
        base_path: "ppt/slides/slide1.xml",
        group_fill: None,
    };
    let result = parse_fill(xml, &test_resolver(), Some(&mut ctx))
        .unwrap()
        .unwrap();
    match result {
        Fill::Image(img) => {
            let src = img.src_rect.unwrap();
            assert!((src.left - 0.05).abs() < 1e-9);
            assert!((src.top - 0.10).abs() < 1e-9);
        }
        _ => panic!("expected image fill"),
    }
}

#[test]
fn parses_blip_fill_with_alpha_mod_fix() {
    let mut archive = build_test_archive(&[("ppt/media/image1.png", b"data")]);
    let mut rels = BTreeMap::new();
    rels.insert(
        "rId1".to_owned(),
        Relationship {
            id: "rId1".to_owned(),
            ty: "image".to_owned(),
            target: "../media/image1.png".to_owned(),
            target_mode: None,
        },
    );
    // amt=5000 ⇒ ST_PositiveFixedPercentage = 5% (0.05).
    let xml = r#"<root><a:blipFill>
        <a:blip r:embed="rId1"><a:alphaModFix amt="5000"/></a:blip>
    </a:blipFill></root>"#;
    let mut ctx = FillParseContext {
        rels: &rels,
        archive: &mut archive,
        base_path: "ppt/slides/slide1.xml",
        group_fill: None,
    };
    let result = parse_fill(xml, &test_resolver(), Some(&mut ctx))
        .unwrap()
        .unwrap();
    match result {
        Fill::Image(img) => {
            assert!(
                (img.alpha - 0.05).abs() < 1e-9,
                "expected alpha=0.05, got {}",
                img.alpha
            );
        }
        _ => panic!("expected image fill"),
    }
}

#[test]
fn blip_fill_without_alpha_mod_fix_defaults_to_opaque() {
    let mut archive = build_test_archive(&[("ppt/media/image1.png", b"data")]);
    let mut rels = BTreeMap::new();
    rels.insert(
        "rId1".to_owned(),
        Relationship {
            id: "rId1".to_owned(),
            ty: "image".to_owned(),
            target: "../media/image1.png".to_owned(),
            target_mode: None,
        },
    );
    let xml = r#"<root><a:blipFill>
        <a:blip r:embed="rId1"/>
    </a:blipFill></root>"#;
    let mut ctx = FillParseContext {
        rels: &rels,
        archive: &mut archive,
        base_path: "ppt/slides/slide1.xml",
        group_fill: None,
    };
    let result = parse_fill(xml, &test_resolver(), Some(&mut ctx))
        .unwrap()
        .unwrap();
    match result {
        Fill::Image(img) => {
            assert!(
                (img.alpha - 1.0).abs() < 1e-9,
                "expected default alpha=1.0, got {}",
                img.alpha
            );
        }
        _ => panic!("expected image fill"),
    }
}

#[test]
fn blip_fill_alpha_mod_fix_without_amt_defaults_to_opaque() {
    let mut archive = build_test_archive(&[("ppt/media/image1.png", b"data")]);
    let mut rels = BTreeMap::new();
    rels.insert(
        "rId1".to_owned(),
        Relationship {
            id: "rId1".to_owned(),
            ty: "image".to_owned(),
            target: "../media/image1.png".to_owned(),
            target_mode: None,
        },
    );
    // `<a:alphaModFix/>` without `amt` is the OOXML default (no change → 100%).
    let xml = r#"<root><a:blipFill>
        <a:blip r:embed="rId1"><a:alphaModFix/></a:blip>
    </a:blipFill></root>"#;
    let mut ctx = FillParseContext {
        rels: &rels,
        archive: &mut archive,
        base_path: "ppt/slides/slide1.xml",
        group_fill: None,
    };
    let result = parse_fill(xml, &test_resolver(), Some(&mut ctx))
        .unwrap()
        .unwrap();
    match result {
        Fill::Image(img) => {
            assert!(
                (img.alpha - 1.0).abs() < 1e-9,
                "expected default alpha=1.0, got {}",
                img.alpha
            );
        }
        _ => panic!("expected image fill"),
    }
}

// --- grpFill ---

#[test]
fn grp_fill_inherits_group_fill_when_present() {
    use slideglance_color::ResolvedColor;
    use slideglance_model::{NoFill, SolidFill};

    let group_fill = Fill::Solid(SolidFill {
        color: ResolvedColor::opaque(Rgb::new(0xAB, 0xCD, 0xEF)),
    });
    let mut archive = build_test_archive(&[("ppt/media/x.png", b"d")]);
    let rels: BTreeMap<String, Relationship> = BTreeMap::new();
    let mut ctx = FillParseContext {
        rels: &rels,
        archive: &mut archive,
        base_path: "ppt/slides/slide1.xml",
        group_fill: Some(&group_fill),
    };
    let xml = "<root><a:grpFill/></root>";
    let result = parse_fill(xml, &test_resolver(), Some(&mut ctx))
        .unwrap()
        .unwrap();
    match result {
        Fill::Solid(s) => assert_eq!(s.color.rgb, Rgb::new(0xAB, 0xCD, 0xEF)),
        other => panic!("expected solid fill (inherited), got {other:?}"),
    }
    let _ = NoFill {}; // satisfy unused-import check
}

#[test]
fn grp_fill_without_group_returns_none() {
    let xml = "<root><a:grpFill/></root>";
    let result = parse_fill(xml, &test_resolver(), None).unwrap();
    assert!(result.is_none());
}

// --- outline ---

#[test]
fn parses_outline_with_solid_fill_and_dash() {
    let xml = r#"<a:ln w="25400" cap="rnd" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:solidFill><a:srgbClr val="FF0000"/></a:solidFill>
        <a:prstDash val="dash"/>
        <a:miter/>
        <a:headEnd type="triangle" w="med" len="lg"/>
    </a:ln>"#;
    let outline = parse_outline(xml, &test_resolver()).unwrap().unwrap();
    assert_eq!(outline.width.raw(), 25_400);
    assert!(matches!(outline.line_cap, Some(LineCap::Round)));
    assert!(matches!(outline.line_join, Some(LineJoin::Miter)));
    assert!(matches!(outline.dash_style, DashStyle::Dash));
    match outline.fill.unwrap() {
        OutlineFill::Solid(s) => assert_eq!(s.color.rgb, Rgb::new(0xFF, 0, 0)),
        OutlineFill::Gradient(_) => panic!("expected solid outline fill"),
    }
    let head = outline.head_end.unwrap();
    assert!(matches!(head.ty, ArrowType::Triangle));
    assert!(matches!(head.length, ArrowSize::Lg));
}

#[test]
fn outline_default_width_is_12700_emu() {
    let xml = r#"<a:ln xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"/>"#;
    let outline = parse_outline(xml, &test_resolver()).unwrap().unwrap();
    assert_eq!(outline.width.raw(), 12_700);
    assert!(matches!(outline.dash_style, DashStyle::Solid));
    assert!(outline.fill.is_none());
}

#[test]
fn outline_with_no_fill_returns_none() {
    let xml = r#"<a:ln xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:noFill/>
    </a:ln>"#;
    let outline = parse_outline(xml, &test_resolver()).unwrap();
    assert!(outline.is_none());
}

#[test]
fn outline_custom_dash() {
    let xml = r#"<a:ln xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:custDash>
            <a:ds d="200000" sp="100000"/>
            <a:ds d="50000" sp="50000"/>
        </a:custDash>
    </a:ln>"#;
    let outline = parse_outline(xml, &test_resolver()).unwrap().unwrap();
    let dash = outline.custom_dash.unwrap();
    // pairs flattened: [d, sp, d, sp]
    assert_eq!(dash.len(), 4);
    assert!((dash[0] - 2.0).abs() < 1e-9);
    assert!((dash[1] - 1.0).abs() < 1e-9);
    assert!((dash[2] - 0.5).abs() < 1e-9);
    assert!((dash[3] - 0.5).abs() < 1e-9);
}

#[test]
fn outline_arrow_type_none_yields_no_arrow() {
    let xml = r#"<a:ln xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:headEnd type="none"/>
    </a:ln>"#;
    let outline = parse_outline(xml, &test_resolver()).unwrap().unwrap();
    assert!(outline.head_end.is_none());
}
