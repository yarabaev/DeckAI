//! `<a:tbl>` table parser.
//!
//! Mirrors.

use std::collections::BTreeMap;

use serde::Deserialize;
use slideglance_color::{ColorResolver, ResolvedColor, Rgb};
use slideglance_model::{
    CellBorders, DashStyle, FontScheme, Outline, OutlineFill, SolidFill, TableCell, TableColumn,
    TableData, TableRow, TableStyleOptions, VerticalAnchor,
};
use slideglance_utils::Emu;

use crate::fill::{build_fill, build_outline, FillParseContext, RawFillContainer, RawOutline};
use crate::relationships::Relationship;
use crate::text_body::{build_text_body, RawTextBody};
use crate::xml::{parse_xml, XmlError};

/// Parses an `<a:tbl>` body into a [`TableData`] structure.
///
/// Returns `Ok(None)` when the table has no `<a:tblGrid>` columns (TS
/// parity).
///
/// # Errors
///
/// Returns [`XmlError`] when the input XML is malformed.
pub fn parse_table(
    xml: &str,
    resolver: &ColorResolver,
    rels: Option<&BTreeMap<String, Relationship>>,
    font_scheme: Option<&FontScheme>,
) -> Result<Option<TableData>, XmlError> {
    let raw: RawTbl = parse_xml(xml)?;
    Ok(build_table(&raw, resolver, rels, font_scheme))
}

pub(crate) fn build_table(
    raw: &RawTbl,
    resolver: &ColorResolver,
    rels: Option<&BTreeMap<String, Relationship>>,
    font_scheme: Option<&FontScheme>,
) -> Option<TableData> {
    let columns = build_columns(raw.tbl_grid.as_ref());
    if columns.is_empty() {
        return None;
    }

    let table_style_id = raw
        .tbl_pr
        .as_ref()
        .and_then(|n| n.table_style_id.as_ref())
        .and_then(|n| n.text.clone())
        .filter(|s| !s.is_empty());
    let has_table_style = table_style_id.is_some();
    let default_borders = if has_table_style {
        Some(default_borders())
    } else {
        None
    };

    let table_style_options = raw.tbl_pr.as_ref().map(|n| TableStyleOptions {
        first_row: parse_optional_bool_attr(n.first_row.as_deref(), Some(true)),
        last_row: parse_optional_bool_attr(n.last_row.as_deref(), Some(false)),
        first_col: parse_optional_bool_attr(n.first_col.as_deref(), Some(false)),
        last_col: parse_optional_bool_attr(n.last_col.as_deref(), Some(false)),
        band_row: parse_optional_bool_attr(n.band_row.as_deref(), Some(true)),
        band_col: parse_optional_bool_attr(n.band_col.as_deref(), Some(false)),
    });

    let rows = build_rows(
        &raw.tr,
        resolver,
        rels,
        font_scheme,
        default_borders.as_ref(),
    );

    Some(TableData {
        rows,
        columns,
        table_style_id,
        table_style_options,
    })
}

fn build_columns(grid: Option<&RawTblGrid>) -> Vec<TableColumn> {
    grid.map(|g| {
        g.grid_col
            .iter()
            .map(|c| TableColumn {
                width: Emu::new(parse_attr_i64(c.w.as_deref(), 0)),
            })
            .collect()
    })
    .unwrap_or_default()
}

fn build_rows(
    rows: &[RawTr],
    resolver: &ColorResolver,
    rels: Option<&BTreeMap<String, Relationship>>,
    font_scheme: Option<&FontScheme>,
    default_borders: Option<&CellBorders>,
) -> Vec<TableRow> {
    rows.iter()
        .map(|tr| TableRow {
            height: Emu::new(parse_attr_i64(tr.h.as_deref(), 0)),
            cells: tr
                .tc
                .iter()
                .map(|tc| build_cell(tc, resolver, rels, font_scheme, default_borders))
                .collect(),
        })
        .collect()
}

fn build_cell(
    tc: &RawTc,
    resolver: &ColorResolver,
    rels: Option<&BTreeMap<String, Relationship>>,
    font_scheme: Option<&FontScheme>,
    default_borders: Option<&CellBorders>,
) -> TableCell {
    let mut text_body = tc
        .tx_body
        .as_ref()
        .and_then(|n| build_text_body(n, resolver, rels, font_scheme, None, None));

    let tc_pr = tc.tc_pr.as_ref();
    let fill = tc_pr
        .and_then(|p| p.fill.as_ref())
        .and_then(|f| build_fill_no_context(f, resolver));

    let inline_borders = tc_pr.and_then(|p| build_cell_borders(p, resolver));
    let borders = inline_borders.or_else(|| default_borders.cloned());

    let grid_span = tc
        .grid_span
        .as_deref()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(1);
    let row_span = tc
        .row_span
        .as_deref()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(1);
    let h_merge = tc
        .h_merge
        .as_deref()
        .is_some_and(|v| v == "1" || v == "true");
    let v_merge = tc
        .v_merge
        .as_deref()
        .is_some_and(|v| v == "1" || v == "true");

    // OOXML spec ST_TextAnchoringType (ECMA-376 Part 1 §20.1.10.59):
    // tcPr@anchor default = "t" (top). PowerPoint visually centers
    // small content in cells (slide 4 ToC) but that is *not* a default
    // anchor — it is dynamic centering applied when the content height
    // is below a threshold of the cell height. We honor the spec default
    // here and fall back to top when the slide / layout tcPr is silent;
    // dynamic centering belongs in the renderer, not the parser.
    if let (Some(body), Some(anchor)) =
        (text_body.as_mut(), tc_pr.and_then(|p| p.anchor.as_deref()))
    {
        if let Some(parsed) = parse_cell_anchor(anchor) {
            body.body_properties.anchor = parsed;
        }
    }
    // OOXML cell padding lives on `<a:tcPr marL/marR/marT/marB>` not
    // on the inner `<a:bodyPr>`. Without this transfer the renderer
    // (and the table-row auto-grow estimator) defaults every cell to
    // PowerPoint's textbox-grade 91440 EMU (~9.6 px) inset, far
    // larger than the ~1 px insets common in dense data tables —
    // which inflated table heights two- to three-fold for slides 2/3/4
    // of the test deck.
    if let (Some(body), Some(p)) = (text_body.as_mut(), tc_pr) {
        let parse = |s: &Option<String>| -> Option<i64> {
            s.as_deref().and_then(|v| v.parse::<i64>().ok())
        };
        if let Some(v) = parse(&p.mar_l) {
            body.body_properties.margin_left = slideglance_utils::Emu::new(v);
        }
        if let Some(v) = parse(&p.mar_r) {
            body.body_properties.margin_right = slideglance_utils::Emu::new(v);
        }
        if let Some(v) = parse(&p.mar_t) {
            body.body_properties.margin_top = slideglance_utils::Emu::new(v);
        }
        if let Some(v) = parse(&p.mar_b) {
            body.body_properties.margin_bottom = slideglance_utils::Emu::new(v);
        }
    }

    TableCell {
        text_body,
        fill,
        borders,
        grid_span,
        row_span,
        h_merge,
        v_merge,
    }
}

fn build_cell_borders(tc_pr: &RawTcPr, resolver: &ColorResolver) -> Option<CellBorders> {
    let top = tc_pr.ln_t.as_ref().and_then(|n| build_outline(n, resolver));
    let bottom = tc_pr.ln_b.as_ref().and_then(|n| build_outline(n, resolver));
    let left = tc_pr.ln_l.as_ref().and_then(|n| build_outline(n, resolver));
    let right = tc_pr.ln_r.as_ref().and_then(|n| build_outline(n, resolver));
    let diagonal_down = tc_pr
        .ln_tl_to_br
        .as_ref()
        .and_then(|n| build_outline(n, resolver));
    let diagonal_up = tc_pr
        .ln_bl_to_tr
        .as_ref()
        .and_then(|n| build_outline(n, resolver));

    // OOXML cells can opt out of borders explicitly with
    // `<a:lnL w="..."><a:noFill/></a:lnL>` — `build_outline` returns
    // `None` for that case (and for absent nodes). To distinguish "I
    // explicitly disabled this side" from "I left this side to the
    // table style default", check whether the raw node was present at
    // all. If *any* side was specified inline, the cell decided its
    // borders and the caller must skip the default-style fallback,
    // even when every specified side resolves to `None` (all-noFill).
    let any_inline = tc_pr.ln_t.is_some()
        || tc_pr.ln_b.is_some()
        || tc_pr.ln_l.is_some()
        || tc_pr.ln_r.is_some()
        || tc_pr.ln_tl_to_br.is_some()
        || tc_pr.ln_bl_to_tr.is_some();
    if !any_inline {
        return None;
    }

    let mut out = CellBorders {
        top,
        bottom,
        left,
        right,
        diagonal_down,
        diagonal_up,
    };
    // Mirror TS: any border without an explicit fill defaults to opaque black
    // so the renderer can stroke it.
    let default_fill = OutlineFill::Solid(SolidFill {
        color: ResolvedColor::opaque(Rgb::new(0, 0, 0)),
    });
    for slot in [
        &mut out.top,
        &mut out.bottom,
        &mut out.left,
        &mut out.right,
        &mut out.diagonal_down,
        &mut out.diagonal_up,
    ] {
        if let Some(border) = slot.as_mut() {
            if border.fill.is_none() {
                border.fill = Some(default_fill.clone());
            }
        }
    }
    Some(out)
}

fn default_borders() -> CellBorders {
    let outline = Outline {
        width: Emu::new(12_700),
        fill: Some(OutlineFill::Solid(SolidFill {
            color: ResolvedColor::opaque(Rgb::new(0, 0, 0)),
        })),
        dash_style: DashStyle::Solid,
        custom_dash: None,
        line_cap: None,
        line_join: None,
        head_end: None,
        tail_end: None,
    };
    CellBorders {
        top: Some(outline.clone()),
        bottom: Some(outline.clone()),
        left: Some(outline.clone()),
        right: Some(outline),
        diagonal_down: None,
        diagonal_up: None,
    }
}

fn build_fill_no_context(
    fill: &RawFillContainer,
    resolver: &ColorResolver,
) -> Option<slideglance_model::Fill> {
    // Cell fills don't have access to the archive, so blipFill / grpFill
    // falls back to None — matching TS behavior.
    let context: Option<&mut FillParseContext<'_>> = None;
    build_fill(fill, resolver, context)
}

fn parse_cell_anchor(s: &str) -> Option<VerticalAnchor> {
    match s {
        "t" => Some(VerticalAnchor::T),
        "ctr" => Some(VerticalAnchor::Ctr),
        "b" => Some(VerticalAnchor::B),
        _ => None,
    }
}

fn parse_optional_bool_attr(s: Option<&str>, default: Option<bool>) -> Option<bool> {
    match s {
        Some("0" | "false") => Some(false),
        Some("1" | "true") => Some(true),
        Some(_) | None => default,
    }
}

fn parse_attr_i64(s: Option<&str>, default: i64) -> i64 {
    s.and_then(|v| v.parse::<i64>().ok()).unwrap_or(default)
}

// --- raw XML shapes ---

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawTbl {
    #[serde(rename = "tblPr")]
    pub tbl_pr: Option<RawTblPr>,
    #[serde(rename = "tblGrid")]
    pub tbl_grid: Option<RawTblGrid>,
    #[serde(default)]
    pub tr: Vec<RawTr>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawTblPr {
    #[serde(rename = "@firstRow")]
    pub first_row: Option<String>,
    #[serde(rename = "@lastRow")]
    pub last_row: Option<String>,
    #[serde(rename = "@firstCol")]
    pub first_col: Option<String>,
    #[serde(rename = "@lastCol")]
    pub last_col: Option<String>,
    #[serde(rename = "@bandRow")]
    pub band_row: Option<String>,
    #[serde(rename = "@bandCol")]
    pub band_col: Option<String>,
    #[serde(rename = "tableStyleId")]
    pub table_style_id: Option<RawText>,
}

/// A leaf text element where we only care about the body. Used for
/// `<a:tableStyleId>{guid}</a:tableStyleId>`.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawText {
    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawTblGrid {
    #[serde(default, rename = "gridCol")]
    pub grid_col: Vec<RawGridCol>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawGridCol {
    #[serde(rename = "@w")]
    pub w: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawTr {
    #[serde(rename = "@h")]
    pub h: Option<String>,
    #[serde(default)]
    pub tc: Vec<RawTc>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawTc {
    #[serde(rename = "@gridSpan")]
    pub grid_span: Option<String>,
    #[serde(rename = "@rowSpan")]
    pub row_span: Option<String>,
    #[serde(rename = "@hMerge")]
    pub h_merge: Option<String>,
    #[serde(rename = "@vMerge")]
    pub v_merge: Option<String>,
    #[serde(rename = "txBody")]
    pub tx_body: Option<RawTextBody>,
    #[serde(rename = "tcPr")]
    pub tc_pr: Option<RawTcPr>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawTcPr {
    #[serde(rename = "@anchor")]
    pub anchor: Option<String>,
    #[serde(rename = "@marL")]
    pub mar_l: Option<String>,
    #[serde(rename = "@marR")]
    pub mar_r: Option<String>,
    #[serde(rename = "@marT")]
    pub mar_t: Option<String>,
    #[serde(rename = "@marB")]
    pub mar_b: Option<String>,
    #[serde(flatten)]
    pub fill: Option<RawFillContainer>,
    #[serde(rename = "lnT")]
    pub ln_t: Option<RawOutline>,
    #[serde(rename = "lnB")]
    pub ln_b: Option<RawOutline>,
    #[serde(rename = "lnL")]
    pub ln_l: Option<RawOutline>,
    #[serde(rename = "lnR")]
    pub ln_r: Option<RawOutline>,
    #[serde(rename = "lnTlToBr")]
    pub ln_tl_to_br: Option<RawOutline>,
    #[serde(rename = "lnBlToTr")]
    pub ln_bl_to_tr: Option<RawOutline>,
}
