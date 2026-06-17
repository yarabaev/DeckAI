//! Example: measure text width for Korean slide titles.
use slideglance_font::text_measure::measure_text_width;
fn main() {
    let font_size_pt = 21.67_f64;
    let with_spaces = "평 가 항 목  조 견 표";
    let no_spaces = "평가항목 조견표";
    let collapsed = "평가항목조견표";
    for (label, t) in [
        ("with_spaces", with_spaces),
        ("no_spaces", no_spaces),
        ("collapsed", collapsed),
    ] {
        let w = measure_text_width(
            t,
            font_size_pt,
            false,
            Some("나눔스퀘어"),
            Some("나눔스퀘어"),
        );
        println!("{label}: '{t}' → width = {w:.2}");
    }
}
