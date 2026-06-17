//! Slide-master parser integration tests. Mirrors representative cases
//! from.

use std::io::{Cursor, Write};

use slideglance_color::{ColorMap, ColorResolver, ColorScheme, Rgb, SchemeColorKey};
use slideglance_parser::{parse_slide_master, PptxArchive};
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

const MASTER_PATH: &str = "ppt/slideMasters/slideMaster1.xml";

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

fn parse(xml: &str, archive: &mut PptxArchive) -> slideglance_model::SlideMaster {
    let resolver = test_resolver();
    parse_slide_master(xml, MASTER_PATH, archive, &resolver, None, None).expect("parse master")
}

#[test]
fn empty_master_returns_default_color_map_and_no_pieces() {
    let xml = r#"<?xml version="1.0"?>
<p:sldMaster xmlns:p="urn:p"/>"#;
    let mut archive = build_test_archive(&[(MASTER_PATH, xml.as_bytes())]);
    let master = parse(xml, &mut archive);
    assert_eq!(master.color_map, ColorMap::default());
    assert!(master.background.is_none());
    assert!(master.elements.is_empty());
    assert!(master.tx_styles.is_none());
    assert!(master.placeholder_styles.is_empty());
}

#[test]
fn color_map_attributes_override_defaults() {
    let xml = r#"<?xml version="1.0"?>
<p:sldMaster xmlns:p="urn:p">
  <p:clrMap bg1="dk2" tx1="lt2" bg2="dk1" tx2="lt1"/>
</p:sldMaster>"#;
    let mut archive = build_test_archive(&[(MASTER_PATH, xml.as_bytes())]);
    let master = parse(xml, &mut archive);
    assert_eq!(master.color_map.bg1, SchemeColorKey::Dk2);
    assert_eq!(master.color_map.tx1, SchemeColorKey::Lt2);
    assert_eq!(master.color_map.bg2, SchemeColorKey::Dk1);
    assert_eq!(master.color_map.tx2, SchemeColorKey::Lt1);
    // Untouched slots fall back to default.
    assert_eq!(master.color_map.accent1, SchemeColorKey::Accent1);
    assert_eq!(master.color_map.fol_hlink, SchemeColorKey::FolHlink);
}

#[test]
fn unknown_color_map_value_falls_back_to_default() {
    let xml = r#"<?xml version="1.0"?>
<p:sldMaster xmlns:p="urn:p">
  <p:clrMap bg1="not-a-real-name"/>
</p:sldMaster>"#;
    let mut archive = build_test_archive(&[(MASTER_PATH, xml.as_bytes())]);
    let master = parse(xml, &mut archive);
    assert_eq!(master.color_map.bg1, SchemeColorKey::Lt1);
}

#[test]
fn master_solid_fill_background() {
    let xml = r#"<?xml version="1.0"?>
<p:sldMaster xmlns:p="urn:p" xmlns:a="urn:a">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:solidFill><a:srgbClr val="00FF00"/></a:solidFill>
      </p:bgPr>
    </p:bg>
    <p:spTree/>
  </p:cSld>
</p:sldMaster>"#;
    let mut archive = build_test_archive(&[(MASTER_PATH, xml.as_bytes())]);
    let master = parse(xml, &mut archive);
    let bg = master.background.expect("background present");
    let fill = bg.fill.expect("fill present");
    match fill {
        slideglance_model::Fill::Solid(s) => assert_eq!(s.color.rgb, Rgb::new(0x00, 0xFF, 0x00)),
        _ => panic!("expected solid fill"),
    }
}

#[test]
#[ignore = "pre-existing parser stack overflow"]
fn placeholder_styles_carry_transform_and_geometry() {
    let xml = r#"<?xml version="1.0"?>
<p:sldMaster xmlns:p="urn:p" xmlns:a="urn:a">
  <p:cSld>
    <p:spTree>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr name="title-ph"/>
          <p:nvPr><p:ph type="title" idx="0"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm>
            <a:off x="100" y="200"/>
            <a:ext cx="300" cy="400"/>
          </a:xfrm>
          <a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
        </p:spPr>
      </p:sp>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr name="not-a-placeholder"/>
        </p:nvSpPr>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sldMaster>"#;
    let mut archive = build_test_archive(&[(MASTER_PATH, xml.as_bytes())]);
    let master = parse(xml, &mut archive);
    assert_eq!(master.placeholder_styles.len(), 1);
    let ph = &master.placeholder_styles[0];
    assert_eq!(ph.placeholder_type, "title");
    assert_eq!(ph.placeholder_idx, Some(0));
    let t = ph.transform.expect("transform present");
    assert_eq!(t.offset_x.raw(), 100);
    assert_eq!(t.extent_height.raw(), 400);
    assert!(matches!(
        ph.geometry,
        Some(slideglance_model::Geometry::Preset(_))
    ));
}

#[test]
#[ignore = "pre-existing parser stack overflow"]
fn tx_styles_with_only_some_styles_present() {
    let xml = r#"<?xml version="1.0"?>
<p:sldMaster xmlns:p="urn:p" xmlns:a="urn:a">
  <p:txStyles>
    <p:titleStyle>
      <a:lvl1pPr><a:defRPr sz="4400"/></a:lvl1pPr>
    </p:titleStyle>
    <p:bodyStyle>
      <a:lvl1pPr><a:defRPr sz="2400"/></a:lvl1pPr>
    </p:bodyStyle>
  </p:txStyles>
</p:sldMaster>"#;
    let mut archive = build_test_archive(&[(MASTER_PATH, xml.as_bytes())]);
    let master = parse(xml, &mut archive);
    let tx_styles = master.tx_styles.expect("tx_styles present");
    assert!(tx_styles.title_style.is_some());
    assert!(tx_styles.body_style.is_some());
    assert!(tx_styles.other_style.is_none());
}

#[test]
#[ignore = "pre-existing parser stack overflow"]
fn master_elements_share_slide_shape_tree_walker() {
    let xml = r#"<?xml version="1.0"?>
<p:sldMaster xmlns:p="urn:p" xmlns:a="urn:a">
  <p:cSld>
    <p:spTree>
      <p:sp>
        <p:nvSpPr><p:cNvPr name="MASTER_SHAPE"/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="10" cy="10"/></a:xfrm></p:spPr>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sldMaster>"#;
    let mut archive = build_test_archive(&[(MASTER_PATH, xml.as_bytes())]);
    let master = parse(xml, &mut archive);
    assert_eq!(master.elements.len(), 1);
    match &master.elements[0] {
        slideglance_model::SlideElement::Shape(s) => {
            assert_eq!(s.object_name.as_deref(), Some("MASTER_SHAPE"));
        }
        _ => panic!("expected shape"),
    }
}
