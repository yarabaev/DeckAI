//! OOXML PPTX semantic model.
//!
//! Pure data types — the result of parsing a `.pptx` archive — used by the
//! renderer (Phase 5) and other consumers. Construction is the parser's job
//! (Phase 3-3); this crate intentionally has no parsing logic.
//!
//! Type design follows OOXML / ECMA-376 nomenclature with the following
//! idiomatic Rust adaptations:
//!
//! - Tagged unions become Rust enums (`#[serde(tag = "type", rename_all = "...")]`).
//! - `string | null` becomes `Option<String>`.
//! - Length-typed values use [`slideglance_utils::Emu`] / [`slideglance_utils::Pt`] /
//!   [`slideglance_utils::HundredthPt`] instead of raw integers.
//! - Resolved colors come from [`slideglance_color::ResolvedColor`].
//!
//! Top-level aggregates ([`SlideMaster`], [`SlideLayout`], [`PresentationInfo`])
//! collect the pieces the parsers return as separate function results, so the
//! parser layer can hand a single owned value back.

#![deny(missing_docs)]

pub mod chart;
pub mod effect;
pub mod fill;
pub mod image;
pub mod line;
pub mod presentation;
pub mod shape;
pub mod slide;
pub mod table;
pub mod text;
pub mod theme;

pub use chart::{
    AxisGroup, BarDirection, ChartAxis, ChartData, ChartDataLabels, ChartDataPoint, ChartElement,
    ChartLegend, ChartSeries, ChartTrendline, ChartType, LegendPosition, OfPieType, RadarStyle,
    TickMark, TrendlineType,
};
pub use effect::{
    BiLevelEffect, BlipEffects, BlurEffect, ClrChangeEffect, DuotoneEffect, EffectList, Glow,
    InnerShadow, LumEffect, OuterShadow, SoftEdge,
};
pub use fill::{
    Fill, GradientFill, GradientStop, GradientType, ImageFill, ImageFillTile, ImageFlip, ImageRect,
    NoFill, PatternFill, SolidFill,
};
pub use image::{ImageElement, SrcRect, StretchFillRect, TileInfo};
pub use line::{
    ArrowEndpoint, ArrowSize, ArrowType, DashStyle, LineCap, LineJoin, Outline, OutlineFill,
};
pub use presentation::{
    EmbeddedFont, ModifyVerifier, PresentationInfo, PresentationSection, Protection, SlideSize,
};
pub use shape::{
    ConnectorElement, CustomGeometry, CustomGeometryPath, Geometry, GroupElement, PresetGeometry,
    ShapeElement, SlideElement, Transform,
};
pub use slide::{
    Background, Presentation, RenderedSlide, Slide, SlideHeaderFooter, SlideLayout, SlideMaster,
};
pub use table::{
    CellBorders, TableCell, TableColumn, TableData, TableElement, TableRow, TableStyleOptions,
};
pub use text::{
    AutoFit, AutoNumScheme, BodyProperties, BulletType, DefaultParagraphLevelProperties,
    DefaultRunProperties, DefaultTextStyle, Hyperlink, Paragraph, ParagraphAlignment,
    ParagraphProperties, PlaceholderBodyPr, PlaceholderStyleInfo, RunProperties, SpacingValue,
    TabStop, TabStopAlignment, TextBody, TextOutline, TextRun, TextVerticalType, TxStyles,
    VerticalAnchor, WrapMode,
};
pub use theme::{FontScheme, FormatScheme, Theme};

// Re-exports of resolved-color and color-scheme types shared with
// `slideglance-color`, so downstream consumers don't need to depend on both crates
// just to handle theme colors.
pub use slideglance_color::{ColorMap, ColorRef, ColorResolver, ColorScheme, ResolvedColor, Rgb};
