//! Top-level presentation metadata (slide size, embedded fonts, sections,
//! protection settings).

use slideglance_utils::Emu;

use crate::text::DefaultTextStyle;

/// Presentation-level information aggregated from `ppt/presentation.xml`.
///
/// Mirrors the TypeScript `PresentationInfo` type from
/// .
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PresentationInfo {
    /// Slide width and height in EMU.
    pub slide_size: SlideSize,
    /// `<p:sldId>` relationship IDs, in order.
    pub slide_r_ids: Vec<String>,
    /// Numeric `<p:sldId @id>` values aligned with [`Self::slide_r_ids`]. Used
    /// to resolve `<p14:section>` membership to slide numbers.
    pub slide_id_values: Vec<i64>,
    /// Default text style block inherited by slides without an explicit one.
    pub default_text_style: Option<DefaultTextStyle>,
    /// Embedded fonts declared by `<p:embeddedFontLst>`.
    pub embedded_fonts: Option<Vec<EmbeddedFont>>,
    /// Modify-verifier protection settings.
    pub protection: Option<Protection>,
    /// Section breaks (`<p14:sectionLst>`) expanded with referenced slide IDs.
    pub sections: Option<Vec<PresentationSection>>,
}

/// Slide width and height in EMU.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SlideSize {
    /// Slide width.
    pub width: Emu,
    /// Slide height.
    pub height: Emu,
}

/// One section break from `<p14:sectionLst>`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PresentationSection {
    /// Section display name.
    pub name: String,
    /// Numeric IDs (matching `<p:sldId @id>`) of slides in this section.
    pub slide_ids: Vec<i64>,
}

/// `<p:embeddedFont>` entry: typeface metadata + per-style relationship IDs.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::struct_excessive_bools)] // four subsetted flags mirror OOXML structure 1:1
pub struct EmbeddedFont {
    /// Font face name (e.g. `"Calibri"`).
    pub typeface: String,
    /// PANOSE classification string (10 hex bytes).
    pub panose: Option<String>,
    /// `pitchFamily` (per OOXML — see `<a:font>` documentation).
    pub pitch_family: Option<i32>,
    /// `charset` (per OOXML).
    pub charset: Option<i32>,
    /// Relationship ID for the regular face binary.
    pub regular_r_id: Option<String>,
    /// Relationship ID for the bold face binary.
    pub bold_r_id: Option<String>,
    /// Relationship ID for the italic face binary.
    pub italic_r_id: Option<String>,
    /// Relationship ID for the bold-italic face binary.
    pub bold_italic_r_id: Option<String>,
    /// `@subsetted` flag on the regular face (`<p:regular @subsetted="1">`).
    pub regular_subsetted: bool,
    /// `@subsetted` flag on the bold face.
    pub bold_subsetted: bool,
    /// `@subsetted` flag on the italic face.
    pub italic_subsetted: bool,
    /// `@subsetted` flag on the bold-italic face.
    pub bold_italic_subsetted: bool,
}

/// `<p:modifyVerifier>` protection block.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Protection {
    /// Modify-verifier hash details.
    pub modify_verifier: Option<ModifyVerifier>,
}

/// Hash and salt parameters for the modify-protection password.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ModifyVerifier {
    /// Hash algorithm name (e.g. `"SHA-512"`).
    pub algorithm_name: Option<String>,
    /// Base64-encoded hash value.
    pub hash_value: Option<String>,
    /// Base64-encoded salt.
    pub salt_value: Option<String>,
    /// Iteration count.
    pub spin_count: Option<i64>,
}
