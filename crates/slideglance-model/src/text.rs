//! Text body / paragraph / run / styling types.

use slideglance_color::ResolvedColor;
use slideglance_utils::{Emu, HundredthPt, Pt};

use crate::shape::{Geometry, Transform};

/// `<p:txBody>` — paragraphs plus body-level box properties.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextBody {
    /// Paragraphs in source order.
    pub paragraphs: Vec<Paragraph>,
    /// `<a:bodyPr>` body properties.
    pub body_properties: BodyProperties,
    /// Fallback text color resolved from the parent shape's
    /// `<p:style>/<a:fontRef>` (typical OOXML pattern: a layout shape
    /// references `lt1` so its text reads white on a dark fill, with
    /// no per-run `<a:solidFill>`). Each run still wins when it has
    /// its own color; this is only consulted as the final tier.
    pub default_text_color: Option<slideglance_color::ResolvedColor>,
}

/// `<a:bodyPr>` text-frame properties.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BodyProperties {
    /// Vertical anchor (`@anchor`).
    pub anchor: VerticalAnchor,
    /// Left inset.
    pub margin_left: Emu,
    /// Right inset.
    pub margin_right: Emu,
    /// Top inset.
    pub margin_top: Emu,
    /// Bottom inset.
    pub margin_bottom: Emu,
    /// Word wrap mode (`@wrap`).
    pub wrap: WrapMode,
    /// Auto-fit mode.
    pub auto_fit: AutoFit,
    /// `<a:normAutofit @fontScale>` — 1.0 if unset.
    pub font_scale: f64,
    /// `<a:normAutofit @lnSpcReduction>` — line-spacing reduction in 0-1.
    pub ln_spc_reduction: f64,
    /// `<a:bodyPr @numCol>`.
    pub num_col: u32,
    /// Vertical text mode.
    pub vert: TextVerticalType,
    /// `<a:bodyPr @spcFirstLastPara>` — when **false** (the spec default,
    /// ECMA-376 Part 1 §21.1.2.1.5), the renderer must IGNORE the
    /// `<a:spcBef>` of the first paragraph and the `<a:spcAft>` of the
    /// last paragraph in this text body. `PowerPoint` sets this to true
    /// to honor those margins; without that flag the text body's outer
    /// edges hug the frame regardless of paragraph spacing values
    /// inherited from master / layout list-styles.
    pub spc_first_last_para: bool,
    /// `<a:bodyPr @compatLnSpc>` — when **true**, line spacing is
    /// computed in a simplistic manner: `line_height = font_size ×
    /// lineSpacing_factor`, ignoring the font's natural ascent /
    /// descent / linegap. When **false** (the default per ECMA-376
    /// §21.1.2.1.4), the natural line height drives the spacing
    /// factor. `PowerPoint`'s compat-mode flag exists so legacy decks
    /// that hard-coded text-frame heights based on font size keep
    /// rendering at their original layout; modern decks leave it
    /// false and use natural metrics.
    pub compat_ln_spc: bool,
    /// `WordArt` warp preset (`<a:bodyPr><a:prstTxWarp prst="textArchUp"/>`).
    pub prst_tx_warp: Option<String>,
}

/// `<a:bodyPr @anchor>`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
pub enum VerticalAnchor {
    /// Top.
    #[default]
    T,
    /// Center.
    Ctr,
    /// Bottom.
    B,
}

/// `<a:bodyPr @wrap>`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
pub enum WrapMode {
    /// Word-wrap inside the box bounds.
    #[default]
    Square,
    /// No wrapping.
    None,
}

/// `<a:bodyPr>` auto-fit family.
//
// All three variants end in "Autofit" because the OOXML schema names the
// elements `noAutofit` / `normAutofit` / `spAutoFit` directly. Renaming to
// drop the suffix would lose that bidirectional mapping with the source XML.
#[allow(clippy::enum_variant_names)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum AutoFit {
    /// `<a:noAutofit/>`.
    #[default]
    NoAutofit,
    /// `<a:normAutofit/>`.
    NormAutofit,
    /// `<a:spAutoFit/>`.
    SpAutofit,
}

/// `<a:bodyPr @vert>`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum TextVerticalType {
    /// Horizontal text.
    #[default]
    Horz,
    /// 90-degree rotated.
    Vert,
    /// 270-degree rotated.
    Vert270,
    /// East-Asian vertical.
    EaVert,
    /// `WordArt` vertical.
    WordArtVert,
    /// Mongolian vertical.
    MongolianVert,
}

/// One paragraph (`<a:p>`).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Paragraph {
    /// Runs (`<a:r>`/`<a:fld>`/`<a:br>`) in source order.
    pub runs: Vec<TextRun>,
    /// Paragraph properties (`<a:pPr>`).
    pub properties: ParagraphProperties,
    /// `<a:endParaRPr>` — applies to a virtual end-of-paragraph run when no
    /// explicit run carries the formatting (used for caret position and
    /// trailing newlines).
    pub end_para_run_properties: Option<RunProperties>,
}

/// `<a:pPr @algn>` (paragraph alignment).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
pub enum ParagraphAlignment {
    /// Left.
    L,
    /// Center.
    Ctr,
    /// Right.
    R,
    /// Justify.
    Just,
}

/// `<a:pPr @algn>` for [`DefaultParagraphLevelProperties`] — same set as
/// [`ParagraphAlignment`]; kept distinct in TS so we mirror the alias.
pub use ParagraphAlignment as DefaultParagraphAlignment;

/// `<a:buAutoNum @type>`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum AutoNumScheme {
    /// `1.`, `2.`, ...
    ArabicPeriod,
    /// `1)`, `2)`, ...
    ArabicParenR,
    /// `I.`, `II.`, ... (uppercase Roman).
    RomanUcPeriod,
    /// `i.`, `ii.`, ... (lowercase Roman).
    RomanLcPeriod,
    /// `A.`, `B.`, ... (uppercase Latin).
    AlphaUcPeriod,
    /// `a.`, `b.`, ... (lowercase Latin).
    AlphaLcPeriod,
    /// `a)`, `b)`, ...
    AlphaLcParenR,
    /// `A)`, `B)`, ...
    AlphaUcParenR,
    /// `1`, `2`, ... (no period).
    ArabicPlain,
    /// `①`, `②`, ... — double-byte plain circle (CJK).
    CircleNumDbPlain,
    /// `❶`, `❷`, ... — white-on-black circle.
    CircleNumWdBlackPlain,
    /// `①`, `②`, ... — white-on-white plain circle (ASCII style).
    CircleNumWdWhitePlain,
}

/// `<a:buNone/>` / `<a:buChar/>` / `<a:buAutoNum/>` discriminator.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "type", rename_all = "camelCase")
)]
pub enum BulletType {
    /// `<a:buNone/>`.
    None,
    /// `<a:buChar @char>`.
    Char {
        /// The bullet character (typically a single Unicode code point).
        char: String,
    },
    /// `<a:buAutoNum @type @startAt>`.
    AutoNum {
        /// Numbering scheme.
        scheme: AutoNumScheme,
        /// Starting number (default 1).
        start_at: u32,
    },
}

/// `<a:spcBef|spcAft>` — paragraph spacing in points or percent.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "type", rename_all = "lowercase")
)]
pub enum SpacingValue {
    /// `<a:spcPts @val>` — value is in 1/100 of a point.
    Pts {
        /// The 1/100-pt amount.
        value: HundredthPt,
    },
    /// `<a:spcPct @val>` — `50000` = 50%.
    Pct {
        /// The per-mille percentage value.
        value: f64,
    },
}

/// `<a:tab>` tab stop.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TabStop {
    /// Tab stop X position (EMU).
    pub position: Emu,
    /// Tab alignment (`@algn`).
    pub alignment: TabStopAlignment,
}

/// `<a:tab @algn>`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
pub enum TabStopAlignment {
    /// Left-aligned tab.
    #[default]
    L,
    /// Centered tab.
    Ctr,
    /// Right-aligned tab.
    R,
    /// Decimal-aligned tab.
    Dec,
}

/// `<a:pPr>` — paragraph-level properties carried alongside the run list.
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ParagraphProperties {
    /// Alignment; `None` means inherited.
    pub alignment: Option<ParagraphAlignment>,
    /// `<a:lnSpc @val>` — line-spacing factor (1.0 = single, 2.0 = double).
    pub line_spacing: Option<f64>,
    /// `<a:spcBef>`. `None` means "inherited" (renderer treats as 0 when
    /// no inherited value resolves); `Some` is an explicit override —
    /// including a deliberate zero — that wins over the lstStyle chain.
    pub space_before: Option<SpacingValue>,
    /// `<a:spcAft>`. Same `None`-means-inherited semantics as `space_before`.
    pub space_after: Option<SpacingValue>,
    /// `<a:pPr @lvl>` (0-8).
    pub level: u8,
    /// Bullet block (`<a:buNone>` / `<a:buChar>` / `<a:buAutoNum>`).
    pub bullet: Option<BulletType>,
    /// `<a:buFont @typeface>`.
    pub bullet_font: Option<String>,
    /// `<a:buClr>`.
    pub bullet_color: Option<ResolvedColor>,
    /// `<a:buSzPct @val>` — percent of run text size.
    pub bullet_size_pct: Option<f64>,
    /// `<a:pPr @marL>` — left margin (EMU).
    pub margin_left: Option<Emu>,
    /// `<a:pPr @indent>` — first-line indent (EMU; can be negative).
    pub indent: Option<Emu>,
    /// `<a:tabLst>` tab stops.
    pub tab_stops: Vec<TabStop>,
}

/// One text run (`<a:r>` or `<a:fld>` or `<a:br>`).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextRun {
    /// Run text content.
    pub text: String,
    /// Run properties (`<a:rPr>`).
    pub properties: RunProperties,
    /// `<a:fld @type>` value when this run originates from a field. The
    /// renderer substitutes the run's text based on this hint (e.g. current
    /// slide number for `"slidenum"`, current date for `"datetime*"`).
    /// Plain runs leave this `None`.
    pub field_type: Option<String>,
}

/// `<a:hlinkClick>` payload.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Hyperlink {
    /// Link target URL.
    pub url: String,
    /// `<a:hlinkClick @tooltip>`.
    pub tooltip: Option<String>,
}

/// `<a:ln>` outline applied to text glyphs.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextOutline {
    /// Stroke width.
    pub width: Emu,
    /// Stroke color.
    pub color: ResolvedColor,
}

/// `<a:rPr>` — full run properties (final, post-inheritance).
//
// OOXML `<a:rPr>` exposes bold / italic / underline / strikethrough as four
// independent boolean attributes. Collapsing them into a bitflag would lose
// direct field access at every call site for no real benefit — the renderer
// inspects them individually anyway.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RunProperties {
    /// Font size (`<a:rPr @sz>` — OOXML stores as 100ths of a point; the
    /// parser converts to [`Pt`] on the way in).
    pub font_size: Option<Pt>,
    /// `<a:latin @typeface>`.
    pub font_family: Option<String>,
    /// `<a:ea @typeface>`.
    pub font_family_ea: Option<String>,
    /// `<a:cs @typeface>`.
    pub font_family_cs: Option<String>,
    /// Bold.
    pub bold: bool,
    /// Italic.
    pub italic: bool,
    /// Any underline (single, double, etc. — collapsed to a flag).
    pub underline: bool,
    /// Any strikethrough.
    pub strikethrough: bool,
    /// Run color.
    pub color: Option<ResolvedColor>,
    /// `<a:rPr @baseline>` — superscript/subscript shift in 1000ths.
    pub baseline: f64,
    /// `<a:hlinkClick>`.
    pub hyperlink: Option<Hyperlink>,
    /// `<a:ln>` text outline.
    pub outline: Option<TextOutline>,
    /// `<a:highlight>` highlight color.
    pub highlight: Option<ResolvedColor>,
    /// `<a:sym @typeface>` — symbol font for private-use area codepoints.
    pub font_family_sym: Option<String>,
    /// `<a:rPr @kern>` — kern-pair activation threshold (half-points).
    /// When `Some(threshold)`, TTF kern table pairs are applied when the
    /// rendered font size exceeds `threshold`.
    pub kern: Option<HundredthPt>,
    /// `<a:rPr @spc>` — character spacing (hundredths of a point).
    /// Positive = expanded, negative = condensed.
    pub char_spacing: Option<HundredthPt>,
}

/// `<a:defRPr>` — defaults applied when run properties are missing.
///
/// Distinct from [`RunProperties`] because every field here is optional with
/// no implicit default value (run defaults inherit upward).
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DefaultRunProperties {
    /// Default font size.
    pub font_size: Option<Pt>,
    /// Default Latin font.
    pub font_family: Option<String>,
    /// Default East-Asian font.
    pub font_family_ea: Option<String>,
    /// Default complex-script font.
    pub font_family_cs: Option<String>,
    /// Bold default.
    pub bold: Option<bool>,
    /// Italic default.
    pub italic: Option<bool>,
    /// Underline default.
    pub underline: Option<bool>,
    /// Strikethrough default.
    pub strikethrough: Option<bool>,
    /// Color default.
    pub color: Option<ResolvedColor>,
    /// Highlight default.
    pub highlight: Option<ResolvedColor>,
}

/// `<a:lvlNpPr>` — defaults at one of the 9 indent levels.
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DefaultParagraphLevelProperties {
    /// Default alignment.
    pub alignment: Option<ParagraphAlignment>,
    /// Default left margin.
    pub margin_left: Option<Emu>,
    /// Default first-line indent.
    pub indent: Option<Emu>,
    /// Default bullet block.
    pub bullet: Option<BulletType>,
    /// Default bullet font.
    pub bullet_font: Option<String>,
    /// Default bullet color.
    pub bullet_color: Option<ResolvedColor>,
    /// Default bullet size percentage.
    pub bullet_size_pct: Option<f64>,
    /// Default `<a:lnSpc>` value (percent × 1000, matching `Paragraph.line_spacing`).
    pub line_spacing: Option<f64>,
    /// Default `<a:spcBef>` value (Pt or Pct, matching `Paragraph.space_before`).
    pub space_before: Option<SpacingValue>,
    /// Default `<a:spcAft>` value.
    pub space_after: Option<SpacingValue>,
    /// Default run properties for this level.
    pub default_run_properties: Option<DefaultRunProperties>,
}

/// `<a:lstStyle>` / `<p:defaultTextStyle>` — paragraph defaults plus 9
/// indent-level entries (`lvl1pPr` … `lvl9pPr`).
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DefaultTextStyle {
    /// `<a:defPPr>` — applied to paragraph level 0 by default.
    pub default_paragraph: Option<DefaultParagraphLevelProperties>,
    /// 9 entries for indent levels 1-9 (`lvl1pPr` is index 0). Each is
    /// `Option` because the source XML may omit individual levels.
    pub levels: Vec<Option<DefaultParagraphLevelProperties>>,
}

/// `<p:txStyles>` block on a slide master — three style stacks for the three
/// placeholder roles.
//
// All three field names end in "_style" because the OOXML elements are
// named `titleStyle` / `bodyStyle` / `otherStyle` — keeping the suffix
// preserves the source-XML mapping at the cost of triggering this lint.
#[allow(clippy::struct_field_names)]
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TxStyles {
    /// `<p:titleStyle>`.
    pub title_style: Option<DefaultTextStyle>,
    /// `<p:bodyStyle>`.
    pub body_style: Option<DefaultTextStyle>,
    /// `<p:otherStyle>`.
    pub other_style: Option<DefaultTextStyle>,
}

/// Per-placeholder style data, used during slide-layout / slide-master
/// inheritance.
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlaceholderStyleInfo {
    /// `<p:ph @type>` (e.g. `"title"`, `"body"`, `"sldNum"`).
    pub placeholder_type: String,
    /// `<p:ph @idx>`.
    pub placeholder_idx: Option<u32>,
    /// `<a:lstStyle>` block carried by this placeholder shape.
    pub lst_style: Option<DefaultTextStyle>,
    /// `<a:bodyPr>` attributes that the layout / master placeholder
    /// **explicitly set**. Each field is `Some(value)` only when the
    /// layout shape's bodyPr carried that attribute literally — empty
    /// or absent attributes stay `None` so the slide's own bodyPr
    /// (which already filled in spec defaults) wins. This is the
    /// inheritance carrier the spec's "ancestor of the same type"
    /// resolution chain (MS-OE376 §5.1.5.1.1) walks.
    pub body_properties: Option<PlaceholderBodyPr>,
    /// Optional transform inherited by slides referencing this placeholder.
    pub transform: Option<Transform>,
    /// Optional geometry inherited by slides referencing this placeholder.
    pub geometry: Option<Geometry>,
}

/// Sparse `<a:bodyPr>` representation used for layout/master inheritance.
/// Every field is `Option`: `Some(value)` means the source XML carried
/// that attribute explicitly, `None` means inherit from the next chain
/// level. This contrasts with [`BodyProperties`] which holds the fully
/// resolved values used by the renderer.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlaceholderBodyPr {
    /// `<a:bodyPr @anchor>`.
    pub anchor: Option<VerticalAnchor>,
    /// `<a:bodyPr @lIns>` (EMU).
    pub margin_left: Option<Emu>,
    /// `<a:bodyPr @rIns>`.
    pub margin_right: Option<Emu>,
    /// `<a:bodyPr @tIns>`.
    pub margin_top: Option<Emu>,
    /// `<a:bodyPr @bIns>`.
    pub margin_bottom: Option<Emu>,
    /// `<a:bodyPr @wrap>`.
    pub wrap: Option<WrapMode>,
    /// `<a:bodyPr @vert>`.
    pub vert: Option<TextVerticalType>,
    /// `<a:bodyPr @numCol>`.
    pub num_col: Option<u32>,
    /// `<a:bodyPr @spcFirstLastPara>`.
    pub spc_first_last_para: Option<bool>,
    /// `<a:bodyPr @compatLnSpc>`.
    pub compat_ln_spc: Option<bool>,
    /// `<a:bodyPr><a:normAutofit/>` / `<a:spAutoFit/>` selection. `None`
    /// means slide didn't author either child element — inherit from
    /// the layout / master placeholder. Coupled with [`Self::font_scale`]
    /// and [`Self::ln_spc_reduction`] which originate from the same
    /// `<a:normAutofit>` attributes.
    pub auto_fit: Option<AutoFit>,
    /// `<a:normAutofit @fontScale>` (1.0 default per OOXML).
    pub font_scale: Option<f64>,
    /// `<a:normAutofit @lnSpcReduction>` (0.0 default per OOXML).
    pub ln_spc_reduction: Option<f64>,
}

impl Default for SpacingValue {
    fn default() -> Self {
        Self::Pts {
            value: HundredthPt::new(0),
        }
    }
}
