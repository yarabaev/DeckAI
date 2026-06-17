//! Batch SVG directory -> PNG directory converter for VRT comparison.
//!
//! Usage: `cargo run --release -p slideglance-png --example dir_to_png -- <in_dir> <out_dir> [width]`

use slideglance_png::{svg_to_png, FontData, PngOptions};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Walk every TTF / OTF / TTC under the macOS system font directories and
/// return the byte buffers. Pure example/comparison helper — the library
/// itself never touches the host file system (deterministic by design).
fn load_macos_system_fonts() -> Vec<FontData> {
    let dirs = [
        "/System/Library/Fonts",
        "/Library/Fonts",
        &format!("{}/Library/Fonts", env::var("HOME").unwrap_or_default()),
    ];
    let mut out = Vec::new();
    for dir in dirs {
        walk(Path::new(dir), &mut out);
    }
    out
}

fn walk(dir: &Path, out: &mut Vec<FontData>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            let ext_lc = ext.to_ascii_lowercase();
            if matches!(ext_lc.as_str(), "ttf" | "otf" | "ttc") {
                if let Ok(bytes) = fs::read(&path) {
                    out.push(FontData::new(bytes));
                }
            }
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: dir_to_png <in_dir> <out_dir> [width]");
        return ExitCode::from(2);
    }
    let in_dir = PathBuf::from(&args[1]);
    let out_dir = PathBuf::from(&args[2]);
    let width: Option<u32> = args.get(3).and_then(|s| s.parse().ok());
    fs::create_dir_all(&out_dir).expect("create out_dir");

    let fonts = load_macos_system_fonts();
    eprintln!("loaded {} system fonts", fonts.len());
    let opts = PngOptions {
        width,
        fonts,
        ..Default::default()
    };

    let mut entries: Vec<PathBuf> = fs::read_dir(&in_dir)
        .expect("read in_dir")
        .filter_map(std::result::Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "svg"))
        .collect();
    entries.sort();

    let mut ok = 0usize;
    let mut fail = 0usize;
    for entry in entries {
        let svg = match fs::read_to_string(&entry) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("read {}: {e}", entry.display());
                fail += 1;
                continue;
            }
        };
        let stem = entry.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
        let out_path = out_dir.join(format!("{stem}.png"));
        match svg_to_png(&svg, &opts) {
            Ok(o) => {
                if let Err(e) = fs::write(&out_path, &o.png) {
                    eprintln!("write {}: {e}", out_path.display());
                    fail += 1;
                } else {
                    ok += 1;
                }
            }
            Err(e) => {
                eprintln!("rasterize {}: {e}", entry.display());
                fail += 1;
            }
        }
    }
    println!("ok={ok} fail={fail}");
    ExitCode::SUCCESS
}
