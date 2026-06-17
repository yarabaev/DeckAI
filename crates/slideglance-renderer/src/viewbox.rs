//! Slide viewBox calculation.
//!
//! The spec computes width/height inline inside `renderSlideToSvg`
//! (`emuToPixels(slideSize.width)`), then emits
//! `viewBox="0 0 W H" width="W" height="H"`. We isolate that into a small
//! value type so renderer batches that consume the slide bounds (background
//! fill area, group clip rects) do not have to recompute it.
//!
//! The viewBox origin is always `(0, 0)` per OOXML semantics — `<p:sldSz>`
//! defines the *size* of the slide, not an offset.

use slideglance_model::SlideSize;

/// Pixel-space slide bounding box. Origin is `(0, 0)`; only the extent
/// matters for SVG emission.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SlideViewBox {
    /// Slide width in pixels at 96 DPI.
    pub width: f64,
    /// Slide height in pixels at 96 DPI.
    pub height: f64,
}

impl SlideViewBox {
    /// Construct from a [`SlideSize`] (EMU values from `<p:sldSz>`).
    #[must_use]
    pub fn from_slide_size(size: SlideSize) -> Self {
        Self {
            width: size.width.to_pixels(),
            height: size.height.to_pixels(),
        }
    }

    /// Format the SVG `viewBox` attribute value (`"0 0 W H"`). Integer-valued
    /// dimensions emit without a decimal point to match the TS output.
    #[must_use]
    pub fn view_box_attr(self) -> String {
        format!("0 0 {} {}", format_dim(self.width), format_dim(self.height))
    }

    /// Format the SVG `width` attribute value.
    #[must_use]
    pub fn width_attr(self) -> String {
        format_dim(self.width)
    }

    /// Format the SVG `height` attribute value.
    #[must_use]
    pub fn height_attr(self) -> String {
        format_dim(self.height)
    }
}

fn format_dim(n: f64) -> String {
    if n.is_finite() && n.fract() == 0.0 && n.abs() < 1.0e16 {
        (n as i64).to_string()
    } else {
        format!("{n}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slideglance_utils::Emu;

    fn size(w: i64, h: i64) -> SlideSize {
        SlideSize {
            width: Emu::new(w),
            height: Emu::new(h),
        }
    }

    #[test]
    fn standard_widescreen() {
        // 16:9 widescreen slide at 96 DPI: 9,144,000 x 5,143,500 EMU
        // -> 960 x 540 px.
        let vb = SlideViewBox::from_slide_size(size(9_144_000, 5_143_500));
        assert_eq!(vb.width, 960.0);
        assert_eq!(vb.height, 540.0);
        assert_eq!(vb.view_box_attr(), "0 0 960 540");
        assert_eq!(vb.width_attr(), "960");
        assert_eq!(vb.height_attr(), "540");
    }

    #[test]
    fn standard_4_3() {
        // 4:3 slide: 9,144,000 x 6,858,000 EMU -> 960 x 720 px.
        let vb = SlideViewBox::from_slide_size(size(9_144_000, 6_858_000));
        assert_eq!(vb.view_box_attr(), "0 0 960 720");
    }

    #[test]
    fn fractional_pixel_dimensions() {
        // 1 EMU = 1/9525 px at 96 DPI; choose a value that does not divide
        // cleanly to verify the fallback formatter.
        let vb = SlideViewBox::from_slide_size(size(1_000_000, 1_000_000));
        assert!(vb.width_attr().contains('.'));
        assert!(vb.view_box_attr().starts_with("0 0 "));
    }

    #[test]
    fn zero_size() {
        let vb = SlideViewBox::from_slide_size(size(0, 0));
        assert_eq!(vb.view_box_attr(), "0 0 0 0");
    }
}
