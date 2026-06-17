//! `<p:sldMaster>` slide-master XML parser.
//!
//! Mirrors. Returns an
//! aggregated [`SlideMaster`] (color-map / background / elements /
//! txStyles / placeholder styles) from a single deserialization pass —
//! the spec exposes one function per piece, but the Rust port
//! groups them so the shared XML deserialization isn't repeated.

use std::collections::BTreeMap;
use std::str::FromStr;

use serde::Deserialize;
use slideglance_color::{ColorMap, ColorResolver, SchemeColorKey};
use slideglance_model::{FontScheme, FormatScheme, PlaceholderStyleInfo, SlideMaster, TxStyles};

use crate::archive::PptxArchive;
use crate::relationships::{build_rels_path, parse_relationships};
use crate::shape_geometry::{build_geometry_parts, build_transform};
use crate::slide::{
    build_background, build_sp_tree_elements, RawCSld, RawShapeSpPr, RawSp, SpTreeChild,
};
use crate::text_body::RawTextBody;
use crate::text_style::{build_list_style, RawListStyle};
use crate::xml::{parse_xml, XmlError};

/// Parses a `<p:sldMaster>` XML body and aggregates all of its parsed pieces
/// into a [`SlideMaster`] model.
///
/// `master_path` is the archive-relative path of the master part (e.g.
/// `"ppt/slideMasters/slideMaster1.xml"`); its `_rels/*.rels` companion is
/// loaded automatically when present.
///
/// # Errors
///
/// Returns [`XmlError`] when the input XML or the master's `.rels` is not
/// well-formed.
pub fn parse_slide_master(
    xml: &str,
    master_path: &str,
    archive: &mut PptxArchive,
    resolver: &ColorResolver,
    font_scheme: Option<&FontScheme>,
    fmt_scheme: Option<&FormatScheme>,
) -> Result<SlideMaster, XmlError> {
    let raw: RawSlideMaster = parse_xml(xml)?;
    let rels_path = build_rels_path(master_path);
    let rels = match archive.xml(&rels_path) {
        Some(rels_xml) => parse_relationships(rels_xml)?,
        None => BTreeMap::new(),
    };

    let color_map = raw
        .clr_map
        .as_ref()
        .map(build_color_map)
        .unwrap_or_default();

    let background = raw
        .c_sld
        .as_ref()
        .and_then(|c| c.bg.as_ref())
        .and_then(|bg| build_background(bg, resolver, &rels, master_path, archive));

    let empty_children: Vec<SpTreeChild> = Vec::new();
    let sp_tree_children: &[SpTreeChild] = raw
        .c_sld
        .as_ref()
        .and_then(|c| c.sp_tree.as_ref())
        .map_or(empty_children.as_slice(), |t| t.children.as_slice());

    let elements = build_sp_tree_elements(
        sp_tree_children,
        &rels,
        master_path,
        archive,
        resolver,
        font_scheme,
        fmt_scheme,
        None,
        None,
    );

    let tx_styles = raw
        .tx_styles
        .as_ref()
        .and_then(|n| build_tx_styles(n, resolver));
    let placeholder_styles = collect_placeholder_styles(sp_tree_children, resolver);

    Ok(SlideMaster {
        color_map,
        background,
        elements,
        tx_styles,
        placeholder_styles,
    })
}

fn build_color_map(raw: &RawClrMap) -> ColorMap {
    let parse = |s: Option<&str>, default: SchemeColorKey| -> SchemeColorKey {
        s.and_then(|v| SchemeColorKey::from_str(v).ok())
            .unwrap_or(default)
    };
    ColorMap {
        bg1: parse(raw.bg1.as_deref(), SchemeColorKey::Lt1),
        tx1: parse(raw.tx1.as_deref(), SchemeColorKey::Dk1),
        bg2: parse(raw.bg2.as_deref(), SchemeColorKey::Lt2),
        tx2: parse(raw.tx2.as_deref(), SchemeColorKey::Dk2),
        accent1: parse(raw.accent1.as_deref(), SchemeColorKey::Accent1),
        accent2: parse(raw.accent2.as_deref(), SchemeColorKey::Accent2),
        accent3: parse(raw.accent3.as_deref(), SchemeColorKey::Accent3),
        accent4: parse(raw.accent4.as_deref(), SchemeColorKey::Accent4),
        accent5: parse(raw.accent5.as_deref(), SchemeColorKey::Accent5),
        accent6: parse(raw.accent6.as_deref(), SchemeColorKey::Accent6),
        hlink: parse(raw.hlink.as_deref(), SchemeColorKey::Hlink),
        fol_hlink: parse(raw.fol_hlink.as_deref(), SchemeColorKey::FolHlink),
    }
}

fn build_tx_styles(raw: &RawTxStyles, resolver: &ColorResolver) -> Option<TxStyles> {
    let title_style = raw
        .title_style
        .as_ref()
        .and_then(|n| build_list_style(n, Some(resolver)));
    let body_style = raw
        .body_style
        .as_ref()
        .and_then(|n| build_list_style(n, Some(resolver)));
    let other_style = raw
        .other_style
        .as_ref()
        .and_then(|n| build_list_style(n, Some(resolver)));
    if title_style.is_none() && body_style.is_none() && other_style.is_none() {
        return None;
    }
    Some(TxStyles {
        title_style,
        body_style,
        other_style,
    })
}

/// Walk the spTree's top-level shapes, picking out every `<p:sp>` that
/// declares a `<p:ph>`. Mirrors TS `parseSlideMasterPlaceholderStyles` /
/// `parseSlideLayoutPlaceholderStyles`. Reused by the layout parser via
/// [`crate::slide_layout::parse_slide_layout`].
pub(crate) fn collect_placeholder_styles(
    children: &[SpTreeChild],
    resolver: &ColorResolver,
) -> Vec<PlaceholderStyleInfo> {
    let mut results = Vec::new();
    for child in children {
        if let SpTreeChild::Sp(sp) = child {
            if let Some(info) = build_placeholder_info(sp, resolver) {
                results.push(info);
            }
        }
    }
    results
}

fn build_placeholder_info(sp: &RawSp, resolver: &ColorResolver) -> Option<PlaceholderStyleInfo> {
    let nv_sp_pr = sp.nv_sp_pr.as_ref()?;
    let nv_pr = nv_sp_pr.nv_pr.as_ref()?;
    let ph = nv_pr.ph.as_ref()?;

    let placeholder_type = ph.ty.clone().unwrap_or_else(|| "body".to_owned());
    let placeholder_idx = ph.idx.as_deref().and_then(|s| s.parse::<u32>().ok());
    let lst_style = sp.tx_body.as_ref().and_then(|tb: &RawTextBody| {
        // build_text_body owns lstStyle parsing; we only need the
        // lstStyle slot to surface inheritance, so we read it via
        // build_text_body and pull out the body's underlying style.
        // Cheaper: parse the lstStyle directly from txBody.lst_style.
        tb.lst_style
            .as_ref()
            .and_then(|n: &RawListStyle| build_list_style(n, Some(resolver)))
    });
    // Layout/master placeholder bodyPr — only the **explicit** attributes
    // (sparse `Option` fields) are carried so the slide-side merge can
    // tell "layout said anchor=ctr" apart from "layout was silent" and
    // skip the latter. MS-OE376 §5.1.5.1.1 specifies this field-wise
    // resolution chain explicitly.
    let body_properties = sp.tx_body.as_ref().and_then(|tb: &RawTextBody| {
        crate::text_body::build_placeholder_body_pr(tb.body_pr.as_ref())
    });

    let transform = sp
        .sp_pr
        .as_ref()
        .and_then(|sp_pr: &RawShapeSpPr| sp_pr.xfrm.as_ref())
        .and_then(build_transform);
    let geometry = sp.sp_pr.as_ref().map(|sp_pr: &RawShapeSpPr| {
        build_geometry_parts(sp_pr.prst_geom.as_ref(), sp_pr.cust_geom.as_ref())
    });

    Some(PlaceholderStyleInfo {
        placeholder_type,
        placeholder_idx,
        lst_style,
        body_properties,
        transform,
        geometry,
    })
}

// === Raw XML types ======================================================

#[derive(Debug, Default, Deserialize)]
struct RawSlideMaster {
    #[serde(rename = "cSld")]
    c_sld: Option<RawCSld>,
    #[serde(rename = "clrMap")]
    clr_map: Option<RawClrMap>,
    #[serde(rename = "txStyles")]
    tx_styles: Option<RawTxStyles>,
}

#[derive(Debug, Default, Deserialize)]
struct RawClrMap {
    #[serde(rename = "@bg1")]
    bg1: Option<String>,
    #[serde(rename = "@tx1")]
    tx1: Option<String>,
    #[serde(rename = "@bg2")]
    bg2: Option<String>,
    #[serde(rename = "@tx2")]
    tx2: Option<String>,
    #[serde(rename = "@accent1")]
    accent1: Option<String>,
    #[serde(rename = "@accent2")]
    accent2: Option<String>,
    #[serde(rename = "@accent3")]
    accent3: Option<String>,
    #[serde(rename = "@accent4")]
    accent4: Option<String>,
    #[serde(rename = "@accent5")]
    accent5: Option<String>,
    #[serde(rename = "@accent6")]
    accent6: Option<String>,
    #[serde(rename = "@hlink")]
    hlink: Option<String>,
    #[serde(rename = "@folHlink")]
    fol_hlink: Option<String>,
}

// Each field tracks one of the OOXML style stacks (`titleStyle`, `bodyStyle`,
// `otherStyle`); the shared `_style` postfix mirrors the source-XML naming
// and is intentional — same convention as the spec `parseTxStyles`.
#[allow(clippy::struct_field_names)]
#[derive(Debug, Default, Deserialize)]
struct RawTxStyles {
    #[serde(rename = "titleStyle")]
    title_style: Option<RawListStyle>,
    #[serde(rename = "bodyStyle")]
    body_style: Option<RawListStyle>,
    #[serde(rename = "otherStyle")]
    other_style: Option<RawListStyle>,
}
