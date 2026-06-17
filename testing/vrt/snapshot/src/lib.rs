//! Snapshot-based visual regression tests for slideglance.
//!
//! Standalone Cargo package — kept out of the workspace so the main
//! `cargo test --workspace` matrix doesn't materialize the snapshot
//! tree on every run. Invoke explicitly:
//!
//! ```sh
//! cargo test --manifest-path testing/vrt/snapshot/Cargo.toml
//! # update baselines after intentional renderer changes:
//! UPDATE_SNAPSHOTS=1 cargo test --manifest-path testing/vrt/snapshot/Cargo.toml
//! ```
//!
//! ### Strategy
//!
//! - **Source of truth: Rust SVG output.** Each fixture × slide is
//!   rendered through `slideglance::convert_to_svg` and the result is
//!   compared byte-for-byte against the persisted snapshot under
//!   `snapshots/{fixture}/slide-{n}.svg`.
//! - **Determinism is the contract.** No fonts are injected here, so
//!   the renderer stays in text-mode (`<text>`/`<tspan>`); `convert_to_svg`
//!   never reads the host clock or the system font catalog. Same input
//!   → byte-identical output across runs and across native/WASM builds.
//! - **Drift detection.** Any change to the renderer that affects the
//!   output of a fixture surfaces as a snapshot diff. The intent is to
//!   force conscious approval (re-run with `UPDATE_SNAPSHOTS=1`).
//! - **Cross-platform.** Snapshots use LF line endings + the renderer's
//!   compact format, both of which are stable across operating systems.

#![deny(missing_docs)]

use std::fs;
use std::path::{Path, PathBuf};

use slideglance_font::CjkPlatform;
use slideglance::{convert_to_svg, ConvertError, ConvertOptions, FontConfig};

/// One fixture entry — a PPTX file plus the slide indices the
/// snapshot suite materializes. `None` = every slide.
#[derive(Debug, Clone)]
pub struct VrtCase {
    /// Display name used as the snapshot directory.
    pub name: &'static str,
    /// Path relative to the repository root (e.g. `testing/fixtures/sample.pptx`).
    pub fixture: &'static str,
    /// 1-based slide numbers to capture, or `None` for every slide.
    pub slides: Option<&'static [u32]>,
}

/// Static case list. Adding a fixture = appending an entry here and
/// re-running with `UPDATE_SNAPSHOTS=1` to materialize the baseline.
///
/// Empty by default — this repo ships the runner machinery, not the
/// fixtures themselves. Drop your `.pptx` files into `testing/fixtures/`
/// and add their entries below.
pub const CASES: &[VrtCase] = &[];

/// Resolve a path relative to the workspace root (three levels up
/// from this crate's manifest at `testing/vrt/snapshot/`).
#[must_use]
pub fn workspace_path(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join(rel)
}

/// Snapshot directory for `case`.
#[must_use]
pub fn snapshot_dir(case: &VrtCase) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("snapshots")
        .join(case.name)
}

/// Render every slide in `case` to SVG. Returns `(slide_number, svg)`
/// pairs for the slides selected by `case.slides` (or every slide).
///
/// # Errors
///
/// Bubbles up [`ConvertError`] from [`convert_to_svg`].
pub fn render_case(case: &VrtCase) -> Result<Vec<(u32, String)>, ConvertError> {
    let bytes = fs::read(workspace_path(case.fixture)).expect("fixture exists");
    let opts = ConvertOptions {
        slides: case.slides.map(<[u32]>::to_vec),
        // Force `Other` so the snapshot is byte-identical regardless
        // of which OS runs the test (production callers leave this at
        // the auto-detected host platform).
        cjk_platform: CjkPlatform::Other,
        ..ConvertOptions::default()
    };
    let slides = convert_to_svg(bytes, &opts)?;
    Ok(slides
        .into_iter()
        .map(|s| (s.slide_number, s.svg))
        .collect())
}

/// Render every slide in `case` to SVG with explicit `embed_fonts` control.
///
/// When `embed_fonts` is `true`, `<p:embeddedFontLst>` entries are inlined
/// into the SVG as `@font-face data:` URIs. When `false`, they are omitted.
///
/// # Errors
///
/// Bubbles up [`ConvertError`] from [`convert_to_svg`].
pub fn render_case_with_embed(
    case: &VrtCase,
    embed_fonts: bool,
) -> Result<Vec<(u32, String)>, ConvertError> {
    let bytes = fs::read(workspace_path(case.fixture)).expect("fixture exists");
    let opts = ConvertOptions {
        slides: case.slides.map(<[u32]>::to_vec),
        cjk_platform: CjkPlatform::Other,
        fonts: FontConfig {
            embed_deck_fonts: embed_fonts,
            ..FontConfig::default()
        },
        ..ConvertOptions::default()
    };
    let slides = convert_to_svg(bytes, &opts)?;
    Ok(slides
        .into_iter()
        .map(|s| (s.slide_number, s.svg))
        .collect())
}

/// Whether snapshot writes are enabled for this run. `UPDATE_SNAPSHOTS=1`
/// (any non-empty value) flips the assert helper into write mode.
#[must_use]
pub fn update_mode() -> bool {
    matches!(std::env::var("UPDATE_SNAPSHOTS"), Ok(v) if !v.is_empty())
}

/// Compare `actual` against the on-disk snapshot at `path`. Writes a
/// fresh snapshot when [`update_mode`] is on; panics with a unified
/// diff otherwise.
pub fn assert_snapshot(path: &Path, actual: &str) {
    if update_mode() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create snapshot dir");
        }
        fs::write(path, actual).expect("write snapshot");
        return;
    }
    let expected = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => panic!(
            "missing snapshot at {} — re-run with UPDATE_SNAPSHOTS=1 to generate",
            path.display()
        ),
    };
    if expected == actual {
        return;
    }
    panic!(
        "snapshot diverged at {}\n--- expected ({} bytes)\n+++ actual ({} bytes)\n{}",
        path.display(),
        expected.len(),
        actual.len(),
        unified_diff(&expected, actual),
    );
}

fn unified_diff(a: &str, b: &str) -> String {
    // Very small textual diff — line-level, no LCS. Enough to point at
    // the first divergence; the snapshot files are the canonical
    // record. Avoids pulling in a `similar` / `dissimilar` dep.
    let mut out = String::new();
    for (i, (al, bl)) in a.lines().zip(b.lines()).enumerate() {
        if al != bl {
            out.push_str(&format!("@ line {}\n- {al}\n+ {bl}\n", i + 1));
            if out.lines().count() > 30 {
                out.push_str("…(truncated)\n");
                return out;
            }
        }
    }
    if a.lines().count() != b.lines().count() {
        out.push_str(&format!(
            "@ line counts: expected {}, actual {}\n",
            a.lines().count(),
            b.lines().count()
        ));
    }
    if out.is_empty() {
        out.push_str("(no line-level diff — likely a trailing newline difference)\n");
    }
    out
}
