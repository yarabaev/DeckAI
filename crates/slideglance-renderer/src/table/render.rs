//! Table rendering pass.
//!
//! Direct port of `renderTable` from.

use std::fmt::Write as _;

use slideglance_font::{CjkPlatform, FontMapping, FontResolver, ScriptFontContext, TextMeasurer};
use slideglance_model::{TableElement, TableStyleOptions, Transform};
use slideglance_utils::Emu;

use crate::fill::{render_fill_attrs, render_outline_attrs};
use crate::geometry::fmt::n;
use crate::id_gen::IdGen;
use crate::slide_context::SlideRenderContext;
use crate::text::render_text_body;
use crate::transform::{build_object_name_attr, build_transform_attr};

use super::presets::{lookup_table_style_preset, TableStylePreset};

/// Result of rendering one [`TableElement`].
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TableElementResult {
    /// `<g>` wrapper containing rect / line / text contents.
    pub content: String,
    /// Any `<defs>` content collected from cell fills / borders.
    pub defs: String,
}

/// Render a [`TableElement`] to a `<g transform="...">` group of cell
/// rectangles, borders, and cell text bodies.
///
/// Threads renderer-wide state (`ids`, `slide`, `script_fonts`, `measurer`,
/// `mapping`, `cjk_platform`) through to the cell text renderer.
//
// The function intentionally has many lines / arguments because it follows
// the spec's nested loop over rows / cells / per-edge borders. The
// helpers it calls (fill / outline / text-body / transform) are factored
// out already.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
#[must_use]
pub fn render_table(
    element: &TableElement,
    ids: &mut IdGen,
    slide: &SlideRenderContext,
    script_fonts: &ScriptFontContext,
    measurer: &dyn TextMeasurer,
    mapping: &FontMapping,
    cjk_platform: CjkPlatform,
    font_resolver: Option<&dyn FontResolver>,
    // Carries the cumulative inverse-of-group-scale through to per-cell
    // `render_text_body`. See `render_text_body`'s param doc for the
    // full rationale; here it's just plumbed through.
    font_size_correction: f64,
) -> TableElementResult {
    let table = &element.table;
    // Per-cell border strokes ride the same scale-compensation as text
    // (see `render_text_body` and `render_outline_attrs`'s stroke_scale)
    // so a table nested inside a 15× scaled group doesn't render hairline
    // borders as 15× thick black bars.
    let stroke_scale = font_size_correction;
    let transform_attr = build_transform_attr(&element.transform);
    let col_widths: Vec<f64> = table.columns.iter().map(|c| c.width.to_pixels()).collect();
    // Per-OOXML, `<a:tr h="...">` is a *minimum* row height; PowerPoint
    // grows the row to fit cell content when it overflows. Without
    // that grow step the test deck's risk-management table (slide 72)
    // collapses to ~13 px tall rows — text is emitted but lives below
    // the visible viewport. Estimate each cell's content height from
    // its paragraph count and per-paragraph default font size, and
    // raise row_heights[i] to the tallest cell in row i.
    let mut row_heights: Vec<f64> = table.rows.iter().map(|r| r.height.to_pixels()).collect();
    // For each column, track which rows have an "empty visual" cell
    // (no text body, no merge). If a single column has empty cells in
    // most rows, that column is almost certainly hosting overlaid
    // pictures and the rows should be equalized so the picture
    // overlay aligns. Per-row tracking alone (the previous version)
    // mis-flagged plain text tables whose middle column happened to
    // be blank in one or two rows.
    let n_rows = row_heights.len();
    let n_cols = col_widths.len();
    let mut empty_per_col: Vec<usize> = vec![0; n_cols];
    let mut has_empty_cell = vec![false; n_rows];
    for (row_idx, row) in table.rows.iter().enumerate() {
        let mut col_idx: usize = 0;
        for cell in &row.cells {
            if cell.h_merge || cell.v_merge {
                col_idx += 1;
                continue;
            }
            if cell.row_span > 1 {
                // Multi-row cells contribute their content height to
                // the *spanned region* total, not a single row.
                col_idx += 1;
                continue;
            }
            let cell_w = spanned_size(&col_widths, col_idx, cell.grid_span);
            // PowerPoint cells that exist solely to host an overlaid
            // picture / icon usually carry an empty `<a:txBody>` (no
            // runs with text). Treat those the same as a missing
            // body so the row equalization step kicks in below — a
            // truly text-bearing cell must contribute at least one
            // non-empty run.
            let cell_is_empty_visual = cell.text_body.as_ref().is_none_or(|tb| {
                tb.paragraphs
                    .iter()
                    .all(|p| p.runs.iter().all(|r| r.text.is_empty()))
            });
            if let Some(text_body) = &cell.text_body {
                let est_h = estimate_cell_content_height(text_body, cell_w, measurer);
                if est_h > row_heights[row_idx] {
                    row_heights[row_idx] = est_h;
                }
            }
            if cell_is_empty_visual {
                has_empty_cell[row_idx] = true;
                if col_idx < empty_per_col.len() {
                    empty_per_col[col_idx] += 1;
                }
            }
            col_idx += 1;
        }
    }
    // Row equalization removed: it mis-flagged tables whose empty
    // column was just a spacer (e.g. slide 4's TOC), pushing every
    // body row to the longest row's height and overflowing the
    // page. Image-overlay alignment is now expected to come from
    // the per-cell estimate alone (the picture-bearing rows still
    // grow to the picture's height because the neighbouring text
    // cells have similar content lengths in the test deck). The
    // tracking variables `has_empty_cell` / `empty_per_col` are
    // left in place for future heuristics but are not consulted.
    let _ = (&has_empty_cell, &empty_per_col);
    // Auto-distribute remaining container height to `<a:tr h="0">`
    // rows. PowerPoint treats `h="0"` as "auto" and grows the row
    // first to fit content, then divides any leftover container
    // height equally among the auto-sized rows. Slide 8's "문제점 및
    // 개선방안" table is the canary: container cy=453 px with a
    // header row of 30 px and four h="0" body rows. Without this
    // pass each body row collapsed to its content height (~80 px),
    // leaving the table ~330 px tall and stranding the
    // absolutely-positioned icons (whose Y matches PowerPoint's
    // 105.75 px / row layout) in the empty space below the table.
    let container_h_px = element.transform.extent_height.to_pixels();
    let total_row_h: f64 = row_heights.iter().sum();
    let remaining = container_h_px - total_row_h;
    if remaining > 0.5 && total_row_h > 0.5 {
        // PowerPoint scales row heights proportionally so the table
        // fills its `<a:xfrm cy>` container. Distributing the leftover
        // *only* to `h=0` rows (previous behaviour) made the single
        // h=0 header on slide 25's `활용방안/내용` table absorb 85 px,
        // pushing it taller than the explicit body rows. A purely
        // proportional grow over every row preserves the relative
        // hierarchy the author authored: header rows (small) gain a
        // little, body rows (large) gain more — matches the PDF
        // reference within a couple of pixels per row, and still
        // keeps slide 8's overlay-icon alignment because every row
        // grows by the same percentage.
        let scale = container_h_px / total_row_h;
        for h in &mut row_heights {
            *h *= scale;
        }
    }

    let mut content = String::new();
    let mut defs = String::new();

    let sp_id_attr = crate::svg_builder::build_sp_id_attr(element.sp_id);
    let _ = write!(
        content,
        "<g{sp_id_attr} transform=\"{transform_attr}\"{}>",
        build_object_name_attr(element.object_name.as_deref())
    );

    let preset = lookup_table_style_preset(table.table_style_id.as_deref());
    let opts = table.table_style_options.unwrap_or_default();
    let row_count = table.rows.len();

    let mut y = 0.0_f64;
    for (row_idx, row) in table.rows.iter().enumerate() {
        let row_h = row_heights.get(row_idx).copied().unwrap_or(0.0);
        let mut x = 0.0_f64;
        let mut col_idx: usize = 0;
        let row_cells_len = row.cells.len();
        for cell in &row.cells {
            if cell.h_merge || cell.v_merge {
                // Merge-continuation cells contribute neither fill nor
                // text — the master cell at the start of the span
                // already painted both. For borders we only need vMerge
                // continuations to emit their L/R edges, because the
                // master may sit one row above and not extend its
                // own L/R far enough by itself in some authored decks
                // (a rowSpan=15 anchor that only contributes one
                // left-edge segment, leaving a 14-row gap on the left
                // border). hMerge top/bottom are *not* re-emitted: the
                // master cell's lnT/lnB already covers the full grid
                // span, so re-drawing the continuation cell's lnT/lnB
                // produces a doubled stroke whenever the continuation
                // cell carries its own outline (e.g. slide 2's first
                // row, where columns 4-5 of an hMerge="1" run carry a
                // near-black 151617 lnT that overpainted the master's
                // light-gray DDDDDD top border).
                let cell_w = col_widths.get(col_idx).copied().unwrap_or(0.0);
                let cell_h = row_heights.get(row_idx).copied().unwrap_or(0.0);
                if let Some(borders) = &cell.borders {
                    if cell.v_merge {
                        emit_border(
                            &mut content,
                            &mut defs,
                            ids,
                            borders.left.as_ref(),
                            x,
                            y,
                            x,
                            y + cell_h,
                            stroke_scale,
                        );
                        emit_border(
                            &mut content,
                            &mut defs,
                            ids,
                            borders.right.as_ref(),
                            x + cell_w,
                            y,
                            x + cell_w,
                            y + cell_h,
                            stroke_scale,
                        );
                    }
                }
                x += cell_w;
                col_idx += 1;
                continue;
            }
            let cell_w = spanned_size(&col_widths, col_idx, cell.grid_span);
            let cell_h = spanned_size(&row_heights, row_idx, cell.row_span);

            // Inline cell fill wins; otherwise consult the resolved style
            // preset for header / banded / first-col tinting.
            let preset_fill = if cell.fill.is_some() {
                None
            } else {
                pick_preset_fill(
                    preset.as_ref(),
                    &opts,
                    row_idx,
                    col_idx,
                    row_count,
                    row_cells_len,
                )
            };
            let fill_attrs = if let Some(color) = preset_fill {
                // Skip render_fill_attrs entirely so the preset color isn't
                // paired with a redundant fill="none" (which resvg rejects).
                format!("fill=\"{color}\"")
            } else {
                let fr = render_fill_attrs(cell.fill.as_ref(), ids);
                if !fr.defs.is_empty() {
                    defs.push_str(&fr.defs);
                }
                fr.attrs
            };

            let _ = write!(
                content,
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" {fill_attrs}/>",
                n(x),
                n(y),
                n(cell_w),
                n(cell_h)
            );

            // Cell borders (top / bottom / left / right / diag down / diag up).
            if let Some(borders) = &cell.borders {
                emit_border(
                    &mut content,
                    &mut defs,
                    ids,
                    borders.top.as_ref(),
                    x,
                    y,
                    x + cell_w,
                    y,
                    stroke_scale,
                );
                emit_border(
                    &mut content,
                    &mut defs,
                    ids,
                    borders.bottom.as_ref(),
                    x,
                    y + cell_h,
                    x + cell_w,
                    y + cell_h,
                    stroke_scale,
                );
                emit_border(
                    &mut content,
                    &mut defs,
                    ids,
                    borders.left.as_ref(),
                    x,
                    y,
                    x,
                    y + cell_h,
                    stroke_scale,
                );
                emit_border(
                    &mut content,
                    &mut defs,
                    ids,
                    borders.right.as_ref(),
                    x + cell_w,
                    y,
                    x + cell_w,
                    y + cell_h,
                    stroke_scale,
                );
                emit_border(
                    &mut content,
                    &mut defs,
                    ids,
                    borders.diagonal_down.as_ref(),
                    x,
                    y,
                    x + cell_w,
                    y + cell_h,
                    stroke_scale,
                );
                emit_border(
                    &mut content,
                    &mut defs,
                    ids,
                    borders.diagonal_up.as_ref(),
                    x,
                    y + cell_h,
                    x + cell_w,
                    y,
                    stroke_scale,
                );
            }

            // Cell text body, translated into cell coordinate space.
            if let Some(text_body) = &cell.text_body {
                let cell_transform = Transform {
                    offset_x: Emu::new(0),
                    offset_y: Emu::new(0),
                    extent_width: pixels_to_emu(cell_w),
                    extent_height: pixels_to_emu(cell_h),
                    rotation: 0.0,
                    flip_h: false,
                    flip_v: false,
                };
                // PowerPoint dynamic centering: when no explicit tcPr@anchor
                // is set (parser left bodyPr.anchor at the spec default
                // "t") AND the content fits comfortably inside the cell,
                // PowerPoint visually centers the block. We mimic this by
                // promoting anchor T → Ctr only when the estimated content
                // height is below ~85 % of the cell height — overflowing
                // cells stay top-anchored so text doesn't slide out the
                // bottom edge. Cells with explicit anchor (Ctr / B set in
                // tcPr) are already handled by the parser and skipped here.
                let mut effective_body = text_body.clone();
                if matches!(
                    effective_body.body_properties.anchor,
                    slideglance_model::VerticalAnchor::T
                ) {
                    let est_h = estimate_cell_content_height(text_body, cell_w, measurer);
                    // Center whenever the content fits (any slack is enough).
                    // Overflowing content keeps top-anchored so it stays
                    // readable inside the cell instead of clipping off both
                    // top and bottom edges.
                    if est_h <= cell_h {
                        effective_body.body_properties.anchor =
                            slideglance_model::VerticalAnchor::Ctr;
                    }
                }
                let text_svg = render_text_body(
                    &effective_body,
                    &cell_transform,
                    slide,
                    script_fonts,
                    measurer,
                    mapping,
                    cjk_platform,
                    font_resolver,
                    true, // table cell — half leading delta
                    font_size_correction,
                );
                if !text_svg.is_empty() {
                    let _ = write!(
                        content,
                        "<g transform=\"translate({}, {})\">{text_svg}</g>",
                        n(x),
                        n(y)
                    );
                }
            }

            if let Some(w) = col_widths.get(col_idx) {
                x += *w;
            }
            col_idx += 1;
        }
        y += row_h;
    }

    content.push_str("</g>");
    TableElementResult { content, defs }
}

// Eight arguments mirror the per-edge `<line>` emission in TS: bundling
// (x1, y1, x2, y2) into a struct would only obscure the per-edge parity.
#[allow(clippy::too_many_arguments)]
fn emit_border(
    content: &mut String,
    defs: &mut String,
    ids: &mut IdGen,
    outline: Option<&slideglance_model::Outline>,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    stroke_scale: f64,
) {
    let Some(outline) = outline else { return };
    let res = render_outline_attrs(Some(outline), ids, stroke_scale);
    if !res.defs.is_empty() {
        defs.push_str(&res.defs);
    }
    let _ = write!(
        content,
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {}/>",
        n(x1),
        n(y1),
        n(x2),
        n(y2),
        res.attrs
    );
}

/// Conservative cell-content-height estimate used to grow `<a:tr h="...">`
/// rows when their authored minimum is smaller than what the cell actually
/// needs. We don't fully wrap-and-measure here — that would re-do work the
/// per-cell `render_text_body` does later — but counting paragraphs and
/// applying a per-paragraph natural line height plus the body insets is
/// enough to keep PowerPoint-style auto-grow behavior on text-heavy
/// tables.
///
/// `cell_w` is the cell's drawable width (already inset-corrected). Used
/// to add a single extra "wrapped line" allowance per paragraph whose
/// raw character count would clearly overflow a single line at the
/// resolved font size.
fn estimate_cell_content_height(
    text_body: &slideglance_model::TextBody,
    cell_w: f64,
    _measurer: &dyn slideglance_font::TextMeasurer,
) -> f64 {
    use slideglance_utils::Emu;
    // Pixels per typographic point (CSS 96 DPI).
    const PX_PER_PT: f64 = 96.0 / 72.0;
    // Heuristic kept until the OOXML cell-row sizing chain is fully
    // implemented; the measurer-driven precise variant regressed slide
    // 4 / 39 / 90 because cell-row sizing depends on more than the wrap
    // line count (table style row-height, equalize step, banding).
    // 1.2 is PowerPoint's default 120% line-height multiplier when the
    // paragraph itself doesn't carry an explicit `<a:lnSpc>`. Used as a
    // fallback only — paragraphs that DO declare lnSpc (slide 25's lower
    // body cell uses `spcPct val="150000"`, i.e. 150%) override this so
    // the row estimate matches the wrapped text actually emitted.
    const DEFAULT_LINE_SPACING: f64 = 1.2;
    const PARAGRAPH_GAP_PX: f64 = 1.5;

    let bp = &text_body.body_properties;
    let inset_top: f64 = Emu::to_pixels(bp.margin_top);
    let inset_bot: f64 = Emu::to_pixels(bp.margin_bottom);
    let inset_l: f64 = Emu::to_pixels(bp.margin_left);
    let inset_r: f64 = Emu::to_pixels(bp.margin_right);
    let usable_w = (cell_w - inset_l - inset_r).max(1.0);
    let mut total = inset_top + inset_bot;
    for (idx, para) in text_body.paragraphs.iter().enumerate() {
        let font_size_pt = para
            .runs
            .iter()
            .find_map(|r| r.properties.font_size.map(slideglance_utils::Pt::raw))
            .unwrap_or(18.0);
        // Honour the paragraph's own line-spacing factor when present.
        // OOXML stores `<a:lnSpc><a:spcPct val="150000"/>` as the raw
        // 150000 in our model — divide by 100 000 to get the 1.5 ×
        // multiplier semantics used downstream. Fall back to the 1.2
        // default otherwise. Percent values typically run 80 000 –
        // 200 000 (0.8 – 2.0 ×); cap at a sane upper bound so a
        // mis-parsed value doesn't blow up the row estimate.
        let line_factor = para
            .properties
            .line_spacing
            .map_or(DEFAULT_LINE_SPACING, |v| (v / 100_000.0).clamp(0.5, 3.0));
        let line_h = font_size_pt * line_factor * PX_PER_PT;
        let total_chars: usize = para.runs.iter().map(|r| r.text.chars().count()).sum();
        let avg_glyph_w = font_size_pt * 0.6 * PX_PER_PT;
        let chars_per_line = (usable_w / avg_glyph_w).floor().max(1.0) as usize;
        let lines = if total_chars == 0 {
            1
        } else {
            total_chars.div_ceil(chars_per_line)
        };
        total += line_h * lines as f64;
        // `<a:spcAft>` adds vertical space *after* every paragraph
        // except the last; honour an explicit Pts override and fall
        // back to the small constant gap when absent.
        if idx + 1 < text_body.paragraphs.len() {
            let gap = match para.properties.space_after {
                Some(slideglance_model::SpacingValue::Pts { value }) => {
                    (value.raw() as f64) / 100.0 * PX_PER_PT
                }
                Some(slideglance_model::SpacingValue::Pct { value }) => {
                    font_size_pt * value / 100_000.0 * PX_PER_PT
                }
                None => PARAGRAPH_GAP_PX,
            };
            total += gap;
        }
    }
    total
}

fn spanned_size(sizes: &[f64], start: usize, span: u32) -> f64 {
    let span = if span == 0 { 1 } else { span as usize };
    let end = (start + span).min(sizes.len());
    sizes[start..end].iter().copied().sum()
}

#[inline]
fn pixels_to_emu(px: f64) -> Emu {
    Emu::from_f64((px / 96.0) * 914_400.0)
}

// `&TableStyleOptions` is borrowed (it's `Copy` and 6 bytes, so clippy
// asks for value passing) — but the caller already borrows
// `table.table_style_options` for the entire loop duration, so passing by
// reference avoids unnecessary copies along the hot loop.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn pick_preset_fill(
    preset: Option<&TableStylePreset>,
    opts: &TableStyleOptions,
    row_idx: usize,
    col_idx: usize,
    row_count: usize,
    _col_count: usize,
) -> Option<&'static str> {
    let preset = preset?;
    let is_first_row = row_idx == 0;
    let is_last_row = row_count > 0 && row_idx == row_count - 1;
    let is_first_col = col_idx == 0;

    if opts.first_row.unwrap_or(false) && is_first_row {
        if let Some(fill) = preset.header_fill {
            return Some(fill);
        }
    }
    if opts.last_row.unwrap_or(false) && is_last_row {
        if let Some(fill) = preset.total_fill {
            return Some(fill);
        }
    }
    if opts.first_col.unwrap_or(false) && is_first_col && !is_first_row {
        if let Some(fill) = preset.first_col_fill {
            return Some(fill);
        }
    }
    if opts.band_row.unwrap_or(false) && row_idx > 0 && row_idx % 2 == 1 {
        if let Some(fill) = preset.band_row_fill {
            return Some(fill);
        }
    }
    None
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use slideglance_color::{ResolvedColor, Rgb};
    use slideglance_font::{FontMapping, HeuristicTextMeasurer};
    use slideglance_model::{
        BodyProperties, BulletType, CellBorders, DashStyle, Fill, NoFill, Outline, OutlineFill,
        Paragraph, ParagraphProperties, RunProperties, SolidFill, TableCell, TableColumn,
        TableData, TableRow, TextBody, TextRun, TextVerticalType, VerticalAnchor, WrapMode,
    };
    use slideglance_utils::Pt;

    fn outline(hex: &str) -> Outline {
        Outline {
            width: Emu::new(9_525),
            fill: Some(OutlineFill::Solid(SolidFill {
                color: ResolvedColor::new(Rgb::from_hex(hex).unwrap(), 1.0),
            })),
            dash_style: DashStyle::Solid,
            custom_dash: None,
            line_cap: None,
            line_join: None,
            head_end: None,
            tail_end: None,
        }
    }

    fn text_body(text: &str) -> TextBody {
        TextBody {
            default_text_color: None,
            paragraphs: vec![Paragraph {
                runs: vec![TextRun {
                    text: text.to_string(),
                    properties: RunProperties::default(),
                    field_type: None,
                }],
                properties: ParagraphProperties::default(),
                end_para_run_properties: None,
            }],
            body_properties: BodyProperties {
                anchor: VerticalAnchor::T,
                margin_left: Emu::new(0),
                margin_right: Emu::new(0),
                margin_top: Emu::new(0),
                margin_bottom: Emu::new(0),
                wrap: WrapMode::Square,
                auto_fit: slideglance_model::AutoFit::NoAutofit,
                font_scale: 1.0,
                ln_spc_reduction: 0.0,
                num_col: 1,
                vert: TextVerticalType::Horz,
                spc_first_last_para: false,
                compat_ln_spc: false,
                prst_tx_warp: None,
            },
        }
    }

    fn empty_cell() -> TableCell {
        TableCell {
            text_body: None,
            fill: None,
            borders: None,
            grid_span: 1,
            row_span: 1,
            h_merge: false,
            v_merge: false,
        }
    }

    fn render_helper(element: &TableElement) -> TableElementResult {
        let mut ids = IdGen::new();
        render_table(
            element,
            &mut ids,
            &SlideRenderContext::new(1),
            &ScriptFontContext::empty(),
            &HeuristicTextMeasurer,
            &FontMapping::new(),
            CjkPlatform::Other,
            None,
            1.0,
        )
    }

    fn one_cell_table(cell: TableCell) -> TableElement {
        TableElement {
            sp_id: None,
            transform: Transform {
                offset_x: Emu::new(0),
                offset_y: Emu::new(0),
                extent_width: Emu::new(914_400),
                extent_height: Emu::new(914_400),
                rotation: 0.0,
                flip_h: false,
                flip_v: false,
            },
            table: TableData {
                rows: vec![TableRow {
                    height: Emu::new(914_400),
                    cells: vec![cell],
                }],
                columns: vec![TableColumn {
                    width: Emu::new(914_400),
                }],
                table_style_id: None,
                table_style_options: None,
            },
            object_name: None,
            hidden: false,
        }
    }

    #[test]
    fn empty_table_emits_only_wrapper_group() {
        let mut t = one_cell_table(empty_cell());
        t.table.rows.clear();
        t.table.columns.clear();
        let res = render_helper(&t);
        assert!(res.content.starts_with("<g transform="));
        assert!(res.content.ends_with("</g>"));
        assert!(!res.content.contains("<rect"));
    }

    #[test]
    fn table_emits_data_sp_id_when_present() {
        let mut t = one_cell_table(empty_cell());
        t.sp_id = Some(99);
        let res = render_helper(&t);
        assert!(
            res.content.contains("data-sp-id=\"99\""),
            "data-sp-id missing: {}",
            res.content
        );
        // Attribute lands on the outer <g> immediately after the tag name.
        assert!(
            res.content.starts_with("<g data-sp-id=\"99\" transform=\""),
            "unexpected ordering: {}",
            res.content
        );
    }

    #[test]
    fn single_cell_emits_one_rect() {
        let res = render_helper(&one_cell_table(empty_cell()));
        assert_eq!(res.content.matches("<rect").count(), 1);
        assert!(res.content.contains("fill=\"none\""));
    }

    #[test]
    fn solid_fill_attrs_emit_in_rect() {
        let mut cell = empty_cell();
        cell.fill = Some(Fill::Solid(SolidFill {
            color: ResolvedColor::new(Rgb::from_hex("#FF0000").unwrap(), 1.0),
        }));
        let res = render_helper(&one_cell_table(cell));
        assert!(res.content.contains("fill=\"#FF0000\""));
    }

    #[test]
    fn no_fill_branch_emits_fill_none() {
        let mut cell = empty_cell();
        cell.fill = Some(Fill::None(NoFill::default()));
        let res = render_helper(&one_cell_table(cell));
        assert!(res.content.contains("fill=\"none\""));
    }

    #[test]
    fn cell_borders_emit_one_line_per_edge() {
        let mut cell = empty_cell();
        cell.borders = Some(CellBorders {
            top: Some(outline("#000000")),
            bottom: Some(outline("#000000")),
            left: Some(outline("#000000")),
            right: Some(outline("#000000")),
            diagonal_down: None,
            diagonal_up: None,
        });
        let res = render_helper(&one_cell_table(cell));
        assert_eq!(res.content.matches("<line").count(), 4);
    }

    #[test]
    fn diagonal_borders_emit_lines_too() {
        let mut cell = empty_cell();
        cell.borders = Some(CellBorders {
            top: None,
            bottom: None,
            left: None,
            right: None,
            diagonal_down: Some(outline("#FF0000")),
            diagonal_up: Some(outline("#00FF00")),
        });
        let res = render_helper(&one_cell_table(cell));
        assert_eq!(res.content.matches("<line").count(), 2);
        assert!(res.content.contains("stroke=\"#FF0000\""));
        assert!(res.content.contains("stroke=\"#00FF00\""));
    }

    #[test]
    fn cell_text_body_renders_inside_translate() {
        let mut cell = empty_cell();
        cell.text_body = Some(text_body("Hello"));
        let res = render_helper(&one_cell_table(cell));
        assert!(res.content.contains("<g transform=\"translate(0, 0)\">"));
        assert!(res.content.contains("Hello"));
    }

    #[test]
    fn merge_cells_skip_rendering_but_advance_x() {
        let mut left = empty_cell();
        left.fill = Some(Fill::Solid(SolidFill {
            color: ResolvedColor::new(Rgb::from_hex("#0000FF").unwrap(), 1.0),
        }));
        let right = TableCell {
            h_merge: true,
            ..empty_cell()
        };
        let element = TableElement {
            sp_id: None,
            transform: Transform {
                offset_x: Emu::new(0),
                offset_y: Emu::new(0),
                extent_width: Emu::new(1_828_800),
                extent_height: Emu::new(914_400),
                rotation: 0.0,
                flip_h: false,
                flip_v: false,
            },
            table: TableData {
                rows: vec![TableRow {
                    height: Emu::new(914_400),
                    cells: vec![left, right],
                }],
                columns: vec![
                    TableColumn {
                        width: Emu::new(914_400),
                    },
                    TableColumn {
                        width: Emu::new(914_400),
                    },
                ],
                table_style_id: None,
                table_style_options: None,
            },
            object_name: None,
            hidden: false,
        };
        let res = render_helper(&element);
        assert_eq!(res.content.matches("<rect").count(), 1);
        assert!(res.content.contains("fill=\"#0000FF\""));
    }

    #[test]
    fn grid_span_widens_cell() {
        let cell = TableCell {
            grid_span: 2,
            ..empty_cell()
        };
        let element = TableElement {
            sp_id: None,
            transform: Transform {
                offset_x: Emu::new(0),
                offset_y: Emu::new(0),
                extent_width: Emu::new(1_828_800),
                extent_height: Emu::new(914_400),
                rotation: 0.0,
                flip_h: false,
                flip_v: false,
            },
            table: TableData {
                rows: vec![TableRow {
                    height: Emu::new(914_400),
                    cells: vec![cell],
                }],
                columns: vec![
                    TableColumn {
                        width: Emu::new(914_400), // 96 px
                    },
                    TableColumn {
                        width: Emu::new(914_400), // 96 px
                    },
                ],
                table_style_id: None,
                table_style_options: None,
            },
            object_name: None,
            hidden: false,
        };
        let res = render_helper(&element);
        assert!(res.content.contains("width=\"192\""));
    }

    #[test]
    fn preset_header_fill_applied_when_first_row_opt_in() {
        let mut t = one_cell_table(empty_cell());
        // Two rows so header / non-header treatment is observable.
        t.table.rows.push(TableRow {
            height: Emu::new(914_400),
            cells: vec![empty_cell()],
        });
        t.table.table_style_id = Some("{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}".to_string());
        t.table.table_style_options = Some(TableStyleOptions {
            first_row: Some(true),
            ..TableStyleOptions::default()
        });
        let res = render_helper(&t);
        assert!(res.content.contains("fill=\"#4472C4\""));
    }

    #[test]
    fn preset_band_row_alternates_starting_at_row_one() {
        let mut t = one_cell_table(empty_cell());
        // Three rows: idx 0 plain, idx 1 banded, idx 2 plain.
        t.table.rows.push(TableRow {
            height: Emu::new(914_400),
            cells: vec![empty_cell()],
        });
        t.table.rows.push(TableRow {
            height: Emu::new(914_400),
            cells: vec![empty_cell()],
        });
        t.table.table_style_id = Some("{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}".to_string());
        t.table.table_style_options = Some(TableStyleOptions {
            band_row: Some(true),
            ..TableStyleOptions::default()
        });
        let res = render_helper(&t);
        assert!(res.content.contains("fill=\"#D9E2F3\""));
    }

    #[test]
    fn unknown_preset_id_doesnt_apply_tinting() {
        let mut t = one_cell_table(empty_cell());
        t.table.table_style_id = Some("not-a-real-guid".to_string());
        t.table.table_style_options = Some(TableStyleOptions {
            first_row: Some(true),
            ..TableStyleOptions::default()
        });
        let res = render_helper(&t);
        // Falls back to the default fill="none" since the preset wasn't
        // resolved.
        assert!(res.content.contains("fill=\"none\""));
    }

    #[test]
    fn pixels_to_emu_round_trip() {
        let e = pixels_to_emu(96.0);
        assert_eq!(e.raw(), 914_400);
    }

    #[test]
    fn empty_table_with_object_name_has_data_attr() {
        let mut t = one_cell_table(empty_cell());
        t.object_name = Some("Table 1".to_string());
        let res = render_helper(&t);
        assert!(res.content.contains("data-object-name=\"Table 1\""));
    }

    #[test]
    fn font_size_unused_warning_suppressed() {
        // Sanity: ensure `Pt` import is exercised so future helpers can use
        // it without churning the test imports.
        let _ = Pt::new(12.0);
        let _ = BulletType::None;
    }
}
