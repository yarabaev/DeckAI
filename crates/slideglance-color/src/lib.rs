//! OOXML theme color resolution and color transforms.
//!
//! Implements ECMA-376 §20.1.2.3 (color transforms — `lumMod`, `lumOff`,
//! `tint`, `shade`, `alpha`) and §20.1.6 (theme color schemes).
//!
//! The crate is purely model-level: it does not parse XML. Phase 3 (parser)
//! constructs [`ColorRef`] values and feeds them through [`ColorResolver`] to
//! obtain the final [`ResolvedColor`] (RGB + alpha) used by the renderer.
//!
//! ## Color spaces
//!
//! - [`Rgb`] — sRGB triple (8 bits per channel).
//! - [`Hsl`] — HSL triple (each component in `[0, 1]`), per W3C CSS Color
//!   Module Level 3 §4.2.4.
//!
//! ## Reference parity
//!
//! Behavior mirrors the original spec (the spec) bit-for-bit on
//! IEEE-754 inputs. `satMod` is intentionally omitted because the reference
//! does not apply it yet; it will be added when the spec does.
//! `PresetColor` is supported with the same eight-name table the TS
//! reference recognizes (see [`presets::resolve_preset`]).

#![deny(missing_docs)]

mod hsl;
mod presets;
mod resolver;
mod rgb;
mod scheme;
mod transforms;

pub use hsl::Hsl;
pub use presets::resolve_preset;
pub use resolver::ColorResolver;
pub use rgb::{ColorParseError, ResolvedColor, Rgb};
pub use scheme::{ColorMap, ColorRef, ColorScheme, SchemeColorKey, UnknownSchemeName};
pub use transforms::{apply_color_transforms, ColorTransform, PerMille};
