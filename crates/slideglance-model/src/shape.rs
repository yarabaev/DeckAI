//! Shape primitives: transform, geometry, and the [`SlideElement`] enum that
//! tags every drawable element on a slide.

use crate::chart::ChartElement;
use crate::effect::EffectList;
use crate::fill::Fill;
use crate::image::ImageElement;
use crate::line::Outline;
use crate::table::TableElement;
use crate::text::{Hyperlink, TextBody};
use slideglance_utils::Emu;

/// Position / size / rotation transform from `<a:xfrm>`.
#[derive(Copy, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Transform {
    /// X offset of the bounding box.
    pub offset_x: Emu,
    /// Y offset of the bounding box.
    pub offset_y: Emu,
    /// Bounding box width.
    pub extent_width: Emu,
    /// Bounding box height.
    pub extent_height: Emu,
    /// Rotation in degrees (0 = upright). OOXML stores this as 1/60000-degree
    /// units; the parser converts to degrees on the way in.
    pub rotation: f64,
    /// Horizontal flip.
    pub flip_h: bool,
    /// Vertical flip.
    pub flip_v: bool,
}

/// Either a preset `PowerPoint` shape (`<a:prstGeom>`) or a custom path
/// (`<a:custGeom>`).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "type", rename_all = "lowercase")
)]
pub enum Geometry {
    /// `<a:prstGeom>` — preset shape with adjustment values.
    Preset(PresetGeometry),
    /// `<a:custGeom>` — custom path geometry.
    Custom(CustomGeometry),
}

/// `<a:prstGeom>` payload.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PresetGeometry {
    /// Preset name (`@prst`), e.g. `"rect"`, `"ellipse"`, `"chevron"`.
    pub preset: String,
    /// `<a:avLst>` adjustment values (`@name` → numeric value).
    pub adjust_values: std::collections::BTreeMap<String, f64>,
}

/// `<a:custGeom>` payload — a list of sub-paths.
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CustomGeometry {
    /// Sub-paths (`<a:path>`).
    pub paths: Vec<CustomGeometryPath>,
}

/// One `<a:path>` inside a `<a:custGeom>`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CustomGeometryPath {
    /// Path width (logical units).
    pub width: f64,
    /// Path height (logical units).
    pub height: f64,
    /// SVG-like path command string assembled by the parser.
    pub commands: String,
}

/// Tagged union of every drawable thing that can sit in a `<p:spTree>`.
//
// The variants intentionally hold their full payload rather than `Box`-ing
// every one: SlideElement is moved/copied at most once per shape during
// rendering, so the per-enum size cost is negligible in practice and the
// pattern-match ergonomics matter more than the byte-count uniformity that
// `large_enum_variant` is trying to enforce.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "type", rename_all = "lowercase")
)]
pub enum SlideElement {
    /// `<p:sp>` — generic shape (with optional text body).
    Shape(ShapeElement),
    /// `<p:pic>` — image.
    Image(ImageElement),
    /// `<p:cxnSp>` — connector line.
    Connector(ConnectorElement),
    /// `<p:grpSp>` — group with nested children.
    Group(GroupElement),
    /// Chart-bearing `<p:graphicFrame>`.
    Chart(ChartElement),
    /// Table-bearing `<p:graphicFrame>`.
    Table(TableElement),
}

/// `<p:sp>` shape element.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShapeElement {
    /// `<p:cNvPr @id>` — slide-scoped unique identifier from the source PPTX.
    /// `None` for legacy decks that omit the attribute.
    pub sp_id: Option<u32>,
    /// Position / size / rotation.
    pub transform: Transform,
    /// Geometry (preset or custom).
    pub geometry: Geometry,
    /// Fill (`<a:*Fill>`). `None` means inherited.
    pub fill: Option<Fill>,
    /// Outline (`<a:ln>`). `None` means inherited.
    pub outline: Option<Outline>,
    /// `<p:txBody>` if present.
    pub text_body: Option<TextBody>,
    /// `<a:effectLst>` if present.
    pub effects: Option<EffectList>,
    /// `<p:ph @type>` placeholder type.
    pub placeholder_type: Option<String>,
    /// `<p:ph @idx>` placeholder index.
    pub placeholder_idx: Option<u32>,
    /// `<p:cNvPr @descr>` alt text.
    pub alt_text: Option<String>,
    /// `<p:cNvPr @name>` — emitted as `data-object-name`.
    pub object_name: Option<String>,
    /// `<p:cNvPr @hidden>`.
    pub hidden: bool,
    /// `<a:hlinkClick>` hyperlink.
    pub hyperlink: Option<Hyperlink>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_default_shape() -> ShapeElement {
        ShapeElement {
            sp_id: None,
            transform: Transform::default(),
            geometry: Geometry::Preset(PresetGeometry {
                preset: "rect".to_string(),
                adjust_values: std::collections::BTreeMap::new(),
            }),
            fill: None,
            outline: None,
            text_body: None,
            effects: None,
            placeholder_type: None,
            placeholder_idx: None,
            alt_text: None,
            object_name: None,
            hidden: false,
            hyperlink: None,
        }
    }

    #[test]
    fn shape_element_carries_sp_id() {
        let s = ShapeElement {
            sp_id: Some(7),
            ..make_default_shape()
        };
        assert_eq!(s.sp_id, Some(7));
    }
}

/// `<p:cxnSp>` connector element.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConnectorElement {
    /// `<p:cNvPr @id>` — slide-scoped unique identifier from the source PPTX.
    /// `None` for legacy decks that omit the attribute.
    pub sp_id: Option<u32>,
    /// Position / size / rotation.
    pub transform: Transform,
    /// Geometry (almost always preset for connectors).
    pub geometry: Geometry,
    /// Outline (connectors are stroke-only).
    pub outline: Option<Outline>,
    /// `<a:effectLst>` if present.
    pub effects: Option<EffectList>,
    /// `<p:cNvPr @descr>` alt text.
    pub alt_text: Option<String>,
    /// `<p:cNvPr @name>`.
    pub object_name: Option<String>,
    /// `<p:cNvPr @hidden>`.
    pub hidden: bool,
}

/// `<p:grpSp>` group element with its own coordinate system.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GroupElement {
    /// `<p:cNvPr @id>` — slide-scoped unique identifier from the source PPTX.
    /// `None` for legacy decks that omit the attribute.
    pub sp_id: Option<u32>,
    /// Group's own transform on the slide.
    pub transform: Transform,
    /// Child coordinate-space transform (`<a:xfrm><a:chOff>` / `<a:chExt>`).
    pub child_transform: Transform,
    /// Nested elements inside the group.
    pub children: Vec<SlideElement>,
    /// `<a:effectLst>` on the group.
    pub effects: Option<EffectList>,
    /// `<p:cNvPr @descr>` alt text.
    pub alt_text: Option<String>,
    /// `<p:cNvPr @name>`.
    pub object_name: Option<String>,
    /// `<p:cNvPr @hidden>`.
    pub hidden: bool,
}
