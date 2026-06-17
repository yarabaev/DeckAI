//! `slideglance` CLI dispatcher.
//!
//! Subcommands:
//!
//! - `convert <input> --output <out>` — render every slide (or a
//!   `--slide N` subset) to SVG / PNG.
//! - `inspect <input>` — print the archive's file tree + per-slide
//!   element type counts. Read-only diagnostic for "what's in this
//!   PPTX?".
//! - `render <input> --slide N --output <out.svg|out.png>` — fast
//!   single-slide render. Equivalent to `convert... --slide N` but
//!   guarantees one output file (errors if N is missing).
//!
//! No external arg-parser dep — `std::env::args` only.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use slideglance::{convert_to_png, convert_to_svg, parse_pptx, AdditionalFont, ConvertOptions};
use slideglance_font::{
    get_latin_os_defaults, standard_resolver_chain, BufferFontResolver, CjkPlatform, FontFace,
    FontMapping, FontResolver,
};
use slideglance_parser::PptxArchive;

fn main() -> ExitCode {
    let mut argv = std::env::args().skip(1).peekable();
    let subcommand = match argv.next().as_deref() {
        Some("convert") => Subcommand::Convert,
        Some("inspect") => Subcommand::Inspect,
        Some("render") => Subcommand::Render,
        Some("--help" | "-h" | "help") | None => {
            print_top_usage();
            return ExitCode::SUCCESS;
        }
        Some("--version" | "-V" | "version") => {
            println!("slideglance {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Some(other) => {
            eprintln!("error: unknown subcommand: {other}\n");
            print_top_usage();
            return ExitCode::from(2);
        }
    };

    // Subcommand-level `--help`: peek the next arg before dispatching.
    if argv
        .peek()
        .is_some_and(|a| a == "--help" || a == "-h" || a == "help")
    {
        match subcommand {
            Subcommand::Convert => print_convert_usage(),
            Subcommand::Inspect => print_inspect_usage(),
            Subcommand::Render => print_render_usage(),
        }
        return ExitCode::SUCCESS;
    }

    let result = match subcommand {
        Subcommand::Convert => run_convert_cli(argv),
        Subcommand::Inspect => run_inspect_cli(argv),
        Subcommand::Render => run_render_cli(argv),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

enum Subcommand {
    Convert,
    Inspect,
    Render,
}

fn print_top_usage() {
    let v = env!("CARGO_PKG_VERSION");
    eprintln!(
        "slideglance {v} — deterministic PPTX → SVG / PNG conversion\n\
\n\
USAGE\n    slideglance <COMMAND> [OPTIONS]\n\
\n\
COMMANDS\n\
    convert    Render every slide (or a subset) into a directory of SVG / PNG files\n\
    render     Render a single slide to one output file\n\
    inspect    Print archive metadata (slides, fonts, layouts, embedded media)\n\
    help       Show this message — also `--help`, `-h`\n\
    version    Show version info — also `--version`, `-V`\n\
\n\
PER-COMMAND HELP\n\
    slideglance convert --help\n\
    slideglance render --help\n\
    slideglance inspect --help\n\
\n\
QUICK EXAMPLES\n\
    slideglance convert deck.pptx --output out/                          # all slides → SVG in out/\n\
    slideglance convert deck.pptx --output out/ --format png --width 1600\n\
    slideglance convert deck.pptx --output out/ --range 1-10 --format png --width 1280\n\
    slideglance render deck.pptx --slide 3 --output slide-3.png --width 1600\n\
    slideglance inspect deck.pptx\n\
\n\
NOTES\n\
    * SVG output is deterministic (same input + options → byte-identical SVG).\n\
    * PNG path-mode emits glyph outlines; supply system fonts via `--font` for accurate Hangul / CJK metrics.\n\
    * Common font dirs (~/Library/Fonts, /System/Library/Fonts, ~/.fonts) are auto-loaded for measurement only."
    );
}

fn print_convert_usage() {
    eprintln!(
        "slideglance convert — render every slide (or a subset) to SVG / PNG files\n\
\n\
USAGE\n    slideglance convert <INPUT.pptx> --output <DIR> [OPTIONS]\n\
\n\
OPTIONS\n\
    -o, --output <DIR>           Directory the per-slide files are written into. Created if absent.\n\
    -f, --format <svg|png>       Output format. Default: svg.\n\
        --slide <N>              Render only this 1-based slide number. Repeatable.\n\
        --range <LO-HI>          Render slides LO through HI (inclusive). Combine with --slide if needed.\n\
        --width <PX>             Output width in pixels (PNG only). Aspect ratio preserved.\n\
        --height <PX>            Output height (PNG only). Honoured when --width is unset.\n\
        --font <PATH>            Add a TTF/OTF/TTC byte buffer to the resolver. Repeatable.\n\
                                 Required for accurate Korean / CJK rendering when the deck doesn't embed its own faces.\n\
        --pad <N>                Zero-pad slide numbers in output filenames to N digits (e.g. --pad 3 → slide-001.svg).\n\
    -h, --help                   Show this message.\n\
\n\
EXAMPLES\n\
    slideglance convert deck.pptx --output out/\n\
    slideglance convert deck.pptx --output out/ --format png --width 1600 --pad 3\n\
    slideglance convert deck.pptx --output out/ --range 1-10 --format svg\n\
    slideglance convert deck.pptx --output out/ --format png --width 1280 \\\n\
        --font /System/Library/Fonts/AppleSDGothicNeo.ttc\n\
\n\
OUTPUT\n    out/slide-1.svg, out/slide-2.svg, …  (or .png with --format png)"
    );
}

fn print_render_usage() {
    eprintln!(
        "slideglance render — render a single slide to one output file\n\
\n\
USAGE\n    slideglance render <INPUT.pptx> --slide <N> --output <FILE.svg|.png> [OPTIONS]\n\
\n\
OPTIONS\n\
    -o, --output <FILE>          Output file path. Format inferred from extension if --format absent.\n\
        --slide <N>              1-based slide number. Required.\n\
    -f, --format <svg|png>       Override the format. Defaults to the file extension.\n\
        --width <PX>             Output width in pixels (PNG only).\n\
        --height <PX>            Output height (PNG only). Honoured when --width is unset.\n\
        --font <PATH>            Add a TTF/OTF/TTC byte buffer. Repeatable.\n\
    -h, --help                   Show this message.\n\
\n\
EXAMPLES\n\
    slideglance render deck.pptx --slide 3 --output slide-3.svg\n\
    slideglance render deck.pptx --slide 1 --output cover.png --width 1920\n\
    slideglance render deck.pptx --slide 5 --output thumb.png --width 320 \\\n\
        --font ~/Library/Fonts/Pretendard-Regular.otf"
    );
}

fn print_inspect_usage() {
    eprintln!(
        "slideglance inspect — print archive contents and per-slide element summary\n\
\n\
USAGE\n    slideglance inspect <INPUT.pptx>\n\
\n\
OUTPUT\n\
    == Archive ==                  XML and media file listing\n\
    == Slides ==                   slide count, layout names, element type counts\n\
    == Fonts ==                    embedded font faces declared in `<p:embeddedFontLst>`\n\
\n\
EXAMPLES\n\
    slideglance inspect deck.pptx\n\
    slideglance inspect deck.pptx | less"
    );
}

// ---------------------------------------------------------------------
// `convert` — render every (or a subset of) slides
// ---------------------------------------------------------------------

#[derive(Default)]
struct ConvertCli {
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    format: OutputFormat,
    slides: Vec<u32>,
    range: Option<(u32, u32)>,
    width: Option<u32>,
    height: Option<u32>,
    fonts: Vec<PathBuf>,
    pad: usize,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    #[default]
    Svg,
    Png,
}

fn run_convert_cli(argv: impl Iterator<Item = String>) -> Result<(), String> {
    let cli = parse_convert_args(argv.peekable())?;
    let input = cli.input.ok_or_else(|| {
        "missing input PPTX path — usage: slideglance convert <INPUT.pptx> --output <DIR>\n       \
         try `slideglance convert --help` for details"
            .to_string()
    })?;
    let output = cli.output.ok_or_else(|| {
        "missing --output <DIR> — try `slideglance convert --help` for details".to_string()
    })?;
    let mut slides = cli.slides;
    if let Some((lo, hi)) = cli.range {
        for s in lo..=hi {
            slides.push(s);
        }
    }
    convert_pipeline(
        &input, &output, cli.format, slides, cli.width, cli.height, &cli.fonts, cli.pad,
    )
}

fn parse_convert_args(
    mut argv: std::iter::Peekable<impl Iterator<Item = String>>,
) -> Result<ConvertCli, String> {
    let mut parsed = ConvertCli::default();
    while let Some(token) = argv.next() {
        match token.as_str() {
            "--output" | "-o" => {
                parsed.output = Some(PathBuf::from(require_value(&mut argv, "--output")?));
            }
            "--format" | "-f" => {
                parsed.format = parse_format(&require_value(&mut argv, "--format")?)?;
            }
            "--slide" => parsed
                .slides
                .push(parse_u32(&require_value(&mut argv, "--slide")?, "--slide")?),
            "--range" => {
                let v = require_value(&mut argv, "--range")?;
                parsed.range = Some(parse_range(&v)?);
            }
            "--width" => {
                parsed.width = Some(parse_u32(&require_value(&mut argv, "--width")?, "--width")?);
            }
            "--height" => {
                parsed.height = Some(parse_u32(
                    &require_value(&mut argv, "--height")?,
                    "--height",
                )?);
            }
            "--font" => parsed
                .fonts
                .push(PathBuf::from(require_value(&mut argv, "--font")?)),
            "--pad" => {
                parsed.pad = parse_u32(&require_value(&mut argv, "--pad")?, "--pad")? as usize;
            }
            "--help" | "-h" => {
                print_convert_usage();
                std::process::exit(0);
            }
            other if other.starts_with("--") => return Err(format!("unknown flag: {other}")),
            other => {
                if parsed.input.is_some() {
                    return Err(format!("unexpected positional argument: {other}"));
                }
                parsed.input = Some(PathBuf::from(other));
            }
        }
    }
    Ok(parsed)
}

fn parse_range(s: &str) -> Result<(u32, u32), String> {
    let mut parts = s.splitn(2, '-');
    let lo = parts
        .next()
        .ok_or_else(|| format!("--range {s}: missing low"))?;
    let hi = parts
        .next()
        .ok_or_else(|| format!("--range {s}: expected lo-hi (e.g. 1-132)"))?;
    let lo = parse_u32(lo, "--range lo")?;
    let hi = parse_u32(hi, "--range hi")?;
    if lo > hi {
        return Err(format!("--range {s}: lo > hi"));
    }
    Ok((lo, hi))
}

// ---------------------------------------------------------------------
// `render` — single slide
// ---------------------------------------------------------------------

#[derive(Default)]
struct RenderCli {
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    format: Option<OutputFormat>,
    slide: Option<u32>,
    width: Option<u32>,
    height: Option<u32>,
    fonts: Vec<PathBuf>,
}

fn run_render_cli(argv: impl Iterator<Item = String>) -> Result<(), String> {
    let cli = parse_render_args(argv.peekable())?;
    let input = cli.input.ok_or_else(|| {
        "missing input PPTX path — usage: slideglance render <INPUT.pptx> --slide <N> --output <FILE>\n       \
         try `slideglance render --help` for details"
            .to_string()
    })?;
    let output = cli.output.ok_or_else(|| {
        "missing --output <FILE> — try `slideglance render --help` for details".to_string()
    })?;
    let slide = cli.slide.ok_or_else(|| {
        "missing --slide <N> — try `slideglance render --help` for details".to_string()
    })?;
    // Format precedence: explicit --format > extension sniff > svg.
    let format = cli.format.unwrap_or_else(|| sniff_format(&output));
    convert_pipeline(
        &input,
        &output,
        format,
        vec![slide],
        cli.width,
        cli.height,
        &cli.fonts,
        0,
    )
}

fn parse_render_args(
    mut argv: std::iter::Peekable<impl Iterator<Item = String>>,
) -> Result<RenderCli, String> {
    let mut parsed = RenderCli::default();
    while let Some(token) = argv.next() {
        match token.as_str() {
            "--output" | "-o" => {
                parsed.output = Some(PathBuf::from(require_value(&mut argv, "--output")?));
            }
            "--format" | "-f" => {
                parsed.format = Some(parse_format(&require_value(&mut argv, "--format")?)?);
            }
            "--slide" => {
                parsed.slide = Some(parse_u32(&require_value(&mut argv, "--slide")?, "--slide")?);
            }
            "--width" => {
                parsed.width = Some(parse_u32(&require_value(&mut argv, "--width")?, "--width")?);
            }
            "--height" => {
                parsed.height = Some(parse_u32(
                    &require_value(&mut argv, "--height")?,
                    "--height",
                )?);
            }
            "--font" => parsed
                .fonts
                .push(PathBuf::from(require_value(&mut argv, "--font")?)),
            "--help" | "-h" => {
                print_render_usage();
                std::process::exit(0);
            }
            other if other.starts_with("--") => return Err(format!("unknown flag: {other}")),
            other => {
                if parsed.input.is_some() {
                    return Err(format!("unexpected positional argument: {other}"));
                }
                parsed.input = Some(PathBuf::from(other));
            }
        }
    }
    Ok(parsed)
}

fn sniff_format(path: &Path) -> OutputFormat {
    match path.extension().and_then(|e| e.to_str()) {
        Some("png" | "PNG") => OutputFormat::Png,
        _ => OutputFormat::Svg,
    }
}

// ---------------------------------------------------------------------
// `inspect` — read-only archive + element summary
// ---------------------------------------------------------------------

fn run_inspect_cli(mut argv: impl Iterator<Item = String>) -> Result<(), String> {
    let arg = argv.next();
    if matches!(arg.as_deref(), Some("--help" | "-h" | "help")) {
        print_inspect_usage();
        return Ok(());
    }
    let input = arg.map(PathBuf::from).ok_or_else(|| {
        "missing input PPTX path — usage: slideglance inspect <INPUT.pptx>".to_string()
    })?;
    if argv.next().is_some() {
        return Err("inspect takes only one positional argument".to_string());
    }

    let display = input.display();
    let bytes = std::fs::read(&input).map_err(|e| format!("read {display}: {e}"))?;

    // Archive walk first — fast, doesn't pay the full parser cost.
    let archive = PptxArchive::open(bytes.clone()).map_err(|e| format!("open archive: {e:?}"))?;
    let xml_files = archive.xml_files();
    let media = archive.media_paths();
    let mut xml_paths: Vec<&String> = xml_files.keys().collect();
    xml_paths.sort();
    let mut media_paths: Vec<&String> = media.iter().collect();
    media_paths.sort();

    println!("== Archive ({display}) ==");
    println!("XML files: {}", xml_paths.len());
    for p in &xml_paths {
        println!(" {p}");
    }
    println!("Media files: {}", media_paths.len());
    for p in &media_paths {
        println!(" {p}");
    }

    // Full parse + per-slide element type counts.
    let presentation = parse_pptx(bytes).map_err(|e| format!("parse_pptx: {e}"))?;
    println!();
    println!("== Presentation ==");
    println!(
        "slide_size: {}x{} EMU",
        presentation.info.slide_size.width.raw(),
        presentation.info.slide_size.height.raw()
    );
    println!("slides: {}", presentation.slides.len());
    for (i, s) in presentation.slides.iter().enumerate() {
        let counts = element_type_counts(&s.slide.elements);
        println!(
            " [{}] slide_number={} layout_name={:?} elements={}",
            i + 1,
            s.slide.slide_number,
            s.slide.layout_name,
            s.slide.elements.len()
        );
        for (kind, n) in counts {
            println!(" - {kind}: {n}");
        }
        if let Some(notes) = &s.slide.notes {
            let preview: String = notes.chars().take(40).collect();
            println!(" notes: {preview:?}");
        }
    }
    Ok(())
}

fn element_type_counts(elements: &[slideglance_model::SlideElement]) -> Vec<(&'static str, usize)> {
    let mut shape = 0usize;
    let mut connector = 0usize;
    let mut group = 0usize;
    let mut image = 0usize;
    let mut chart = 0usize;
    let mut table = 0usize;
    for el in elements {
        match el {
            slideglance_model::SlideElement::Shape(_) => shape += 1,
            slideglance_model::SlideElement::Connector(_) => connector += 1,
            slideglance_model::SlideElement::Group(_) => group += 1,
            slideglance_model::SlideElement::Image(_) => image += 1,
            slideglance_model::SlideElement::Chart(_) => chart += 1,
            slideglance_model::SlideElement::Table(_) => table += 1,
        }
    }
    [
        ("shape", shape),
        ("connector", connector),
        ("group", group),
        ("image", image),
        ("chart", chart),
        ("table", table),
    ]
    .into_iter()
    .filter(|(_, n)| *n > 0)
    .collect()
}

// ---------------------------------------------------------------------
// shared pipeline
// ---------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn convert_pipeline(
    input: &Path,
    output: &Path,
    format: OutputFormat,
    slides: Vec<u32>,
    width: Option<u32>,
    height: Option<u32>,
    fonts: &[PathBuf],
    pad: usize,
) -> Result<(), String> {
    let input_display = input.display();
    let bytes = std::fs::read(input).map_err(|e| format!("read {input_display}: {e}"))?;

    let font_buffers: Vec<Vec<u8>> = fonts
        .iter()
        .map(|p| std::fs::read(p).map_err(|e| format!("read font {}: {e}", p.display())))
        .collect::<Result<_, _>>()?;
    // Walk the host font directories. Used for: (1) measurement
    // (`fonts.measurement_only_fonts`), so wrap calculations use real
    // per-glyph advances; (2) the path-mode font resolver, so glyph
    // outline extraction can fall back to OS-installed faces when the
    // slide references a font that was not passed via `--font`.
    let host_font_buffers = load_host_font_buffers();
    let resolver = build_resolver(&font_buffers, &host_font_buffers)?;
    // Wrap every `--font` buffer as an `AdditionalFont` so it lands in
    // both the SVG `<defs>` block and the PNG rasterizer fontdb (KDD-22:
    // single source of truth — `inline_fonts` drives both). Chart
    // `<text>` labels still resolve through the curated set; the rest of
    // the host font set stays measurement-only to keep startup fast.
    let inline_fonts: Vec<AdditionalFont> = font_buffers
        .iter()
        .filter_map(|bytes| {
            let typeface = slideglance_font::all_face_family_names(bytes)
                .into_iter()
                .next()?;
            Some(AdditionalFont::regular(typeface, bytes.clone()))
        })
        .collect();
    let measurement_only_fonts = host_font_buffers;

    let opts = ConvertOptions {
        slides: if slides.is_empty() {
            None
        } else {
            Some(slides)
        },
        width,
        height,
        fonts: slideglance::FontConfig {
            resolver: resolver.map(|r| r as Box<dyn slideglance_font::FontResolver + Send + Sync>),
            inline_fonts,
            measurement_only_fonts,
            ..slideglance::FontConfig::default()
        },
        ..ConvertOptions::default()
    };

    match format {
        OutputFormat::Svg => {
            let rendered = convert_to_svg(bytes, &opts).map_err(|e| e.to_string())?;
            write_outputs_padded(
                output,
                "svg",
                rendered
                    .into_iter()
                    .map(|s| (s.slide_number, s.svg.into_bytes())),
                pad,
            )
        }
        OutputFormat::Png => {
            let rendered = convert_to_png(bytes, &opts).map_err(|e| e.to_string())?;
            write_outputs_padded(
                output,
                "png",
                rendered.into_iter().map(|s| (s.slide_number, s.png)),
                pad,
            )
        }
    }
}

/// Walk the host's standard font directories and return every
/// TTF / OTF / TTC byte buffer. Used to feed `ConvertOptions
///.measurement_fonts` so wrap calculations use real per-glyph
/// advances. Errors silently — a missing directory just contributes
/// no fonts; the renderer still works (heuristic measurer fallback).
fn load_host_font_buffers() -> Vec<Vec<u8>> {
    use std::path::Path;
    fn walk(dir: &Path, out: &mut Vec<Vec<u8>>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                let ext_lc = ext.to_ascii_lowercase();
                if matches!(ext_lc.as_str(), "ttf" | "otf" | "ttc") {
                    if let Ok(bytes) = std::fs::read(&path) {
                        out.push(bytes);
                    }
                }
            }
        }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let dirs: &[&str] = if cfg!(target_os = "macos") {
        &["/System/Library/Fonts", "/Library/Fonts"]
    } else if cfg!(target_os = "windows") {
        &["C:\\Windows\\Fonts"]
    } else {
        &["/usr/share/fonts", "/usr/local/share/fonts"]
    };
    let mut out: Vec<Vec<u8>> = Vec::new();
    for d in dirs {
        walk(Path::new(d), &mut out);
    }
    if !home.is_empty() {
        if cfg!(target_os = "macos") {
            walk(Path::new(&format!("{home}/Library/Fonts")), &mut out);
        } else {
            walk(Path::new(&format!("{home}/.fonts")), &mut out);
            walk(Path::new(&format!("{home}/.local/share/fonts")), &mut out);
        }
    }
    out
}

/// Build the path-mode font resolver chain.
///
/// `cli_fonts` are the `--font` paths the user explicitly passed; parse
/// errors here are fatal (the user asked for these fonts specifically).
/// `host_fonts` come from the OS font directory walk; parse errors there
/// are silently skipped (a corrupt legacy font on the host shouldn't
/// abort the whole render).
///
/// Each TTC sub-face is registered under every alias `all_family_names`
/// reports, so a deck referencing e.g. `"Apple SD Gothic Neo"` /
/// `"AppleSDGothicNeo"` / a localized Korean alias all resolve to the
/// same face. The whole chain is wrapped via `standard_resolver_chain`
/// (PPTX → OSS replacement + per-platform CJK fallback) so legacy CJK
/// names like `"맑은 고딕"` fall through to whichever Korean / CJK face
/// the host happens to ship — matching what `PowerPoint` does on a
/// machine without the original font installed.
///
/// The CLI then wraps the standard chain with [`LatinOsDefaultsFallback`]
/// so a deck referencing a Latin family that is neither installed nor
/// listed in the OSS mapping table (e.g. `Inter Display` on a Mac that
/// has not installed Inter) falls back to the host's OS-default sans
/// (Helvetica Neue / Segoe UI / Liberation Sans). Without this layer
/// the PNG raster path emits blank text for unknown Latin families,
/// since path-mode rendering bypasses the SVG `font-family=` chain
/// that `latin_defaults` already injects for browser consumers.
fn build_resolver(
    cli_fonts: &[Vec<u8>],
    host_fonts: &[Vec<u8>],
) -> Result<Option<Box<dyn FontResolver + Send + Sync>>, String> {
    let mut buffer = BufferFontResolver::new();

    for (i, bytes) in cli_fonts.iter().enumerate() {
        register_font_buffer(&mut buffer, bytes)
            .map_err(|e| format!("font {i} parse error: {e}"))?;
    }
    for bytes in host_fonts {
        // Host fonts: a single broken file should not abort startup.
        let _ = register_font_buffer(&mut buffer, bytes);
    }

    if buffer.is_empty() {
        return Ok(None);
    }

    let platform = CjkPlatform::current();
    let chained = standard_resolver_chain(buffer, FontMapping::new(), platform);
    let with_latin_fallback = LatinOsDefaultsFallback {
        inner: chained,
        platform,
    };
    Ok(Some(Box::new(with_latin_fallback)))
}

/// CLI-local resolver wrapper: when `inner` cannot resolve a name,
/// retry with each entry in [`get_latin_os_defaults`] for the host
/// platform. Lives in the binary rather than `slideglance-font`
/// because library callers (WASM bridge, browser renderer) intentionally
/// let the browser's per-glyph CSS fallback do the same job — only the
/// native PNG path-mode pipeline needs an in-process substitute.
struct LatinOsDefaultsFallback<R: FontResolver> {
    inner: R,
    platform: CjkPlatform,
}

impl<R: FontResolver> FontResolver for LatinOsDefaultsFallback<R> {
    fn resolve(&self, name: &str) -> Option<Arc<FontFace>> {
        if let Some(face) = self.inner.resolve(name) {
            return Some(face);
        }
        for default_name in get_latin_os_defaults(self.platform) {
            if let Some(face) = self.inner.resolve(default_name) {
                return Some(face);
            }
        }
        None
    }
}

/// Register every TTC sub-face under every alias the face reports.
///
/// Mirrors what a system font scanner would do: a single `.ttc` file
/// like `AppleSDGothicNeo.ttc` carries 8+ weight variants; we want every
/// one available under both the localized `"Apple SD Gothic Neo"` form
/// and the no-space `"AppleSDGothicNeo"` form so `<a:latin typeface="…">`
/// matches regardless of how the deck spells the family name.
///
/// Italic/bold faces are intentionally NOT registered under the bare
/// family name (`"Arial"`) because the typographic family ID is shared
/// across the whole family — registering both `Arial.ttf` and
/// `Arial Italic.ttf` under `"Arial"` is a name collision where
/// `BTreeMap::insert` lets the last write win, and the OS read-dir order
/// often places the italic variant after the regular. The result was
/// `<a:buFont typeface="Arial">` resolving to the italic face and every
/// auto-numbered bullet ("1.", "2.", "3.") rendering slanted. Italic /
/// bold variants stay reachable through their full names (`"Arial
/// Italic"`, `"Arial Bold"`), which `resolve_face` already tries first
/// when the run requests them.
fn register_font_buffer(
    buffer: &mut BufferFontResolver,
    bytes: &[u8],
) -> Result<(), slideglance_font::FontError> {
    let face_count = ttf_parser::fonts_in_collection(bytes).unwrap_or(1);
    for idx in 0..face_count {
        let face = FontFace::from_bytes(bytes.to_vec(), idx)?;
        let parsed =
            ttf_parser::Face::parse(bytes, idx).map_err(slideglance_font::FontError::from)?;
        let is_italic = parsed.is_italic();
        let is_bold = parsed.is_bold();
        let is_styled = is_italic || is_bold;
        let primary = face.family_name();
        let face_arc = Arc::new(face);
        let mut names = face_arc.all_family_names();
        if let Some(p) = &primary {
            if !names.contains(p) {
                names.push(p.clone());
            }
        }
        for name in names {
            if name.is_empty() {
                continue;
            }
            // Styled (italic / bold) faces only register under names that
            // already encode the style suffix — never under the bare
            // family name shared with the regular face.
            if is_styled
                && name_matches_bare_family(&name, primary.as_ref(), &names_lower(&face_arc))
            {
                continue;
            }
            buffer.insert_arc(name, face_arc.clone());
        }
    }
    Ok(())
}

/// Lowercase set of every name a face exposes — used to detect when an
/// alias is the "bare family" form (no Italic/Bold suffix).
fn names_lower(face: &FontFace) -> Vec<String> {
    face.all_family_names()
        .into_iter()
        .map(|n| n.to_lowercase())
        .collect()
}

/// True when `name` is the unsuffixed family form (e.g. `"Arial"`) for a
/// face whose other aliases do carry the style suffix (`"Arial Italic"`,
/// `"Arial Bold"`). Heuristic — checks whether `name` lowercased is a
/// prefix of any other registered alias and the suffix is purely style
/// terms (`italic`, `bold`, `oblique`, ...).
fn name_matches_bare_family(
    name: &str,
    primary: Option<&String>,
    other_aliases_lower: &[String],
) -> bool {
    // The "preferred family" (name id 16) is by definition unsuffixed.
    // If our name equals the typographic family that other aliases
    // extend with a style suffix, treat this as bare.
    let lower = name.to_lowercase();
    let primary_matches = primary
        .as_ref()
        .is_some_and(|p| p.eq_ignore_ascii_case(name));
    if !primary_matches {
        return false;
    }
    other_aliases_lower.iter().any(|alias| {
        if alias == &lower {
            return false;
        }
        let Some(suffix) = alias.strip_prefix(&lower) else {
            return false;
        };
        let suffix = suffix.trim_start();
        let trimmed = suffix.trim_end_matches(' ').trim_end_matches('-');
        // Style-only suffix tokens, separately or combined.
        const STYLE_TOKENS: &[&str] = &[
            "italic",
            "bold",
            "oblique",
            "bold italic",
            "italic bold",
            "bolditalic",
            "italicbold",
        ];
        STYLE_TOKENS.iter().any(|t| trimmed.eq_ignore_ascii_case(t))
    })
}

#[allow(dead_code)]
fn write_outputs(
    output: &Path,
    extension: &str,
    items: impl IntoIterator<Item = (u32, Vec<u8>)>,
) -> Result<(), String> {
    write_outputs_padded(output, extension, items, 0)
}

/// Multi-slide write with zero-padded slide numbers (e.g. `slide-001.png`).
/// Used by callers that want predictable file names for downstream tooling
/// (the `.compare/` diff framework expects 3-digit padding so 132-slide
/// listings sort lexicographically).
fn write_outputs_padded(
    output: &Path,
    extension: &str,
    items: impl IntoIterator<Item = (u32, Vec<u8>)>,
    pad: usize,
) -> Result<(), String> {
    let collected: Vec<(u32, Vec<u8>)> = items.into_iter().collect();
    if collected.is_empty() {
        return Err("no slides matched the filter".to_string());
    }
    if collected.len() == 1 && pad == 0 {
        let (_, bytes) = &collected[0];
        return write_file(output, bytes);
    }
    let display = output.display();
    std::fs::create_dir_all(output).map_err(|e| format!("mkdir {display}: {e}"))?;
    for (n, bytes) in &collected {
        let path = if pad > 0 {
            output.join(format!("slide-{n:0pad$}.{extension}"))
        } else {
            output.join(format!("slide-{n}.{extension}"))
        };
        write_file(&path, bytes)?;
    }
    Ok(())
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let display = path.display();
    let mut f = std::fs::File::create(path).map_err(|e| format!("create {display}: {e}"))?;
    f.write_all(bytes)
        .map_err(|e| format!("write {display}: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------

fn require_value(
    argv: &mut std::iter::Peekable<impl Iterator<Item = String>>,
    flag: &str,
) -> Result<String, String> {
    argv.next()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn parse_format(s: &str) -> Result<OutputFormat, String> {
    match s {
        "svg" => Ok(OutputFormat::Svg),
        "png" => Ok(OutputFormat::Png),
        other => Err(format!("--format must be svg or png, got {other}")),
    }
}

fn parse_u32(s: &str, flag: &str) -> Result<u32, String> {
    s.parse::<u32>().map_err(|e| format!("{flag} {s}: {e}"))
}
