//! Perceptual diff between two directories of PNGs.
//!
//! For each `slide-N.png` in `ours_dir` and matching `slide-NNN.png`
//! (zero-padded) in `ref_dir`, computes a normalized RMSE on a downscaled
//! 64x36 grayscale signature and prints a sorted list (worst first).
//!
//! Usage: `cargo run --release -p slideglance-png --example perceptual_diff -- <ref_dir> <ours_dir>`

use image::ImageReader;
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

const W: u32 = 64;
const H: u32 = 36;

fn signature(path: &PathBuf) -> Option<Vec<u8>> {
    let img = ImageReader::open(path).ok()?.decode().ok()?;
    let small = img
        .resize_exact(W, H, image::imageops::FilterType::Triangle)
        .to_luma8();
    Some(small.into_raw())
}

fn rmse(a: &[u8], b: &[u8]) -> f64 {
    let n = a.len().min(b.len());
    if n == 0 {
        return f64::INFINITY;
    }
    let mut acc = 0.0_f64;
    for i in 0..n {
        let d = f64::from(a[i]) - f64::from(b[i]);
        acc += d * d;
    }
    (acc / n as f64).sqrt() / 255.0
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: perceptual_diff <ref_dir> <ours_dir>");
        return ExitCode::from(2);
    }
    let ref_dir = PathBuf::from(&args[1]);
    let ours_dir = PathBuf::from(&args[2]);

    let mut results: Vec<(u32, f64)> = Vec::new();
    for n in 1..=200u32 {
        let ref_path = ref_dir.join(format!("slide-{n:03}.png"));
        let ours_path = ours_dir.join(format!("slide-{n}.png"));
        if !ref_path.exists() || !ours_path.exists() {
            continue;
        }
        let (Some(a), Some(b)) = (signature(&ref_path), signature(&ours_path)) else {
            continue;
        };
        results.push((n, rmse(&a, &b)));
    }
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    println!("rank\tslide\trmse");
    for (rank, (n, score)) in results.iter().enumerate() {
        println!("{rank:4}\t{n:5}\t{score:.4}");
    }
    ExitCode::SUCCESS
}
