//! Slide-parser integration tests. Mirrors representative cases from
//! .

use std::io::{Cursor, Write};

use slideglance_color::{ColorMap, ColorResolver, ColorScheme, Rgb, SchemeColorKey};
use slideglance_model::{Geometry, PlaceholderStyleInfo, PresetGeometry, SlideElement, Transform};
use slideglance_parser::{parse_slide, PptxArchive};
use slideglance_utils::Emu;
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

const SLIDE_PATH: &str = "ppt/slides/slide1.xml";

fn parse(slide_xml: &str, archive: &mut PptxArchive) -> slideglance_model::Slide {
    let resolver = test_resolver();
    parse_slide(
        slide_xml,
        SLIDE_PATH,
        1,
        archive,
        &resolver,
        None,
        None,
        &[],
    )
    .expect("parse slide")
}

fn parse_with_placeholders(
    slide_xml: &str,
    archive: &mut PptxArchive,
    placeholder_styles: &[PlaceholderStyleInfo],
) -> slideglance_model::Slide {
    let resolver = test_resolver();
    parse_slide(
        slide_xml,
        SLIDE_PATH,
        1,
        archive,
        &resolver,
        None,
        None,
        placeholder_styles,
    )
    .expect("parse slide")
}

#[test]
fn parses_minimal_slide() {
    let xml = r#"<?xml version="1.0"?>
<p:sld xmlns:p="urn:p" xmlns:a="urn:a">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr/>
      <p:grpSpPr/>
    </p:spTree>
  </p:cSld>
</p:sld>"#;
    let mut archive = build_test_archive(&[(SLIDE_PATH, xml.as_bytes())]);
    let slide = parse(xml, &mut archive);
    assert_eq!(slide.slide_number, 1);
    assert!(slide.elements.is_empty());
    assert!(slide.background.is_none());
    assert!(slide.show_master_sp);
    assert!(slide.header_footer.is_none());
}

#[test]
fn show_master_sp_false_attribute_disables_master_shapes() {
    let xml = r#"<?xml version="1.0"?>
<p:sld xmlns:p="urn:p" showMasterSp="0">
  <p:cSld><p:spTree/></p:cSld>
</p:sld>"#;
    let mut archive = build_test_archive(&[(SLIDE_PATH, xml.as_bytes())]);
    let slide = parse(xml, &mut archive);
    assert!(!slide.show_master_sp);
}

#[test]
fn shape_tree_preserves_source_order_and_types() {
    let xml = r#"<?xml version="1.0"?>
<p:sld xmlns:p="urn:p" xmlns:a="urn:a">
  <p:cSld>
    <p:spTree>
      <p:sp>
        <p:nvSpPr><p:cNvPr name="A"/><p:cNvSpPr/><p:nvSpPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100" cy="100"/></a:xfrm></p:spPr>
      </p:sp>
      <p:cxnSp>
        <p:nvCxnSpPr><p:cNvPr name="B"/></p:nvCxnSpPr>
        <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100" cy="100"/></a:xfrm></p:spPr>
      </p:cxnSp>
      <p:sp>
        <p:nvSpPr><p:cNvPr name="C"/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100" cy="100"/></a:xfrm></p:spPr>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;
    let mut archive = build_test_archive(&[(SLIDE_PATH, xml.as_bytes())]);
    let slide = parse(xml, &mut archive);
    assert_eq!(slide.elements.len(), 3);
    let ordered: Vec<(&str, Option<&str>)> = slide
        .elements
        .iter()
        .map(|el| match el {
            SlideElement::Shape(s) => ("shape", s.object_name.as_deref()),
            SlideElement::Connector(c) => ("connector", c.object_name.as_deref()),
            _ => ("other", None),
        })
        .collect();
    assert_eq!(
        ordered,
        vec![
            ("shape", Some("A")),
            ("connector", Some("B")),
            ("shape", Some("C")),
        ]
    );
}

#[test]
fn placeholder_falls_back_to_master_transform_and_geometry() {
    let xml = r#"<?xml version="1.0"?>
<p:sld xmlns:p="urn:p" xmlns:a="urn:a">
  <p:cSld>
    <p:spTree>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr name="title"/>
          <p:nvPr><p:ph type="title"/></p:nvPr>
        </p:nvSpPr>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;
    let mut archive = build_test_archive(&[(SLIDE_PATH, xml.as_bytes())]);
    let placeholder_styles = vec![PlaceholderStyleInfo {
        placeholder_type: "title".to_owned(),
        placeholder_idx: None,
        lst_style: None,
        body_properties: None,
        transform: Some(Transform {
            offset_x: Emu::new(100),
            offset_y: Emu::new(200),
            extent_width: Emu::new(300),
            extent_height: Emu::new(400),
            rotation: 0.0,
            flip_h: false,
            flip_v: false,
        }),
        geometry: Some(Geometry::Preset(PresetGeometry {
            preset: "rect".to_owned(),
            adjust_values: std::collections::BTreeMap::new(),
        })),
    }];
    let slide = parse_with_placeholders(xml, &mut archive, &placeholder_styles);
    assert_eq!(slide.elements.len(), 1);
    match &slide.elements[0] {
        SlideElement::Shape(sh) => {
            assert_eq!(sh.transform.offset_x.raw(), 100);
            assert_eq!(sh.transform.offset_y.raw(), 200);
            assert_eq!(sh.transform.extent_width.raw(), 300);
            assert_eq!(sh.placeholder_type.as_deref(), Some("title"));
        }
        _ => panic!("expected shape"),
    }
}

#[test]
fn placeholder_ctr_title_falls_back_to_title() {
    let xml = r#"<?xml version="1.0"?>
<p:sld xmlns:p="urn:p" xmlns:a="urn:a">
  <p:cSld>
    <p:spTree>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr name="ctr"/>
          <p:nvPr><p:ph type="ctrTitle"/></p:nvPr>
        </p:nvSpPr>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;
    let mut archive = build_test_archive(&[(SLIDE_PATH, xml.as_bytes())]);
    let placeholder_styles = vec![PlaceholderStyleInfo {
        placeholder_type: "title".to_owned(),
        placeholder_idx: None,
        lst_style: None,
        body_properties: None,
        transform: Some(Transform {
            offset_x: Emu::new(7),
            offset_y: Emu::new(8),
            extent_width: Emu::new(9),
            extent_height: Emu::new(10),
            rotation: 0.0,
            flip_h: false,
            flip_v: false,
        }),
        geometry: None,
    }];
    let slide = parse_with_placeholders(xml, &mut archive, &placeholder_styles);
    match &slide.elements[0] {
        SlideElement::Shape(sh) => {
            assert_eq!(sh.transform.offset_x.raw(), 7);
            assert_eq!(sh.placeholder_type.as_deref(), Some("ctrTitle"));
        }
        _ => panic!("expected shape"),
    }
}

#[test]
fn group_recurses_into_children() {
    let xml = r#"<?xml version="1.0"?>
<p:sld xmlns:p="urn:p" xmlns:a="urn:a">
  <p:cSld>
    <p:spTree>
      <p:grpSp>
        <p:nvGrpSpPr><p:cNvPr name="GROUP"/></p:nvGrpSpPr>
        <p:grpSpPr>
          <a:xfrm>
            <a:off x="10" y="20"/>
            <a:ext cx="1000" cy="2000"/>
            <a:chOff x="5" y="6"/>
            <a:chExt cx="1100" cy="2200"/>
          </a:xfrm>
        </p:grpSpPr>
        <p:sp>
          <p:nvSpPr><p:cNvPr name="CHILD"/></p:nvSpPr>
          <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100" cy="100"/></a:xfrm></p:spPr>
        </p:sp>
      </p:grpSp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;
    let mut archive = build_test_archive(&[(SLIDE_PATH, xml.as_bytes())]);
    let slide = parse(xml, &mut archive);
    assert_eq!(slide.elements.len(), 1);
    match &slide.elements[0] {
        SlideElement::Group(g) => {
            assert_eq!(g.object_name.as_deref(), Some("GROUP"));
            assert_eq!(g.transform.offset_x.raw(), 10);
            assert_eq!(g.transform.extent_width.raw(), 1000);
            assert_eq!(g.child_transform.offset_x.raw(), 5);
            assert_eq!(g.child_transform.extent_width.raw(), 1100);
            assert_eq!(g.children.len(), 1);
            match &g.children[0] {
                SlideElement::Shape(sh) => {
                    assert_eq!(sh.object_name.as_deref(), Some("CHILD"));
                }
                _ => panic!("expected nested shape"),
            }
        }
        _ => panic!("expected group"),
    }
}

#[test]
fn alternate_content_uses_first_choice_in_source_order() {
    let xml = r#"<?xml version="1.0"?>
<p:sld xmlns:p="urn:p" xmlns:a="urn:a" xmlns:mc="urn:mc">
  <p:cSld>
    <p:spTree>
      <p:sp>
        <p:nvSpPr><p:cNvPr name="BEFORE"/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="10" cy="10"/></a:xfrm></p:spPr>
      </p:sp>
      <mc:AlternateContent>
        <mc:Choice Requires="a14">
          <p:sp>
            <p:nvSpPr><p:cNvPr name="ALT"/></p:nvSpPr>
            <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="10" cy="10"/></a:xfrm></p:spPr>
          </p:sp>
        </mc:Choice>
        <mc:Fallback/>
      </mc:AlternateContent>
      <p:sp>
        <p:nvSpPr><p:cNvPr name="AFTER"/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="10" cy="10"/></a:xfrm></p:spPr>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;
    let mut archive = build_test_archive(&[(SLIDE_PATH, xml.as_bytes())]);
    let slide = parse(xml, &mut archive);
    assert_eq!(slide.elements.len(), 3);
    let names: Vec<_> = slide
        .elements
        .iter()
        .filter_map(|el| match el {
            SlideElement::Shape(s) => s.object_name.as_deref(),
            _ => None,
        })
        .collect();
    assert_eq!(names, vec!["BEFORE", "ALT", "AFTER"]);
}

#[test]
fn header_footer_defaults_to_true_when_attrs_absent() {
    let xml = r#"<?xml version="1.0"?>
<p:sld xmlns:p="urn:p">
  <p:cSld><p:spTree/></p:cSld>
  <p:hf/>
</p:sld>"#;
    let mut archive = build_test_archive(&[(SLIDE_PATH, xml.as_bytes())]);
    let slide = parse(xml, &mut archive);
    let hf = slide.header_footer.expect("hf present");
    assert!(hf.show_slide_number);
    assert!(hf.show_date_time);
    assert!(hf.show_footer);
}

#[test]
fn header_footer_explicit_false_attrs_are_respected() {
    let xml = r#"<?xml version="1.0"?>
<p:sld xmlns:p="urn:p">
  <p:cSld><p:spTree/></p:cSld>
  <p:hf sldNum="0" dt="false" ftr="0"/>
</p:sld>"#;
    let mut archive = build_test_archive(&[(SLIDE_PATH, xml.as_bytes())]);
    let slide = parse(xml, &mut archive);
    let hf = slide.header_footer.expect("hf present");
    assert!(!hf.show_slide_number);
    assert!(!hf.show_date_time);
    assert!(!hf.show_footer);
}

#[test]
fn shape_with_no_xfrm_and_no_placeholder_is_skipped() {
    let xml = r#"<?xml version="1.0"?>
<p:sld xmlns:p="urn:p" xmlns:a="urn:a">
  <p:cSld>
    <p:spTree>
      <p:sp>
        <p:nvSpPr><p:cNvPr name="orphan"/></p:nvSpPr>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;
    let mut archive = build_test_archive(&[(SLIDE_PATH, xml.as_bytes())]);
    let slide = parse(xml, &mut archive);
    assert!(slide.elements.is_empty());
}

#[test]
fn solid_fill_background_resolved_through_resolver() {
    let xml = r#"<?xml version="1.0"?>
<p:sld xmlns:p="urn:p" xmlns:a="urn:a">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:solidFill><a:srgbClr val="FF0000"/></a:solidFill>
      </p:bgPr>
    </p:bg>
    <p:spTree/>
  </p:cSld>
</p:sld>"#;
    let mut archive = build_test_archive(&[(SLIDE_PATH, xml.as_bytes())]);
    let slide = parse(xml, &mut archive);
    let bg = slide.background.expect("background present");
    let fill = bg.fill.expect("fill present");
    match fill {
        slideglance_model::Fill::Solid(s) => assert_eq!(s.color.rgb, Rgb::new(0xFF, 0, 0)),
        _ => panic!("expected solid fill"),
    }
}

#[test]
fn image_loads_media_and_emits_base64() {
    let png = b"\x89PNG\r\n\x1a\nfake";
    let slide_xml = r#"<?xml version="1.0"?>
<p:sld xmlns:p="urn:p" xmlns:a="urn:a" xmlns:r="urn:r">
  <p:cSld>
    <p:spTree>
      <p:pic>
        <p:nvPicPr><p:cNvPr name="picture"/></p:nvPicPr>
        <p:blipFill>
          <a:blip r:embed="rId1"/>
          <a:srcRect l="0" t="0" r="0" b="0"/>
        </p:blipFill>
        <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="500" cy="500"/></a:xfrm></p:spPr>
      </p:pic>
    </p:spTree>
  </p:cSld>
</p:sld>"#;
    let rels_xml = r#"<?xml version="1.0"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/>
</Relationships>"#;
    let mut archive = build_test_archive(&[
        (SLIDE_PATH, slide_xml.as_bytes()),
        ("ppt/slides/_rels/slide1.xml.rels", rels_xml.as_bytes()),
        ("ppt/media/image1.png", png),
    ]);
    let slide = parse(slide_xml, &mut archive);
    assert_eq!(slide.elements.len(), 1);
    match &slide.elements[0] {
        SlideElement::Image(img) => {
            assert_eq!(img.mime_type, "image/png");
            assert_eq!(img.object_name.as_deref(), Some("picture"));
            // Base64 of the 14 byte PNG header — non-empty and decodes back.
            assert!(!img.image_data.is_empty());
            assert_eq!(img.transform.extent_width.raw(), 500);
        }
        _ => panic!("expected image"),
    }
}

#[test]
fn nested_groups_recurse_arbitrarily_deep() {
    let xml = r#"<?xml version="1.0"?>
<p:sld xmlns:p="urn:p" xmlns:a="urn:a">
  <p:cSld>
    <p:spTree>
      <p:grpSp>
        <p:nvGrpSpPr><p:cNvPr name="OUTER"/></p:nvGrpSpPr>
        <p:grpSpPr>
          <a:xfrm>
            <a:off x="0" y="0"/><a:ext cx="100" cy="100"/>
            <a:chOff x="0" y="0"/><a:chExt cx="100" cy="100"/>
          </a:xfrm>
        </p:grpSpPr>
        <p:grpSp>
          <p:nvGrpSpPr><p:cNvPr name="INNER"/></p:nvGrpSpPr>
          <p:grpSpPr>
            <a:xfrm>
              <a:off x="0" y="0"/><a:ext cx="50" cy="50"/>
              <a:chOff x="0" y="0"/><a:chExt cx="50" cy="50"/>
            </a:xfrm>
          </p:grpSpPr>
          <p:sp>
            <p:nvSpPr><p:cNvPr name="LEAF"/></p:nvSpPr>
            <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="10" cy="10"/></a:xfrm></p:spPr>
          </p:sp>
        </p:grpSp>
      </p:grpSp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;
    let mut archive = build_test_archive(&[(SLIDE_PATH, xml.as_bytes())]);
    let slide = parse(xml, &mut archive);
    let SlideElement::Group(outer) = &slide.elements[0] else {
        panic!("expected outer group");
    };
    let SlideElement::Group(inner) = &outer.children[0] else {
        panic!("expected inner group");
    };
    match &inner.children[0] {
        SlideElement::Shape(s) => assert_eq!(s.object_name.as_deref(), Some("LEAF")),
        _ => panic!("expected leaf shape"),
    }
}
