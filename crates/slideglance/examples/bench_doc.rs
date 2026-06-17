//! Benchmark the on-demand `PptxDocument` API against the eager
//! `convert_to_svg` baseline.
//!
//! Usage: `cargo run --release -p slideglance --example bench_doc <path/to.pptx>`
//!
//! Reports parse time, first-slide render time, and total per-slide
//! render time + the median media count emitted under `external_media`.

use std::time::Instant;

use slideglance::{PptxDocument, SlideRenderOptions};

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: bench_doc <path/to.pptx>");
    let bytes = std::fs::read(&path).expect("read pptx");

    let t0 = Instant::now();
    let doc = PptxDocument::parse(bytes, &[], &[], true).expect("parse");
    let parse_ms = t0.elapsed().as_millis();
    let count = doc.slide_count();
    println!("parse: {parse_ms} ms, {count} slides");

    let opts = SlideRenderOptions {
        external_media: true,
        include_font_defs: false,
        ..SlideRenderOptions::default()
    };

    let t1 = Instant::now();
    let first = doc
        .render_slide(1, &opts)
        .expect("render")
        .expect("slide 1 exists");
    let first_ms = t1.elapsed().as_millis();
    println!(
        "first slide (external_media): {first_ms} ms, svg={} bytes, media={} blobs",
        first.svg.len(),
        first.media.len(),
    );

    let t2 = Instant::now();
    let mut total_svg = 0usize;
    let mut total_blobs = 0usize;
    for n in 1..=count {
        let r = doc.render_slide(n, &opts).expect("render").expect("slide");
        total_svg += r.svg.len();
        total_blobs += r.media.len();
    }
    let all_ms = t2.elapsed().as_millis();
    println!(
 "all {count} slides (external_media): {all_ms} ms, total svg={total_svg} bytes, total blob refs={total_blobs}"
 );
}
