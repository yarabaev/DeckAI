//! Built-in `PowerPoint` table-style preset table.
//!
//! Direct port of. The
//! GUIDs reference well-known built-in styles that `PowerPoint` clients
//! recognize without shipping the matching `tableStyles.xml` part — we
//! hard-code the colors so downstream rendering matches typical decks
//! instead of falling through to a plain black grid.

/// Hex (`#RRGGBB`) string presets for one named table style.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TableStylePreset {
    /// Header row fill (`tableStyleOptions.firstRow`).
    pub header_fill: Option<&'static str>,
    /// Header row text color.
    pub header_color: Option<&'static str>,
    /// Total / last row fill (`tableStyleOptions.lastRow`).
    pub total_fill: Option<&'static str>,
    /// Total row top border, rendered as solid 1pt.
    pub total_border: Option<&'static str>,
    /// Even-row banding fill (`tableStyleOptions.bandRow`).
    pub band_row_fill: Option<&'static str>,
    /// First-column emphasis fill (`tableStyleOptions.firstCol`).
    pub first_col_fill: Option<&'static str>,
    /// First-column text color.
    pub first_col_color: Option<&'static str>,
    /// Last-column emphasis fill (`tableStyleOptions.lastCol`).
    pub last_col_fill: Option<&'static str>,
    /// Default cell text color.
    pub text_color: Option<&'static str>,
    /// Inner border color (between cells).
    pub border_color: Option<&'static str>,
}

/// Look up a preset by `<a:tableStyleId>`. Returns `None` when the GUID
/// isn't recognized; the caller then renders without auto-applied tinting.
///
/// Brace and case normalization happens here so callers don't need to
/// massage the input.
#[must_use]
pub fn lookup_table_style_preset(table_style_id: Option<&str>) -> Option<TableStylePreset> {
    let id = table_style_id?;
    let normalized: String = id
        .chars()
        .filter(|c| *c != '{' && *c != '}')
        .flat_map(char::to_uppercase)
        .collect();
    preset_for(&normalized)
}

// The match arms intentionally repeat `..TableStylePreset::default()` and
// in some cases have identical color tables (e.g. Themed Style 1 mirrors
// Medium Style 2 / Accent 1) — the GUID-to-colors mapping is the source of
// truth and merging arms by formula would obscure the parity table.
#[allow(clippy::match_same_arms)]
fn preset_for(normalized: &str) -> Option<TableStylePreset> {
    match normalized {
        // Medium Style 2 — Accent 1 (most common default).
        "5C22544A-7EE6-4342-B048-85BDC9FD1C3A" => Some(TableStylePreset {
            header_fill: Some("#4472C4"),
            header_color: Some("#FFFFFF"),
            band_row_fill: Some("#D9E2F3"),
            border_color: Some("#8FAADC"),
            text_color: Some("#000000"),
            ..TableStylePreset::default()
        }),
        // Medium Style 2 — Accent 2.
        "21E4AEA4-8DFA-4A89-87EB-49C32662AFE0" => Some(TableStylePreset {
            header_fill: Some("#ED7D31"),
            header_color: Some("#FFFFFF"),
            band_row_fill: Some("#FBE5D6"),
            border_color: Some("#F4B183"),
            text_color: Some("#000000"),
            ..TableStylePreset::default()
        }),
        // Medium Style 2 — Accent 3.
        "F2DE63D5-997A-4646-A377-4702673A728D" => Some(TableStylePreset {
            header_fill: Some("#A5A5A5"),
            header_color: Some("#FFFFFF"),
            band_row_fill: Some("#EDEDED"),
            border_color: Some("#D9D9D9"),
            text_color: Some("#000000"),
            ..TableStylePreset::default()
        }),
        // Medium Style 2 — Accent 4.
        "17292A2E-F333-43FB-9621-5CBBE7FDCDCB" => Some(TableStylePreset {
            header_fill: Some("#FFC000"),
            header_color: Some("#000000"),
            band_row_fill: Some("#FFF2CC"),
            border_color: Some("#FFD966"),
            text_color: Some("#000000"),
            ..TableStylePreset::default()
        }),
        // Medium Style 2 — Accent 5.
        "5DA37D80-6434-44D0-A028-1B22A696006F" => Some(TableStylePreset {
            header_fill: Some("#5B9BD5"),
            header_color: Some("#FFFFFF"),
            band_row_fill: Some("#DEEBF7"),
            border_color: Some("#9DC3E6"),
            text_color: Some("#000000"),
            ..TableStylePreset::default()
        }),
        // Medium Style 2 — Accent 6.
        "8EC20E35-A176-4012-BC5E-935CFFF8708E" => Some(TableStylePreset {
            header_fill: Some("#70AD47"),
            header_color: Some("#FFFFFF"),
            band_row_fill: Some("#E2EFDA"),
            border_color: Some("#A9D08E"),
            text_color: Some("#000000"),
            ..TableStylePreset::default()
        }),
        // Light Style 1 — Accent 1.
        "9D7B26C5-B5C8-4FB0-AAF6-71C5C7B8D8C5" => Some(TableStylePreset {
            header_color: Some("#4472C4"),
            band_row_fill: Some("#D9E2F3"),
            border_color: Some("#4472C4"),
            text_color: Some("#000000"),
            ..TableStylePreset::default()
        }),
        // Themed Style 1 — Accent 1.
        "D27102A9-8310-4765-A935-A1911B00CA55" => Some(TableStylePreset {
            header_fill: Some("#4472C4"),
            header_color: Some("#FFFFFF"),
            border_color: Some("#FFFFFF"),
            text_color: Some("#000000"),
            band_row_fill: Some("#D9E2F3"),
            ..TableStylePreset::default()
        }),
        // No Style — Table Grid.
        "2D5ABB26-0587-4C30-8999-92F81FD0307C" => Some(TableStylePreset {
            border_color: Some("#000000"),
            text_color: Some("#000000"),
            ..TableStylePreset::default()
        }),
        // Default fallback for unknown GUIDs when style options opt in.
        "DEFAULT" => Some(TableStylePreset {
            header_fill: Some("#4472C4"),
            header_color: Some("#FFFFFF"),
            band_row_fill: Some("#D9E2F3"),
            border_color: Some("#8FAADC"),
            text_color: Some("#000000"),
            ..TableStylePreset::default()
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_id_returns_none() {
        assert!(lookup_table_style_preset(None).is_none());
    }

    #[test]
    fn unknown_id_returns_none() {
        assert!(lookup_table_style_preset(Some("not-a-guid")).is_none());
    }

    #[test]
    fn brace_and_case_normalization() {
        let lower = lookup_table_style_preset(Some("{5c22544a-7ee6-4342-b048-85bdc9fd1c3a}"));
        let upper = lookup_table_style_preset(Some("5C22544A-7EE6-4342-B048-85BDC9FD1C3A"));
        assert_eq!(lower, upper);
        assert_eq!(lower.unwrap().header_fill, Some("#4472C4"));
    }

    #[test]
    fn medium_style_2_accent_1_colors() {
        let p = lookup_table_style_preset(Some("5C22544A-7EE6-4342-B048-85BDC9FD1C3A")).unwrap();
        assert_eq!(p.header_fill, Some("#4472C4"));
        assert_eq!(p.band_row_fill, Some("#D9E2F3"));
        assert_eq!(p.border_color, Some("#8FAADC"));
    }

    #[test]
    fn no_style_table_grid_has_borders_only() {
        let p = lookup_table_style_preset(Some("2D5ABB26-0587-4C30-8999-92F81FD0307C")).unwrap();
        assert!(p.header_fill.is_none());
        assert_eq!(p.border_color, Some("#000000"));
    }

    #[test]
    fn default_preset_resolves() {
        let p = lookup_table_style_preset(Some("DEFAULT")).unwrap();
        assert_eq!(p.header_fill, Some("#4472C4"));
    }
}
