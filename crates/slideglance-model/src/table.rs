//! Table element placed inside a `<p:graphicFrame>`.

use slideglance_utils::Emu;

use crate::fill::Fill;
use crate::line::Outline;
use crate::shape::Transform;
use crate::text::TextBody;

/// Table-bearing graphic frame.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TableElement {
    /// `<p:cNvPr @id>` — slide-scoped unique identifier from the source PPTX.
    /// `None` for legacy decks that omit the attribute.
    pub sp_id: Option<u32>,
    /// Position / size / rotation.
    pub transform: Transform,
    /// Table data (rows / columns / cells).
    pub table: TableData,
    /// `<p:cNvPr @name>`.
    pub object_name: Option<String>,
    /// `<p:cNvPr @hidden>`.
    pub hidden: bool,
}

/// `<a:tbl>` content.
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TableData {
    /// Rows (`<a:tr>`) in source order.
    pub rows: Vec<TableRow>,
    /// Column descriptors (`<a:gridCol>`).
    pub columns: Vec<TableColumn>,
    /// Built-in `PowerPoint` table style GUID (`<a:tableStyleId>`).
    pub table_style_id: Option<String>,
    /// Per-table style toggles (`<a:tblPr>`).
    pub table_style_options: Option<TableStyleOptions>,
}

/// One row.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TableRow {
    /// Row height (EMU).
    pub height: Emu,
    /// Cells in this row, in source order.
    pub cells: Vec<TableCell>,
}

/// One column descriptor.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TableColumn {
    /// Column width (EMU).
    pub width: Emu,
}

/// One cell (`<a:tc>`).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TableCell {
    /// Cell text body.
    pub text_body: Option<TextBody>,
    /// Cell fill.
    pub fill: Option<Fill>,
    /// Cell borders.
    pub borders: Option<CellBorders>,
    /// `<a:tc @gridSpan>` — number of columns this cell spans.
    pub grid_span: u32,
    /// `<a:tc @rowSpan>`.
    pub row_span: u32,
    /// `<a:tc @hMerge>` — this cell is the right side of a horizontal merge.
    pub h_merge: bool,
    /// `<a:tc @vMerge>` — bottom side of a vertical merge.
    pub v_merge: bool,
}

/// `<a:tblPr>` style toggles.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TableStyleOptions {
    /// First-row banding.
    pub first_row: Option<bool>,
    /// Last-row banding.
    pub last_row: Option<bool>,
    /// First-column banding.
    pub first_col: Option<bool>,
    /// Last-column banding.
    pub last_col: Option<bool>,
    /// Row banding.
    pub band_row: Option<bool>,
    /// Column banding.
    pub band_col: Option<bool>,
}

/// `<a:tcBdr>` borders, all six edges.
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CellBorders {
    /// `<a:lnT>`.
    pub top: Option<Outline>,
    /// `<a:lnB>`.
    pub bottom: Option<Outline>,
    /// `<a:lnL>`.
    pub left: Option<Outline>,
    /// `<a:lnR>`.
    pub right: Option<Outline>,
    /// `<a:lnTlToBr>` — diagonal top-left to bottom-right.
    pub diagonal_down: Option<Outline>,
    /// `<a:lnBlToTr>` — diagonal bottom-left to top-right.
    pub diagonal_up: Option<Outline>,
}
