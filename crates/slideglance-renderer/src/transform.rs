//! Translate / rotate / flip composition for `<a:xfrm>`.
//!
//! Direct port of. The spec
//! emits `translate(x, y) [rotate(...)] [translate(...) scale(±1, ±1)]` with
//! a single space between segments — bit-for-bit string equality is part of
//! the test contract.
//!
//! Pixel coordinates use the default 96 DPI conversion via [`Emu::to_pixels`].

use slideglance_model::Transform;

use crate::svg_builder::escape_xml_attr;

/// Build the `transform="..."` attribute *value* (the inner string only).
///
/// The output is one or more SVG transform segments separated by spaces. The
/// implementation matches the spec port's emission rules exactly — including
/// the choice to emit `translate(0, 0)` even for zero offsets, since callers
/// rely on the transform always producing at least one segment.
// Single-letter names (`t`/`x`/`y`/`w`/`h`) match the spec's local
// vocabulary; renaming would only obscure parity for readers cross-checking
// against.
#[allow(clippy::many_single_char_names)]
#[must_use]
pub fn build_transform_attr(t: &Transform) -> String {
    let (x, y, w, h) = (
        t.offset_x.to_pixels(),
        t.offset_y.to_pixels(),
        t.extent_width.to_pixels(),
        t.extent_height.to_pixels(),
    );

    let mut parts: Vec<String> = Vec::new();
    parts.push(format!("translate({}, {})", format_num(x), format_num(y)));

    if t.rotation != 0.0 {
        parts.push(format!(
            "rotate({}, {}, {})",
            format_num(t.rotation),
            format_num(w / 2.0),
            format_num(h / 2.0)
        ));
    }

    if t.flip_h || t.flip_v {
        let sx = if t.flip_h { -1 } else { 1 };
        let sy = if t.flip_v { -1 } else { 1 };
        let tx = if t.flip_h { w } else { 0.0 };
        let ty = if t.flip_v { h } else { 0.0 };
        parts.push(format!("translate({}, {})", format_num(tx), format_num(ty)));
        parts.push(format!("scale({sx}, {sy})"));
    }

    parts.join(" ")
}

/// Build an optional ` data-object-name="..."` attribute for a `cNvPr@name`.
///
/// Returns an empty string when the name is `None` or empty so the result can
/// be appended unconditionally to a tag. The leading space is intentional and
/// mirrors the spec port (`buildObjectNameAttr`). Because attribute values are
/// always emitted with double quotes, only `&`, `"`, and `<` need escaping.
#[must_use]
pub fn build_object_name_attr(object_name: Option<&str>) -> String {
    match object_name {
        Some(name) if !name.is_empty() => {
            // The TS port only escapes `&`, `"`, `<` here (not `>`). Match
            // that behaviour exactly so output bytes equal the TS output.
            let mut escaped = String::with_capacity(name.len());
            for ch in name.chars() {
                match ch {
                    '&' => escaped.push_str("&amp;"),
                    '"' => escaped.push_str("&quot;"),
                    '<' => escaped.push_str("&lt;"),
                    other => escaped.push(other),
                }
            }
            format!(" data-object-name=\"{escaped}\"")
        }
        _ => String::new(),
    }
}

/// Format a number for SVG output. JavaScript renders integral floats as
/// `"0"`, `"96"` rather than `"0.0"`. Match that so byte-equality with the TS
/// reference holds for the integer cases used in tests.
fn format_num(n: f64) -> String {
    if n.is_finite() && n.fract() == 0.0 && n.abs() < 1.0e16 {
        // Print with no decimal point — `-0` is collapsed to `0` to match JS
        // `Number.prototype.toString`.
        let i = n as i64;
        if i == 0 {
            "0".to_string()
        } else {
            i.to_string()
        }
    } else {
        // Fractional value: use Rust's default float formatting. We keep the
        // value verbatim; tests do not exercise this path because OOXML
        // dimensions in EMU divide evenly by 9525 at 96 DPI.
        format!("{n}")
    }
}

/// Re-export of the attribute escape so callers do not need a second import
/// just for inline `escape_xml_attr` usage in nearby tag builders.
#[doc(hidden)]
pub fn _escape_attr(s: &str) -> String {
    escape_xml_attr(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use slideglance_utils::Emu;

    fn xfrm() -> Transform {
        Transform {
            offset_x: Emu::new(0),
            offset_y: Emu::new(0),
            extent_width: Emu::new(914_400),
            extent_height: Emu::new(914_400),
            rotation: 0.0,
            flip_h: false,
            flip_v: false,
        }
    }

    // The following tests are 1:1 ports of
    // . String equality is
    // intentional — drift from the TS output would change SVG bytes.

    #[test]
    fn translate_only_when_no_rotation_or_flip() {
        let t = Transform {
            offset_x: Emu::new(914_400),
            offset_y: Emu::new(914_400),
            ..xfrm()
        };
        assert_eq!(build_transform_attr(&t), "translate(96, 96)");
    }

    #[test]
    fn translate_zero_for_zero_offsets() {
        assert_eq!(build_transform_attr(&xfrm()), "translate(0, 0)");
    }

    #[test]
    fn translate_plus_rotate() {
        let t = Transform {
            offset_x: Emu::new(914_400),
            offset_y: Emu::new(914_400),
            extent_width: Emu::new(1_828_800),
            extent_height: Emu::new(914_400),
            rotation: 45.0,
            ..xfrm()
        };
        assert_eq!(
            build_transform_attr(&t),
            "translate(96, 96) rotate(45, 96, 48)"
        );
    }

    #[test]
    fn rotate_center_is_half_width_half_height() {
        let t = Transform {
            extent_width: Emu::new(1_828_800),
            extent_height: Emu::new(1_828_800),
            rotation: 90.0,
            ..xfrm()
        };
        assert_eq!(
            build_transform_attr(&t),
            "translate(0, 0) rotate(90, 96, 96)"
        );
    }

    #[test]
    fn omits_rotate_when_zero() {
        let t = Transform {
            rotation: 0.0,
            ..xfrm()
        };
        assert!(!build_transform_attr(&t).contains("rotate"));
    }

    #[test]
    fn handles_negative_rotation() {
        let t = Transform {
            rotation: -45.0,
            ..xfrm()
        };
        assert!(build_transform_attr(&t).contains("rotate(-45, 48, 48)"));
    }

    #[test]
    fn handles_360_rotation() {
        let t = Transform {
            rotation: 360.0,
            ..xfrm()
        };
        assert!(build_transform_attr(&t).contains("rotate(360, 48, 48)"));
    }

    #[test]
    fn flip_h_only() {
        let t = Transform {
            extent_width: Emu::new(1_828_800),
            extent_height: Emu::new(914_400),
            flip_h: true,
            ..xfrm()
        };
        let out = build_transform_attr(&t);
        assert!(out.contains("translate(192, 0)"));
        assert!(out.contains("scale(-1, 1)"));
    }

    #[test]
    fn flip_v_only() {
        let t = Transform {
            extent_width: Emu::new(1_828_800),
            extent_height: Emu::new(914_400),
            flip_v: true,
            ..xfrm()
        };
        let out = build_transform_attr(&t);
        assert!(out.contains("translate(0, 96)"));
        assert!(out.contains("scale(1, -1)"));
    }

    #[test]
    fn both_flips() {
        let t = Transform {
            extent_width: Emu::new(1_828_800),
            extent_height: Emu::new(914_400),
            flip_h: true,
            flip_v: true,
            ..xfrm()
        };
        let out = build_transform_attr(&t);
        assert!(out.contains("translate(192, 96)"));
        assert!(out.contains("scale(-1, -1)"));
    }

    #[test]
    fn rotation_plus_flip_h() {
        let t = Transform {
            offset_x: Emu::new(914_400),
            offset_y: Emu::new(914_400),
            extent_width: Emu::new(1_828_800),
            extent_height: Emu::new(914_400),
            rotation: 45.0,
            flip_h: true,
            ..xfrm()
        };
        assert_eq!(
            build_transform_attr(&t),
            "translate(96, 96) rotate(45, 96, 48) translate(192, 0) scale(-1, 1)"
        );
    }

    #[test]
    fn rotation_plus_flip_v() {
        let t = Transform {
            extent_width: Emu::new(1_828_800),
            extent_height: Emu::new(914_400),
            rotation: 90.0,
            flip_v: true,
            ..xfrm()
        };
        assert_eq!(
            build_transform_attr(&t),
            "translate(0, 0) rotate(90, 96, 48) translate(0, 96) scale(1, -1)"
        );
    }

    #[test]
    fn rotation_plus_both_flips() {
        let t = Transform {
            extent_width: Emu::new(1_828_800),
            extent_height: Emu::new(914_400),
            rotation: 180.0,
            flip_h: true,
            flip_v: true,
            ..xfrm()
        };
        assert_eq!(
            build_transform_attr(&t),
            "translate(0, 0) rotate(180, 96, 48) translate(192, 96) scale(-1, -1)"
        );
    }

    // --- buildObjectNameAttr ---

    #[test]
    fn object_name_none_returns_empty() {
        assert_eq!(build_object_name_attr(None), "");
    }

    #[test]
    fn object_name_empty_returns_empty() {
        assert_eq!(build_object_name_attr(Some("")), "");
    }

    #[test]
    fn object_name_simple() {
        assert_eq!(
            build_object_name_attr(Some("Title 1")),
            " data-object-name=\"Title 1\""
        );
    }

    #[test]
    fn object_name_escapes_amp_quote_lt_only() {
        // The spec does not escape `>` here.
        assert_eq!(
            build_object_name_attr(Some("a & \"b\" < c > d")),
            " data-object-name=\"a &amp; &quot;b&quot; &lt; c > d\""
        );
    }
}
