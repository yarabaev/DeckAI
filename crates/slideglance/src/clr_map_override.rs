//! `<p:clrMapOvr>` / `<p:overrideClrMapping>` parser used by both slides
//! and slide layouts. Mirrors.
//!
//! Returns `None` when no override is declared (`<p:masterClrMapping/>`
//! is treated as "no override" — the master's color map applies as-is).

use std::str::FromStr;

use serde::Deserialize;
use slideglance_color::SchemeColorKey;

use slideglance_parser::parse_xml;

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ColorMapOverride {
    pub bg1: Option<SchemeColorKey>,
    pub tx1: Option<SchemeColorKey>,
    pub bg2: Option<SchemeColorKey>,
    pub tx2: Option<SchemeColorKey>,
    pub accent1: Option<SchemeColorKey>,
    pub accent2: Option<SchemeColorKey>,
    pub accent3: Option<SchemeColorKey>,
    pub accent4: Option<SchemeColorKey>,
    pub accent5: Option<SchemeColorKey>,
    pub accent6: Option<SchemeColorKey>,
    pub hlink: Option<SchemeColorKey>,
    pub fol_hlink: Option<SchemeColorKey>,
}

/// Parse a `<p:sld>` or `<p:sldLayout>` body and pull out a
/// [`ColorMapOverride`] when present. Cheap pre-check on the raw text
/// avoids paying for the XML parse on the common no-override case.
pub(crate) fn parse_clr_map_override(xml: &str) -> Option<ColorMapOverride> {
    if !xml.contains("clrMapOvr") {
        return None;
    }
    let raw: RawRoot = parse_xml(xml).ok()?;
    let clr_map_ovr = raw.clr_map_ovr.as_ref()?;
    if clr_map_ovr.master_clr_mapping.is_some() {
        // `<p:masterClrMapping/>` — explicitly use the master's mapping.
        return None;
    }
    let override_node = clr_map_ovr.override_clr_mapping.as_ref()?;
    let result = ColorMapOverride {
        bg1: parse_key(override_node.bg1.as_deref()),
        tx1: parse_key(override_node.tx1.as_deref()),
        bg2: parse_key(override_node.bg2.as_deref()),
        tx2: parse_key(override_node.tx2.as_deref()),
        accent1: parse_key(override_node.accent1.as_deref()),
        accent2: parse_key(override_node.accent2.as_deref()),
        accent3: parse_key(override_node.accent3.as_deref()),
        accent4: parse_key(override_node.accent4.as_deref()),
        accent5: parse_key(override_node.accent5.as_deref()),
        accent6: parse_key(override_node.accent6.as_deref()),
        hlink: parse_key(override_node.hlink.as_deref()),
        fol_hlink: parse_key(override_node.fol_hlink.as_deref()),
    };
    if has_any_override(&result) {
        Some(result)
    } else {
        None
    }
}

fn parse_key(s: Option<&str>) -> Option<SchemeColorKey> {
    s.and_then(|v| SchemeColorKey::from_str(v).ok())
}

fn has_any_override(o: &ColorMapOverride) -> bool {
    o.bg1.is_some()
        || o.tx1.is_some()
        || o.bg2.is_some()
        || o.tx2.is_some()
        || o.accent1.is_some()
        || o.accent2.is_some()
        || o.accent3.is_some()
        || o.accent4.is_some()
        || o.accent5.is_some()
        || o.accent6.is_some()
        || o.hlink.is_some()
        || o.fol_hlink.is_some()
}

// --- Raw XML shapes (post namespace strip) ---

#[derive(Debug, Default, Deserialize)]
struct RawRoot {
    #[serde(rename = "clrMapOvr")]
    clr_map_ovr: Option<RawClrMapOvr>,
}

#[derive(Debug, Default, Deserialize)]
struct RawClrMapOvr {
    #[serde(rename = "masterClrMapping")]
    master_clr_mapping: Option<EmptyMarker>,
    #[serde(rename = "overrideClrMapping")]
    override_clr_mapping: Option<RawOverrideClrMapping>,
}

#[derive(Debug, Default, Deserialize)]
struct EmptyMarker {}

// All field names stay ASCII identifiers; the source-XML attributes are
// `bg1` / `tx1` / etc. without prefix. clippy would otherwise complain
// about the shared `_` prefix-less names — but the OOXML naming is what it
// is, so we keep the raw struct mapping mechanical.
#[derive(Debug, Default, Deserialize)]
struct RawOverrideClrMapping {
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
