//! Adhoc EMF+ raster extractor — `cargo run -p slideglance-emf --example extract <path>`
fn main() {
    let path = std::env::args().nth(1).expect("usage: extract <emf>");
    let data = std::fs::read(&path).expect("read");
    eprintln!("input bytes: {}", data.len());
    match slideglance_emf::extract_raster(&data) {
        Some(png) => {
            let out = std::env::args()
                .nth(2)
                .unwrap_or_else(|| "/tmp/emfp_out.png".into());
            eprintln!("got PNG: {} bytes -> {}", png.len(), out);
            std::fs::write(&out, &png).expect("write");
        }
        None => eprintln!("extract_raster returned None"),
    }
}
