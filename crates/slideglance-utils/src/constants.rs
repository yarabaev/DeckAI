//! OOXML coordinate-system constants.

/// EMU per inch (ECMA-376 §20.1.2.1).
pub const EMU_PER_INCH: i64 = 914_400;

/// EMU per point (1 pt = 1/72 inch = 12,700 EMU).
pub const EMU_PER_POINT: i64 = 12_700;

/// Default rendering DPI used by `Emu::to_pixels` when no DPI is supplied.
pub const DEFAULT_DPI: u32 = 96;

/// Default output width (px) for slide rendering.
pub const DEFAULT_OUTPUT_WIDTH: u32 = 960;

/// Rotation unit: PPTX stores rotations as 1/60000 of a degree.
pub const ROTATION_UNIT: i64 = 60_000;
