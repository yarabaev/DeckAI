//! Slide-XML element parser — image.
//!
//! Extracted from `slide.rs` so the per-element builder lives next
//! to its raw-XML model. The dispatcher in `slide::push_sp_tree_child`
//! routes to the `pub(super)` entry point in this module.

use std::collections::BTreeMap;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use serde::Deserialize;
use slideglance_color::ColorResolver;
use slideglance_model::{ImageElement, ImageFlip, SrcRect, StretchFillRect, TileInfo};
use slideglance_utils::Emu;

use crate::archive::PptxArchive;
use crate::blip_effect::{build_blip_effects, RawBlip};
use crate::effect::build_effect_list;
use crate::relationships::{resolve_relationship_target, Relationship};
use crate::shape_geometry::build_transform;

use super::shape::{RawCNvPr, RawShapeSpPr};
use super::{parse_attr_i64, parse_sp_id, parse_truthy, FRACTION_DIVISOR, TILE_DEFAULT_SCALE};

pub(super) fn build_image(
    pic: &RawPic,
    rels: &BTreeMap<String, Relationship>,
    base_path: &str,
    archive: &mut PptxArchive,
    resolver: &ColorResolver,
) -> Option<ImageElement> {
    let sp_pr = pic.sp_pr.as_ref()?;
    let transform = sp_pr.xfrm.as_ref().and_then(build_transform)?;

    let blip_fill = pic.blip_fill.as_ref()?;
    let blip = blip_fill.blip.as_ref()?;
    // PowerPoint stores Office 2016+ SVG icons with the SVG vector data
    // inside `<a:blip><a:extLst><a:ext><asvg:svgBlip r:embed="rIdN"/></a:ext></a:extLst></a:blip>`
    // and a PNG fallback in the regular `r:embed`. Prefer the SVG so
    // resvg renders crisp vectors; fall back to the raster otherwise.
    let r_id = blip.svg_embed().or(blip.embed.as_deref())?;
    let rel = rels.get(r_id)?;
    let media_path = resolve_relationship_target(base_path, &rel.target);
    let mime_type = mime_for_path(&media_path);
    let media_bytes = archive.media(&media_path).ok().flatten()?;
    let image_data = BASE64_STANDARD.encode(media_bytes);

    let effects = sp_pr
        .effect_lst
        .as_ref()
        .and_then(|e| build_effect_list(e, resolver));
    let blip_effects = build_blip_effects(blip, resolver);
    let src_rect = blip_fill.src_rect.as_ref().and_then(build_src_rect);
    let stretch = blip_fill.stretch.as_ref().and_then(build_stretch_fill_rect);
    let tile = blip_fill.tile.as_ref().map(build_tile_info);

    let cnv_pr = pic.nv_pic_pr.as_ref().and_then(|n| n.c_nv_pr.as_ref());
    let sp_id = parse_sp_id(cnv_pr);
    let alt_text = cnv_pr
        .and_then(|c| c.descr.clone())
        .filter(|s| !s.is_empty());
    let object_name = cnv_pr
        .and_then(|c| c.name.clone())
        .filter(|s| !s.is_empty());
    let hidden = cnv_pr
        .and_then(|c| c.hidden.as_deref())
        .is_some_and(parse_truthy);

    Some(ImageElement {
        sp_id,
        transform,
        image_data,
        mime_type,
        effects,
        blip_effects,
        src_rect,
        alt_text,
        object_name,
        hidden,
        stretch,
        tile,
        alpha: blip.alpha(),
    })
}

pub(super) fn mime_for_path(path: &str) -> String {
    let ext = path
        .rsplit('.')
        .next()
        .map_or_else(|| "png".to_owned(), str::to_ascii_lowercase);
    // OOXML's image/png fallback covers both unknown extensions and the
    // canonical `.png` case, so we collapse them onto one arm.
    let mime = match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "emf" => "image/emf",
        "wmf" => "image/wmf",
        _ => "image/png",
    };
    mime.to_owned()
}

pub(super) fn build_src_rect(node: &RawSrcRect) -> Option<SrcRect> {
    let l = parse_attr_i64(node.l.as_deref(), 0) as f64 / FRACTION_DIVISOR;
    let t = parse_attr_i64(node.t.as_deref(), 0) as f64 / FRACTION_DIVISOR;
    let r = parse_attr_i64(node.r.as_deref(), 0) as f64 / FRACTION_DIVISOR;
    let b = parse_attr_i64(node.b.as_deref(), 0) as f64 / FRACTION_DIVISOR;
    if l == 0.0 && t == 0.0 && r == 0.0 && b == 0.0 {
        return None;
    }
    Some(SrcRect {
        left: l,
        top: t,
        right: r,
        bottom: b,
    })
}

pub(super) fn build_stretch_fill_rect(node: &RawStretch) -> Option<StretchFillRect> {
    let fill_rect = node.fill_rect.as_ref()?;
    let l = parse_attr_i64(fill_rect.l.as_deref(), 0) as f64 / FRACTION_DIVISOR;
    let t = parse_attr_i64(fill_rect.t.as_deref(), 0) as f64 / FRACTION_DIVISOR;
    let r = parse_attr_i64(fill_rect.r.as_deref(), 0) as f64 / FRACTION_DIVISOR;
    let b = parse_attr_i64(fill_rect.b.as_deref(), 0) as f64 / FRACTION_DIVISOR;
    if l == 0.0 && t == 0.0 && r == 0.0 && b == 0.0 {
        return None;
    }
    Some(StretchFillRect {
        left: l,
        top: t,
        right: r,
        bottom: b,
    })
}

pub(super) fn build_tile_info(node: &RawTile) -> TileInfo {
    let sx = parse_attr_i64(node.sx.as_deref(), TILE_DEFAULT_SCALE) as f64 / FRACTION_DIVISOR;
    let sy = parse_attr_i64(node.sy.as_deref(), TILE_DEFAULT_SCALE) as f64 / FRACTION_DIVISOR;
    let flip = match node.flip.as_deref() {
        Some("x") => ImageFlip::X,
        Some("y") => ImageFlip::Y,
        Some("xy") => ImageFlip::Xy,
        _ => ImageFlip::None,
    };
    let align = node.align.clone().unwrap_or_else(|| "tl".to_owned());
    TileInfo {
        tx: Emu::new(parse_attr_i64(node.tx.as_deref(), 0)),
        ty: Emu::new(parse_attr_i64(node.ty.as_deref(), 0)),
        sx,
        sy,
        flip,
        align,
    }
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawPic {
    #[serde(rename = "nvPicPr")]
    pub nv_pic_pr: Option<RawNvPicPr>,
    #[serde(rename = "blipFill")]
    pub blip_fill: Option<RawPicBlipFill>,
    #[serde(rename = "spPr")]
    pub sp_pr: Option<RawShapeSpPr>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawNvPicPr {
    #[serde(rename = "cNvPr")]
    pub c_nv_pr: Option<RawCNvPr>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawPicBlipFill {
    pub blip: Option<RawBlip>,
    #[serde(rename = "srcRect")]
    pub src_rect: Option<RawSrcRect>,
    pub stretch: Option<RawStretch>,
    pub tile: Option<RawTile>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawSrcRect {
    #[serde(rename = "@l")]
    pub l: Option<String>,
    #[serde(rename = "@t")]
    pub t: Option<String>,
    #[serde(rename = "@r")]
    pub r: Option<String>,
    #[serde(rename = "@b")]
    pub b: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawStretch {
    #[serde(rename = "fillRect")]
    pub fill_rect: Option<RawSrcRect>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawTile {
    #[serde(rename = "@tx")]
    pub tx: Option<String>,
    #[serde(rename = "@ty")]
    pub ty: Option<String>,
    #[serde(rename = "@sx")]
    pub sx: Option<String>,
    #[serde(rename = "@sy")]
    pub sy: Option<String>,
    #[serde(rename = "@flip")]
    pub flip: Option<String>,
    #[serde(rename = "@algn")]
    pub align: Option<String>,
}
