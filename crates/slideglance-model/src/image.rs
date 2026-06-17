//! Image element placed on a slide.

use crate::effect::{BlipEffects, EffectList};
use crate::fill::ImageFlip;
use crate::shape::Transform;
use slideglance_utils::Emu;

/// Picture (`<p:pic>`) on the slide.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ImageElement {
    /// `<p:cNvPr @id>` — slide-scoped unique identifier from the source PPTX.
    /// `None` for legacy decks that omit the attribute.
    pub sp_id: Option<u32>,
    /// Position / size / rotation transform.
    pub transform: Transform,
    /// Base64-encoded image bytes (no `data:` prefix).
    pub image_data: String,
    /// Image MIME type.
    pub mime_type: String,
    /// Shape-level effects (shadow / glow / soft-edge).
    pub effects: Option<EffectList>,
    /// Image-content effects (grayscale / biLevel / blur / lum / etc.).
    pub blip_effects: Option<BlipEffects>,
    /// `<a:srcRect>` source crop.
    pub src_rect: Option<SrcRect>,
    /// Alt text from `<p:cNvPr @descr>`.
    pub alt_text: Option<String>,
    /// `<p:cNvPr @name>` — emitted as `data-object-name` on the wrapping `<g>`.
    pub object_name: Option<String>,
    /// `<p:cNvPr @hidden>` — when `true`, the image is parsed but not
    /// rendered.
    pub hidden: bool,
    /// `<a:stretch><a:fillRect/>`.
    pub stretch: Option<StretchFillRect>,
    /// `<a:tile>` parameters when present.
    pub tile: Option<TileInfo>,
    /// Image opacity from `<a:blip><a:alphaModFix amt="..."/>`. Ranges
    /// `0.0..=1.0`; `1.0` means fully opaque (also the default when the
    /// `amt` attribute is missing or the element is absent).
    pub alpha: f64,
}

/// `<a:srcRect>` source crop fractions.
#[derive(Copy, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SrcRect {
    /// Left fraction.
    pub left: f64,
    /// Top fraction.
    pub top: f64,
    /// Right fraction.
    pub right: f64,
    /// Bottom fraction.
    pub bottom: f64,
}

/// `<a:stretch><a:fillRect/>` — same shape as [`SrcRect`].
#[derive(Copy, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StretchFillRect {
    /// Left fraction.
    pub left: f64,
    /// Top fraction.
    pub top: f64,
    /// Right fraction.
    pub right: f64,
    /// Bottom fraction.
    pub bottom: f64,
}

/// `<a:tile>` parameters for an image element.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TileInfo {
    /// X offset.
    pub tx: Emu,
    /// Y offset.
    pub ty: Emu,
    /// X scale.
    pub sx: f64,
    /// Y scale.
    pub sy: f64,
    /// Flip behavior.
    pub flip: ImageFlip,
    /// Alignment passthrough.
    pub align: String,
}
