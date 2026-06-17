//! Unit-aware primitives for OOXML coordinate systems.
//!
//! PPTX uses several length units that must not be mixed at call sites:
//! `Emu` (English Metric Units), `Pt` (points), and `HundredthPt` (1/100
//! point, used for paragraph spacing in ECMA-376). This crate provides
//! zero-cost newtypes that make unit confusion a compile-time error and
//! exposes conversion helpers between them.
//!
//! Reference: ECMA-376 §20.1.2.1.

#![deny(missing_docs)]

mod constants;
mod units;

pub use constants::{
    DEFAULT_DPI, DEFAULT_OUTPUT_WIDTH, EMU_PER_INCH, EMU_PER_POINT, ROTATION_UNIT,
};
pub use units::{Emu, HundredthPt, Pt};

/// Converts a 1/60000-degree rotation value to degrees.
///
/// PPTX rotation attributes are stored as `1/60000`th of a degree per
/// ECMA-376 §20.1.10.3.
#[must_use]
pub fn rotation_to_degrees(rotation: i64) -> f64 {
    rotation as f64 / ROTATION_UNIT as f64
}
