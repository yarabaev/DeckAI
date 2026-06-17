//! Fill types: solid color, gradient, image (blip), pattern, none.

use slideglance_color::ResolvedColor;
use slideglance_utils::Emu;

/// Tagged union for the four `<a:*Fill>` element families plus the explicit
/// `<a:noFill/>`. Maps to TS `Fill` from.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "type", rename_all = "lowercase")
)]
pub enum Fill {
    /// Single resolved color.
    Solid(SolidFill),
    /// Multi-stop gradient.
    Gradient(GradientFill),
    /// Embedded raster image fill.
    Image(ImageFill),
    /// Pattern fill (preset `PowerPoint` pattern + foreground/background color).
    Pattern(PatternFill),
    /// Explicit no-fill (`<a:noFill/>`).
    None(NoFill),
}

/// `<a:solidFill>`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SolidFill {
    /// The resolved fill color.
    pub color: ResolvedColor,
}

/// `<a:gradFill>`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GradientFill {
    /// Gradient stops, ordered by `<a:gs pos>`.
    pub stops: Vec<GradientStop>,
    /// Gradient angle in degrees (0 = horizontal, 90 = vertical).
    pub angle: f64,
    /// Linear vs. radial geometry.
    pub gradient_type: GradientType,
    /// Center X (path gradient only) — fraction of the bounding box.
    pub center_x: Option<f64>,
    /// Center Y (path gradient only).
    pub center_y: Option<f64>,
}

/// Gradient geometry kind.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
pub enum GradientType {
    /// `<a:lin>` linear gradient.
    Linear,
    /// `<a:path>` radial / shape-tracing gradient.
    Radial,
}

/// One stop on a gradient.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GradientStop {
    /// Stop position in `[0, 1]` (TS uses 0-1 fractions).
    pub position: f64,
    /// Resolved color at this stop.
    pub color: ResolvedColor,
}

/// `<a:blipFill>` — image as fill.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ImageFill {
    /// Base64-encoded image data (data URI body, no `data:` prefix).
    pub image_data: String,
    /// MIME type (e.g. `"image/png"`).
    pub mime_type: String,
    /// Tile parameters (`<a:tile>`) — `None` if the image is stretched.
    pub tile: Option<ImageFillTile>,
    /// Source crop on the original image (0-1 fractions of l/t/r/b),
    /// mirroring `<a:srcRect>`. Applied before stretch/tile compositing.
    pub src_rect: Option<ImageRect>,
    /// Stretch fill rectangle (`<a:stretch><a:fillRect/>`). Each value is a
    /// 0-1 fraction; positive insets the image edge, negative extends it
    /// past the container edge. `None` means the image fills 1:1.
    pub stretch: Option<ImageRect>,
    /// Image opacity from `<a:blip><a:alphaModFix amt="..."/>`. Ranges
    /// `0.0..=1.0`; `1.0` means fully opaque (also the default when the
    /// `amt` attribute is missing or the element is absent).
    pub alpha: f64,
}

/// 4-edge inset/extension fractions used by both `srcRect` and `fillRect`.
#[derive(Copy, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ImageRect {
    /// Left edge fraction (positive = inset).
    pub left: f64,
    /// Top edge fraction.
    pub top: f64,
    /// Right edge fraction.
    pub right: f64,
    /// Bottom edge fraction.
    pub bottom: f64,
}

/// `<a:tile>` parameters for image fill.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ImageFillTile {
    /// Tile X offset.
    pub tx: Emu,
    /// Tile Y offset.
    pub ty: Emu,
    /// Tile X scale factor (1.0 = original size).
    pub sx: f64,
    /// Tile Y scale factor.
    pub sy: f64,
    /// Tile flip behavior.
    pub flip: ImageFlip,
    /// Tile alignment (`<a:tile alignment="...">`) — string passthrough.
    pub align: String,
}

/// `<a:tile flip>` enum.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
pub enum ImageFlip {
    /// No flip.
    #[default]
    None,
    /// Flip across X.
    X,
    /// Flip across Y.
    Y,
    /// Flip across both axes.
    Xy,
}

/// `<a:pattFill>` — preset stipple pattern with foreground/background color.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PatternFill {
    /// `PowerPoint` preset name (e.g. `"pct5"`, `"diagBrick"`). Passed through
    /// to the renderer.
    pub preset: String,
    /// Foreground (pattern) color.
    pub foreground_color: ResolvedColor,
    /// Background color.
    pub background_color: ResolvedColor,
}

/// Marker struct for `<a:noFill/>` — kept as a struct (rather than a unit
/// variant) so the discriminator-tagged enum serializes consistently.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NoFill {}
