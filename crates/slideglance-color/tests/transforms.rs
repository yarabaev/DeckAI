//! Ported from the spec.
//!
//! Where the TypeScript tests asserted on hex strings (which preserve the
//! input case for pass-through and lowercase for transformed output), the
//! ports below assert on `Rgb` directly — that is the underlying semantic
//! both implementations agree on.

use slideglance_color::{apply_color_transforms, ColorTransform, PerMille, ResolvedColor, Rgb};

fn opaque(rgb: Rgb) -> ResolvedColor {
    ResolvedColor::opaque(rgb)
}

#[test]
fn returns_unchanged_color_when_no_transforms() {
    let result = apply_color_transforms(
        opaque(Rgb::new(0xFF, 0x00, 0x00)),
        &ColorTransform::default(),
    );
    assert_eq!(result, ResolvedColor::new(Rgb::new(0xFF, 0x00, 0x00), 1.0));
}

// --- lumMod / lumOff ---

#[test]
fn lum_mod_50pct_darkens_gray_to_404040() {
    // #808080 -> HSL(0, 0, 0.502) -> lumMod 50% -> l=0.251 -> ~#404040
    let result = apply_color_transforms(
        opaque(Rgb::new(0x80, 0x80, 0x80)),
        &ColorTransform {
            lum_mod: Some(PerMille::new(50_000)),
            ..Default::default()
        },
    );
    assert_eq!(result.rgb, Rgb::new(0x40, 0x40, 0x40));
    assert_eq!(result.alpha, 1.0);
}

#[test]
fn lum_mod_100pct_is_identity() {
    let result = apply_color_transforms(
        opaque(Rgb::new(0x80, 0x80, 0x80)),
        &ColorTransform {
            lum_mod: Some(PerMille::new(100_000)),
            ..Default::default()
        },
    );
    assert_eq!(result.rgb, Rgb::new(0x80, 0x80, 0x80));
}

#[test]
fn lum_off_brightens_black_to_gray() {
    // #000000 -> HSL(0, 0, 0) -> lumOff +50% -> l=0.5 -> #808080
    let result = apply_color_transforms(
        opaque(Rgb::new(0x00, 0x00, 0x00)),
        &ColorTransform {
            lum_off: Some(PerMille::new(50_000)),
            ..Default::default()
        },
    );
    assert_eq!(result.rgb, Rgb::new(0x80, 0x80, 0x80));
}

#[test]
fn lum_mod_and_lum_off_combine() {
    // #808080 -> l=0.502 -> lumMod 75% + lumOff 25% -> l = 0.502*0.75 + 0.25 = 0.6265
    let base = opaque(Rgb::new(0x80, 0x80, 0x80));
    let result = apply_color_transforms(
        base,
        &ColorTransform {
            lum_mod: Some(PerMille::new(75_000)),
            lum_off: Some(PerMille::new(25_000)),
            ..Default::default()
        },
    );
    assert_ne!(result.rgb, base.rgb);
    assert_eq!(result.alpha, 1.0);
}

#[test]
fn lum_clamps_to_one_when_overflowed() {
    let result = apply_color_transforms(
        opaque(Rgb::new(0xFF, 0xFF, 0xFF)),
        &ColorTransform {
            lum_mod: Some(PerMille::new(200_000)),
            lum_off: Some(PerMille::new(50_000)),
            ..Default::default()
        },
    );
    assert_eq!(result.rgb, Rgb::new(0xFF, 0xFF, 0xFF));
}

#[test]
fn lum_clamps_to_zero_when_lum_mod_is_zero() {
    let result = apply_color_transforms(
        opaque(Rgb::new(0x80, 0x80, 0x80)),
        &ColorTransform {
            lum_mod: Some(PerMille::new(0)),
            ..Default::default()
        },
    );
    assert_eq!(result.rgb, Rgb::new(0x00, 0x00, 0x00));
}

// --- tint (blend toward white) ---

#[test]
fn tint_50pct_on_red() {
    // #FF0000 -> tint 50% -> r=255+(255-255)*0.5=255, g=0+128=128, b=0+128=128
    let result = apply_color_transforms(
        opaque(Rgb::new(0xFF, 0x00, 0x00)),
        &ColorTransform {
            tint: Some(PerMille::new(50_000)),
            ..Default::default()
        },
    );
    assert_eq!(result.rgb, Rgb::new(0xFF, 0x80, 0x80));
}

#[test]
fn tint_100pct_yields_white() {
    let result = apply_color_transforms(
        opaque(Rgb::new(0x00, 0x00, 0x00)),
        &ColorTransform {
            tint: Some(PerMille::new(100_000)),
            ..Default::default()
        },
    );
    assert_eq!(result.rgb, Rgb::new(0xFF, 0xFF, 0xFF));
}

#[test]
fn tint_0pct_is_identity() {
    let result = apply_color_transforms(
        opaque(Rgb::new(0xFF, 0x00, 0x00)),
        &ColorTransform {
            tint: Some(PerMille::new(0)),
            ..Default::default()
        },
    );
    assert_eq!(result.rgb, Rgb::new(0xFF, 0x00, 0x00));
}

#[test]
fn tint_on_white_is_no_op() {
    let result = apply_color_transforms(
        opaque(Rgb::new(0xFF, 0xFF, 0xFF)),
        &ColorTransform {
            tint: Some(PerMille::new(50_000)),
            ..Default::default()
        },
    );
    assert_eq!(result.rgb, Rgb::new(0xFF, 0xFF, 0xFF));
}

// --- shade (blend toward black) ---

#[test]
fn shade_50pct_on_red() {
    // #FF0000 -> shade 50% -> r=255*0.5=128, g=0, b=0
    let result = apply_color_transforms(
        opaque(Rgb::new(0xFF, 0x00, 0x00)),
        &ColorTransform {
            shade: Some(PerMille::new(50_000)),
            ..Default::default()
        },
    );
    assert_eq!(result.rgb, Rgb::new(0x80, 0x00, 0x00));
}

#[test]
fn shade_0pct_yields_black() {
    let result = apply_color_transforms(
        opaque(Rgb::new(0xFF, 0xFF, 0xFF)),
        &ColorTransform {
            shade: Some(PerMille::new(0)),
            ..Default::default()
        },
    );
    assert_eq!(result.rgb, Rgb::new(0x00, 0x00, 0x00));
}

#[test]
fn shade_100pct_is_identity() {
    let result = apply_color_transforms(
        opaque(Rgb::new(0xFF, 0x00, 0x00)),
        &ColorTransform {
            shade: Some(PerMille::new(100_000)),
            ..Default::default()
        },
    );
    assert_eq!(result.rgb, Rgb::new(0xFF, 0x00, 0x00));
}

#[test]
fn shade_on_black_is_no_op() {
    let result = apply_color_transforms(
        opaque(Rgb::new(0x00, 0x00, 0x00)),
        &ColorTransform {
            shade: Some(PerMille::new(50_000)),
            ..Default::default()
        },
    );
    assert_eq!(result.rgb, Rgb::new(0x00, 0x00, 0x00));
}

// --- alpha ---

#[test]
fn alpha_50pct() {
    let result = apply_color_transforms(
        opaque(Rgb::new(0xFF, 0x00, 0x00)),
        &ColorTransform {
            alpha: Some(PerMille::new(50_000)),
            ..Default::default()
        },
    );
    assert_eq!(result.rgb, Rgb::new(0xFF, 0x00, 0x00));
    assert_eq!(result.alpha, 0.5);
}

#[test]
fn alpha_0pct_is_fully_transparent() {
    let result = apply_color_transforms(
        opaque(Rgb::new(0xFF, 0x00, 0x00)),
        &ColorTransform {
            alpha: Some(PerMille::new(0)),
            ..Default::default()
        },
    );
    assert_eq!(result.alpha, 0.0);
}

#[test]
fn alpha_100pct() {
    let result = apply_color_transforms(
        opaque(Rgb::new(0xFF, 0x00, 0x00)),
        &ColorTransform {
            alpha: Some(PerMille::new(100_000)),
            ..Default::default()
        },
    );
    assert_eq!(result.alpha, 1.0);
}

// --- compound transforms ---

#[test]
fn applies_multiple_transforms_in_sequence() {
    let result = apply_color_transforms(
        opaque(Rgb::new(0x44, 0x72, 0xC4)),
        &ColorTransform {
            lum_mod: Some(PerMille::new(75_000)),
            tint: Some(PerMille::new(20_000)),
            alpha: Some(PerMille::new(80_000)),
            ..Default::default()
        },
    );
    assert_eq!(result.alpha, 0.8);
    // The original test only asserts truthy hex; we just assert the color
    // changed away from the input.
    assert_ne!(result.rgb, Rgb::new(0x44, 0x72, 0xC4));
}

// --- HSL roundtrip preservation through lumMod 100% ---

#[test]
fn preserves_pure_red_through_lum_mod_100pct() {
    let result = apply_color_transforms(
        opaque(Rgb::new(0xFF, 0x00, 0x00)),
        &ColorTransform {
            lum_mod: Some(PerMille::new(100_000)),
            ..Default::default()
        },
    );
    assert_eq!(result.rgb, Rgb::new(0xFF, 0x00, 0x00));
}

#[test]
fn preserves_pure_green_through_lum_mod_100pct() {
    let result = apply_color_transforms(
        opaque(Rgb::new(0x00, 0xFF, 0x00)),
        &ColorTransform {
            lum_mod: Some(PerMille::new(100_000)),
            ..Default::default()
        },
    );
    assert_eq!(result.rgb, Rgb::new(0x00, 0xFF, 0x00));
}

#[test]
fn preserves_pure_blue_through_lum_mod_100pct() {
    let result = apply_color_transforms(
        opaque(Rgb::new(0x00, 0x00, 0xFF)),
        &ColorTransform {
            lum_mod: Some(PerMille::new(100_000)),
            ..Default::default()
        },
    );
    assert_eq!(result.rgb, Rgb::new(0x00, 0x00, 0xFF));
}

#[test]
fn preserves_white_through_lum_mod_100pct() {
    let result = apply_color_transforms(
        opaque(Rgb::new(0xFF, 0xFF, 0xFF)),
        &ColorTransform {
            lum_mod: Some(PerMille::new(100_000)),
            ..Default::default()
        },
    );
    assert_eq!(result.rgb, Rgb::new(0xFF, 0xFF, 0xFF));
}

#[test]
fn preserves_black_through_lum_mod_100pct() {
    let result = apply_color_transforms(
        opaque(Rgb::new(0x00, 0x00, 0x00)),
        &ColorTransform {
            lum_mod: Some(PerMille::new(100_000)),
            ..Default::default()
        },
    );
    assert_eq!(result.rgb, Rgb::new(0x00, 0x00, 0x00));
}

// --- satMod / satOff (project-level divergence from TS) ---

#[test]
fn sat_mod_full_desaturates_to_grayscale() {
    // satMod=0 -> S = 0 -> chroma collapses to luminance gray.
    let result = apply_color_transforms(
        opaque(Rgb::new(0xFF, 0x00, 0x00)),
        &ColorTransform {
            sat_mod: Some(PerMille::new(0)),
            ..Default::default()
        },
    );
    assert_eq!(result.rgb.r, result.rgb.g);
    assert_eq!(result.rgb.g, result.rgb.b);
}

#[test]
fn sat_mod_100pct_is_identity() {
    let result = apply_color_transforms(
        opaque(Rgb::new(0x44, 0x72, 0xC4)),
        &ColorTransform {
            sat_mod: Some(PerMille::new(100_000)),
            ..Default::default()
        },
    );
    assert_eq!(result.rgb, Rgb::new(0x44, 0x72, 0xC4));
}

#[test]
fn sat_off_negative_desaturates() {
    // satOff = -1.0 -> drives saturation toward zero (clamped).
    let result = apply_color_transforms(
        opaque(Rgb::new(0xFF, 0x00, 0x00)),
        &ColorTransform {
            sat_off: Some(PerMille::new(-100_000)),
            ..Default::default()
        },
    );
    assert_eq!(result.rgb.r, result.rgb.g);
    assert_eq!(result.rgb.g, result.rgb.b);
}

#[test]
fn sat_off_zero_is_identity() {
    let result = apply_color_transforms(
        opaque(Rgb::new(0x44, 0x72, 0xC4)),
        &ColorTransform {
            sat_off: Some(PerMille::new(0)),
            ..Default::default()
        },
    );
    assert_eq!(result.rgb, Rgb::new(0x44, 0x72, 0xC4));
}

#[test]
fn sat_mod_combined_with_lum_mod_shares_hsl_round_trip() {
    // satMod + lumMod must compose in a single HSL pass (matches the
    // single-conversion contract documented on apply_color_transforms).
    let combined = apply_color_transforms(
        opaque(Rgb::new(0x44, 0x72, 0xC4)),
        &ColorTransform {
            sat_mod: Some(PerMille::new(80_000)),
            lum_mod: Some(PerMille::new(75_000)),
            ..Default::default()
        },
    );
    // Smoke check: result is darker (lumMod < 1) and less saturated
    // than the input but still distinguishably colored.
    assert!(combined.rgb.r != 0 || combined.rgb.g != 0 || combined.rgb.b != 0);
    let max = combined.rgb.r.max(combined.rgb.g).max(combined.rgb.b);
    let min = combined.rgb.r.min(combined.rgb.g).min(combined.rgb.b);
    assert!(max > min, "sat_mod=0.8 should retain some chroma");
}

#[test]
fn sat_mod_clamps_above_one() {
    // satMod=2.0 saturates beyond 100% — clamp to 1.0 (no overflow).
    let result = apply_color_transforms(
        opaque(Rgb::new(0x80, 0x80, 0x80)),
        &ColorTransform {
            sat_mod: Some(PerMille::new(200_000)),
            ..Default::default()
        },
    );
    // Pure gray (S=0) stays gray under any satMod multiplier.
    assert_eq!(result.rgb.r, result.rgb.g);
    assert_eq!(result.rgb.g, result.rgb.b);
}
