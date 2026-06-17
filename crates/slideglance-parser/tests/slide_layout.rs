//! Slide-layout parser integration tests. Mirrors representative cases
//! from.

use std::io::{Cursor, Write};

use slideglance_color::{ColorMap, ColorResolver, ColorScheme, Rgb};
use slideglance_parser::{parse_slide_layout, PptxArchive};
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

const LAYOUT_PATH: &str = "ppt/slideLayouts/slideLayout1.xml";

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
    ColorResolver::new(scheme, ColorMap::default())
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

fn parse(xml: &str, archive: &mut PptxArchive) -> slideglance_model::SlideLayout {
    let resolver = test_resolver();
    parse_slide_layout(xml, LAYOUT_PATH, archive, &resolver, None, None).expect("parse layout")
}

#[test]
fn show_master_sp_defaults_to_true_when_attr_absent() {
    let xml = r#"<?xml version="1.0"?>
<p:sldLayout xmlns:p="urn:p"/>"#;
    let mut archive = build_test_archive(&[(LAYOUT_PATH, xml.as_bytes())]);
    let layout = parse(xml, &mut archive);
    assert!(layout.show_master_sp);
}

#[test]
fn show_master_sp_false_attr_disables_master_shapes() {
    let xml = r#"<?xml version="1.0"?>
<p:sldLayout xmlns:p="urn:p" showMasterSp="0"/>"#;
    let mut archive = build_test_archive(&[(LAYOUT_PATH, xml.as_bytes())]);
    let layout = parse(xml, &mut archive);
    assert!(!layout.show_master_sp);
}

#[test]
fn layout_name_extracted_from_csld() {
    let xml = r#"<?xml version="1.0"?>
<p:sldLayout xmlns:p="urn:p">
  <p:cSld name="Title and Content"><p:spTree/></p:cSld>
</p:sldLayout>"#;
    let mut archive = build_test_archive(&[(LAYOUT_PATH, xml.as_bytes())]);
    let layout = parse(xml, &mut archive);
    assert_eq!(layout.name.as_deref(), Some("Title and Content"));
}

#[test]
fn layout_placeholder_styles_collected_from_sp_tree() {
    let xml = r#"<?xml version="1.0"?>
<p:sldLayout xmlns:p="urn:p" xmlns:a="urn:a">
  <p:cSld>
    <p:spTree>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr name="body-ph"/>
          <p:nvPr><p:ph type="body" idx="1"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm>
            <a:off x="500" y="600"/>
            <a:ext cx="1000" cy="200"/>
          </a:xfrm>
        </p:spPr>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle>
            <a:lvl1pPr><a:defRPr sz="1800"/></a:lvl1pPr>
          </a:lstStyle>
          <a:p/>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sldLayout>"#;
    let mut archive = build_test_archive(&[(LAYOUT_PATH, xml.as_bytes())]);
    let layout = parse(xml, &mut archive);
    assert_eq!(layout.placeholder_styles.len(), 1);
    let ph = &layout.placeholder_styles[0];
    assert_eq!(ph.placeholder_type, "body");
    assert_eq!(ph.placeholder_idx, Some(1));
    assert!(
        ph.lst_style.is_some(),
        "lst_style should be lifted from txBody"
    );
    let t = ph.transform.expect("transform present");
    assert_eq!(t.offset_x.raw(), 500);
}

#[test]
fn layout_elements_share_slide_walker() {
    let xml = r#"<?xml version="1.0"?>
<p:sldLayout xmlns:p="urn:p" xmlns:a="urn:a">
  <p:cSld>
    <p:spTree>
      <p:sp>
        <p:nvSpPr><p:cNvPr name="LAYOUT_SHAPE"/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="10" cy="10"/></a:xfrm></p:spPr>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sldLayout>"#;
    let mut archive = build_test_archive(&[(LAYOUT_PATH, xml.as_bytes())]);
    let layout = parse(xml, &mut archive);
    assert_eq!(layout.elements.len(), 1);
}

#[test]
fn layout_solid_fill_background() {
    let xml = r#"<?xml version="1.0"?>
<p:sldLayout xmlns:p="urn:p" xmlns:a="urn:a">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:solidFill><a:srgbClr val="0000FF"/></a:solidFill>
      </p:bgPr>
    </p:bg>
    <p:spTree/>
  </p:cSld>
</p:sldLayout>"#;
    let mut archive = build_test_archive(&[(LAYOUT_PATH, xml.as_bytes())]);
    let layout = parse(xml, &mut archive);
    let bg = layout.background.expect("bg present");
    let fill = bg.fill.expect("fill present");
    match fill {
        slideglance_model::Fill::Solid(s) => assert_eq!(s.color.rgb, Rgb::new(0, 0, 0xFF)),
        _ => panic!("expected solid fill"),
    }
}
