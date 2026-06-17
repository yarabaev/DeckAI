//! Ported from.

use std::collections::HashMap;
use std::io::{Cursor, Write};

use slideglance_parser::PptxArchive;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

fn build_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let buf = Cursor::new(Vec::<u8>::new());
    let mut writer = ZipWriter::new(buf);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, data) in entries {
        writer.start_file(*name, opts).expect("start file");
        writer.write_all(data).expect("write entry");
    }
    let cursor = writer.finish().expect("finish zip");
    cursor.into_inner()
}

#[test]
fn reads_xml_files_and_provides_lazy_media_access() {
    let xml_content = b"<p:presentation/>";
    let image_data = [0x89_u8, 0x50, 0x4E, 0x47];
    let zip = build_zip(&[
        ("ppt/presentation.xml", xml_content),
        (
            "ppt/_rels/presentation.xml.rels",
            b"<Relationships/>" as &[u8],
        ),
        ("ppt/media/image1.png", &image_data),
    ]);

    let mut archive = PptxArchive::open(zip).expect("open");

    assert_eq!(
        archive.xml("ppt/presentation.xml"),
        Some("<p:presentation/>"),
    );
    assert_eq!(
        archive.xml("ppt/_rels/presentation.xml.rels"),
        Some("<Relationships/>"),
    );
    assert_eq!(
        archive.media("ppt/media/image1.png").unwrap(),
        Some(&image_data[..]),
    );
}

#[test]
fn does_not_eagerly_decompress_media_files() {
    let image_data = vec![0xAB_u8; 1024];
    let zip = build_zip(&[
        ("ppt/presentation.xml", b"<xml/>" as &[u8]),
        ("ppt/media/large-image.png", &image_data),
    ]);

    let mut archive = PptxArchive::open(zip).expect("open");

    assert!(!archive
        .xml_files()
        .contains_key("ppt/media/large-image.png"));
    assert!(archive.media_paths().contains("ppt/media/large-image.png"));
    assert_eq!(
        archive.media("ppt/media/large-image.png").unwrap(),
        Some(image_data.as_slice()),
    );
}

#[test]
fn returns_none_for_non_existent_media() {
    let zip = build_zip(&[("ppt/presentation.xml", b"<xml/>" as &[u8])]);
    let mut archive = PptxArchive::open(zip).expect("open");
    assert!(archive
        .media("ppt/media/nonexistent.png")
        .unwrap()
        .is_none());
}

#[test]
fn caches_media_for_repeated_access() {
    let image_data = [0xFF_u8, 0xD8, 0xFF, 0xE0];
    let zip = build_zip(&[("ppt/media/photo.jpg", &image_data)]);
    let mut archive = PptxArchive::open(zip).expect("open");

    let first_ptr = archive
        .media("ppt/media/photo.jpg")
        .unwrap()
        .unwrap()
        .as_ptr();
    let second_ptr = archive
        .media("ppt/media/photo.jpg")
        .unwrap()
        .unwrap()
        .as_ptr();
    assert_eq!(
        first_ptr, second_ptr,
        "second access should return cached buffer (same pointer)",
    );
}

#[test]
fn includes_content_types_xml() {
    let zip = build_zip(&[
        ("[Content_Types].xml", b"<Types/>" as &[u8]),
        ("ppt/presentation.xml", b"<xml/>" as &[u8]),
    ]);
    let archive = PptxArchive::open(zip).expect("open");
    assert_eq!(archive.xml("[Content_Types].xml"), Some("<Types/>"));
}

#[test]
fn skips_non_xml_non_rels_non_media_entries() {
    let zip = build_zip(&[
        ("ppt/presentation.xml", b"<xml/>" as &[u8]),
        ("docProps/thumbnail.jpeg", b"\xff\xd8\xff" as &[u8]),
    ]);
    let archive = PptxArchive::open(zip).expect("open");
    let map: &HashMap<String, String> = archive.xml_files();
    assert_eq!(map.len(), 1);
    assert!(map.contains_key("ppt/presentation.xml"));
    assert!(!map.contains_key("docProps/thumbnail.jpeg"));
    // The non-XML, non-media entry is also not exposed via media paths.
    assert!(!archive.media_paths().contains("docProps/thumbnail.jpeg"));
}

#[test]
fn rejects_non_zip_input() {
    let result = PptxArchive::open(b"not a zip".to_vec());
    assert!(result.is_err(), "non-zip input should be rejected");
}
