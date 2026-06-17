//! Slide, slide layout, slide master, and per-slide header/footer toggles.

use crate::fill::Fill;
use crate::shape::SlideElement;
use crate::text::{PlaceholderStyleInfo, TxStyles};
use slideglance_color::ColorMap;

/// One slide in the presentation.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Slide {
    // `slide_number` is OOXML domain terminology (vs. a generic `number`),
    // so we keep the prefix for clarity at call sites that handle multiple
    // numbered things (`slide_number`, `placeholder_idx`, …).
    #[allow(clippy::struct_field_names)]
    /// 1-based slide number in presentation order.
    pub slide_number: u32,
    /// Slide background. `None` means no slide-level background — the layout's
    /// (then master's) background applies.
    pub background: Option<Background>,
    /// Top-level shapes / pictures / tables / charts on this slide.
    pub elements: Vec<SlideElement>,
    /// `<p:cSld><p:bg/></p:cSld>` toggle equivalent: when `false`, the master
    /// shapes are suppressed for this slide.
    pub show_master_sp: bool,
    /// Per-slide header/footer toggle overrides.
    pub header_footer: Option<SlideHeaderFooter>,
    /// Speaker-notes text extracted from `notesSlide{N}.xml`.
    pub notes: Option<String>,
    /// Slide layout's `cSld @name` (used as `data-layout-name` SVG attribute).
    pub layout_name: Option<String>,
}

/// Background fill block.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Background {
    /// Fill applied as the background. `None` means `<a:noFill/>` was
    /// declared explicitly — distinct from "no background block at all"
    /// which is represented by [`Slide::background`] being `None`.
    pub fill: Option<Fill>,
}

/// Slide-level header/footer toggles. OOXML stores these on `<p:hf>` inside
/// `<p:sld>` (and inside `<p:presentation>` as global defaults).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SlideHeaderFooter {
    /// Show slide-number placeholder on this slide.
    pub show_slide_number: bool,
    /// Show date/time placeholder.
    pub show_date_time: bool,
    /// Show footer placeholder.
    pub show_footer: bool,
    /// Static footer text override (rare — usually inherited from layout).
    pub footer_text: Option<String>,
    /// Static datetime text override; if `None`, the renderer formats the
    /// current date.
    pub datetime_text: Option<String>,
}

/// Aggregated slide-layout data.
///
/// TS the spec returns each piece (background, elements, show-master-sp,
/// placeholder styles) as a separate parser function result. The Rust port
/// groups them into one struct so the parser layer (Phase 3-3) can return a
/// single owned value.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SlideLayout {
    /// Layout's `cSld @name`.
    pub name: Option<String>,
    /// Background defined on the layout.
    pub background: Option<Background>,
    /// Layout shapes (typically placeholders).
    pub elements: Vec<SlideElement>,
    /// Whether slides using this layout should display master shapes.
    pub show_master_sp: bool,
    /// Per-placeholder style information for inheritance.
    pub placeholder_styles: Vec<PlaceholderStyleInfo>,
}

/// Aggregated slide-master data.
///
/// As with [`SlideLayout`], the spec returns separate pieces; the
/// Rust port groups them.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SlideMaster {
    /// Master's `<p:clrMap>`.
    pub color_map: ColorMap,
    /// Background defined on the master.
    pub background: Option<Background>,
    /// Master shapes.
    pub elements: Vec<SlideElement>,
    /// `<p:txStyles>` block (title / body / other style stacks).
    pub tx_styles: Option<TxStyles>,
    /// Per-placeholder style information for inheritance.
    pub placeholder_styles: Vec<PlaceholderStyleInfo>,
}

/// Per-slide rendering bundle: the slide itself plus the layout / master
/// shapes that should be composited beneath it. Mirrors the spec's
/// `ParsedSlide` interface in.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RenderedSlide {
    /// The slide content with text inheritance already applied.
    pub slide: Slide,
    /// Layout shapes drawn beneath the slide's own elements.
    pub layout_elements: Vec<SlideElement>,
    /// Master shapes drawn beneath the layout. Only painted when
    /// [`Self::layout_show_master_sp`] is `true` AND
    /// [`Slide::show_master_sp`] is `true`.
    pub master_elements: Vec<SlideElement>,
    /// `<p:sldLayout @showMasterSp>` toggle from the slide's layout.
    pub layout_show_master_sp: bool,
}

/// Top-level presentation aggregate. Holds presentation-wide metadata plus
/// every slide already resolved with its layout / master inheritance.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Presentation {
    /// Document-level info (slide size, slide id list, embedded fonts,
    /// sections, default text style, …).
    pub info: crate::presentation::PresentationInfo,
    /// Resolved theme (color/font/format schemes).
    pub theme: crate::theme::Theme,
    /// Every slide bundled with its layout / master rendering context.
    pub slides: Vec<RenderedSlide>,
}
