//! `<a:blipFill>` color-adjustment rendering.
//!
//! Direct port of.
//! Produces one `<filter>` definition (when any effect is set) covering
//! the OOXML image-color stack: `grayscl` → `biLevel` → `blur` → `lum`
//! → `duotone` → `clrChange`. Each stage chains via SVG filter
//! `result` / `in` references; absent stages skip without disturbing
//! the chain.
//!
//! The output is meant to be applied to an `<image>`-bearing element
//! (typically wrapping the `<image>` in an inner `<g filter="url(#…)">`),
//! independently of any shape-level [`render_effects`] filter.
//!
//! `clrChange` only supports the `clrTo.alpha == 0` "set transparent
//! color" approximation — exact per-pixel keying is not achievable
//! with SVG filters alone. Other targets render as a no-op (matches
//! TS).
//!
//! [`render_effects`]: crate::effects::render_effects

use std::fmt::Write as _;

use slideglance_color::ResolvedColor;
use slideglance_model::BlipEffects;

use crate::effects::EffectResult;
use crate::geometry::fmt::n;
use crate::id_gen::IdGen;

/// Compute the SVG filter for a `<a:blipFill>` color-adjustment
/// chain. Returns an empty result when:
/// - `blip_effects` is `None`,
/// - none of the configured effects produce a primitive (e.g. a
///   `clrChange` whose `clrTo.alpha` is not `0`).
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn render_blip_effects(blip_effects: Option<&BlipEffects>, ids: &mut IdGen) -> EffectResult {
    let Some(eff) = blip_effects else {
        return EffectResult::default();
    };

    let mut primitives = String::new();
    let mut last_result: String = "SourceGraphic".to_string();

    if eff.grayscale {
        primitives.push_str("<feColorMatrix type=\"saturate\" values=\"0\" result=\"grayscale\"/>");
        last_result = "grayscale".to_string();
    }

    if let Some(bi) = &eff.bi_level {
        if !eff.grayscale {
            let _ = write!(
                primitives,
                "<feColorMatrix in=\"{last_result}\" type=\"saturate\" values=\"0\" result=\"biLevelGray\"/>"
            );
            last_result = "biLevelGray".to_string();
        }
        let tv = build_threshold_table(bi.threshold);
        let _ = write!(
            primitives,
            "<feComponentTransfer in=\"{last_result}\" result=\"biLevel\">"
        );
        let _ = write!(
            primitives,
            "<feFuncR type=\"discrete\" tableValues=\"{tv}\"/>"
        );
        let _ = write!(
            primitives,
            "<feFuncG type=\"discrete\" tableValues=\"{tv}\"/>"
        );
        let _ = write!(
            primitives,
            "<feFuncB type=\"discrete\" tableValues=\"{tv}\"/>"
        );
        primitives.push_str("</feComponentTransfer>");
        last_result = "biLevel".to_string();
    }

    if let Some(blur) = &eff.blur {
        let std_dev = blur.radius.to_pixels() / 2.0;
        let _ = write!(
            primitives,
            "<feGaussianBlur in=\"{last_result}\" stdDeviation=\"{}\" result=\"blipBlur\"/>",
            n(std_dev)
        );
        last_result = "blipBlur".to_string();
    }

    if let Some(lum) = &eff.lum {
        let slope = round_3(1.0 + lum.contrast);
        let intercept = round_3(lum.brightness - lum.contrast / 2.0);
        let _ = write!(
            primitives,
            "<feComponentTransfer in=\"{last_result}\" result=\"lumResult\">"
        );
        let _ = write!(
            primitives,
            "<feFuncR type=\"linear\" slope=\"{}\" intercept=\"{}\"/>",
            n(slope),
            n(intercept)
        );
        let _ = write!(
            primitives,
            "<feFuncG type=\"linear\" slope=\"{}\" intercept=\"{}\"/>",
            n(slope),
            n(intercept)
        );
        let _ = write!(
            primitives,
            "<feFuncB type=\"linear\" slope=\"{}\" intercept=\"{}\"/>",
            n(slope),
            n(intercept)
        );
        primitives.push_str("</feComponentTransfer>");
        last_result = "lumResult".to_string();
    }

    if let Some(d) = &eff.duotone {
        let (r1, g1, b1) = rgb_norm(&d.color1);
        let (r2, g2, b2) = rgb_norm(&d.color2);
        if !eff.grayscale && eff.bi_level.is_none() {
            let _ = write!(
                primitives,
                "<feColorMatrix in=\"{last_result}\" type=\"saturate\" values=\"0\" result=\"duotoneGray\"/>"
            );
            last_result = "duotoneGray".to_string();
        }
        let _ = write!(
            primitives,
            "<feComponentTransfer in=\"{last_result}\" result=\"duotoneResult\">"
        );
        let _ = write!(
            primitives,
            "<feFuncR type=\"table\" tableValues=\"{} {}\"/>",
            n(r1),
            n(r2)
        );
        let _ = write!(
            primitives,
            "<feFuncG type=\"table\" tableValues=\"{} {}\"/>",
            n(g1),
            n(g2)
        );
        let _ = write!(
            primitives,
            "<feFuncB type=\"table\" tableValues=\"{} {}\"/>",
            n(b1),
            n(b2)
        );
        primitives.push_str("</feComponentTransfer>");
        last_result = "duotoneResult".to_string();
    }

    if let Some(cc) = &eff.clr_change {
        // TS only emits the approximation when the target is fully
        // transparent. Other targets are not representable as a single
        // SVG filter and would require per-pixel sampling.
        //
        // Intentional divergence from the spec:
        // The spec computes
        //   alpha = scale * (R + G + B - Rf - Gf - Bf)
        // which is *signed*. For the common `clrFrom = white` case any
        // pixel darker than white produces a strongly negative alpha
        // (e.g. black yields -3·scale) that SVG clamps to 0 — turning
        // every dark pixel transparent alongside the intended white.
        // Slide 30's React logo (a dark blue atom + black "React"
        // text on a white background, with `<a:clrChange>` keying out
        // the white) ended up fully invisible because the atom's dark
        // pixels were also stripped. PowerPoint and the reference PDF
        // keep the dark pixels opaque, so we negate the slope and the
        // intercept to get the inverted formula
        //   alpha = scale * ((Rf + Gf + Bf) - (R + G + B))
        // Pure white: 0. Pure black: +3·scale → clamped to 1. Near-
        // white (0.99…) drops smoothly toward 0 for a soft edge. This
        // is still an approximation (no per-pixel keying), but it
        // matches PowerPoint for the white-to-transparent path that
        // every test deck slide uses.
        if cc.clr_to.alpha == 0.0 {
            let (rf, gf, bf) = rgb_norm(&cc.clr_from);
            let scale: f64 = 20.0;
            // Two-stage filter so RGB is preserved exactly:
            //   1) `feColorMatrix` builds an alpha-only *mask* whose
            //      RGB channels are zero and whose alpha is the
            //      inverted distance-to-clrFrom approximation
            //      (alpha=0 at clrFrom, alpha→1 elsewhere).
            //   2) `feComposite operator="in"` multiplies the original
            //      `SourceGraphic` by the mask's alpha, so opaque pixels
            //      keep their authored colour and clrFrom-matching
            //      pixels drop out entirely.
            //
            // Doing the alpha math as a single 4×5 matrix that also
            // copies RGB straight through worked in principle, but
            // resvg 0.47 stores filter intermediates as premultiplied
            // RGBA — `(1, 1, 1, 0)` collapsed to `(0, 0, 0, 0)`, and
            // the residual edge pixels rendered as a black halo
            // around every clrChange'd image (slide 30's React logo
            // showed up inside a solid black rectangle). Splitting
            // the mask out keeps SourceGraphic unmodified along the
            // colour path.
            let _ = write!(
                primitives,
                "<feColorMatrix in=\"{last_result}\" type=\"matrix\" values=\"0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 {} {} {} 0 {}\" result=\"clrChangeMask\"/>",
                n(round_3(-scale)),
                n(round_3(-scale)),
                n(round_3(-scale)),
                n(round_3(scale * (rf + gf + bf)))
            );
            let _ = write!(
                primitives,
                "<feComposite in=\"{last_result}\" in2=\"clrChangeMask\" operator=\"in\" result=\"clrChangeResult\"/>",
            );
            last_result = "clrChangeResult".to_string();
        }
    }

    if primitives.is_empty() {
        return EffectResult::default();
    }

    let _ = last_result; // last assignment is intentional but read by future callers if any.

    let id = ids.next_id("blip-effect");
    let filter_defs =
        format!("<filter id=\"{id}\" color-interpolation-filters=\"sRGB\">{primitives}</filter>");
    EffectResult {
        filter_attr: format!("filter=\"url(#{id})\""),
        filter_defs,
    }
}

/// Build a discrete-step threshold table for `<feFuncR/G/B>`. Mirrors
/// the spec's 16-step quantization.
fn build_threshold_table(threshold: f64) -> String {
    const STEPS: usize = 16;
    let mut out = String::with_capacity(STEPS * 2);
    for i in 0..STEPS {
        if i > 0 {
            out.push(' ');
        }
        let v = i as f64 / STEPS as f64;
        out.push(if v < threshold { '0' } else { '1' });
    }
    out
}

/// Convert a [`ResolvedColor`] to normalised `(r, g, b)` channel values
/// in `[0, 1]`, rounded to spec precision (`Math.round(x*1000)/1000`).
fn rgb_norm(color: &ResolvedColor) -> (f64, f64, f64) {
    (
        round_3(f64::from(color.rgb.r) / 255.0),
        round_3(f64::from(color.rgb.g) / 255.0),
        round_3(f64::from(color.rgb.b) / 255.0),
    )
}

/// `Math.round(n * 1000) / 1000` — spec's per-blip rounding.
#[inline]
fn round_3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use slideglance_color::{ResolvedColor, Rgb};
    use slideglance_model::{BiLevelEffect, BlurEffect, ClrChangeEffect, DuotoneEffect, LumEffect};
    use slideglance_utils::Emu;

    fn rgb(hex: &str) -> ResolvedColor {
        ResolvedColor::new(Rgb::from_hex(hex).unwrap(), 1.0)
    }

    fn transparent(hex: &str) -> ResolvedColor {
        ResolvedColor::new(Rgb::from_hex(hex).unwrap(), 0.0)
    }

    #[test]
    fn none_returns_empty_result() {
        let mut ids = IdGen::new();
        let r = render_blip_effects(None, &mut ids);
        assert!(r.filter_attr.is_empty());
        assert!(r.filter_defs.is_empty());
    }

    #[test]
    fn empty_blip_effects_returns_empty_result() {
        let mut ids = IdGen::new();
        let r = render_blip_effects(Some(&BlipEffects::default()), &mut ids);
        assert!(r.filter_attr.is_empty());
    }

    #[test]
    fn grayscale_emits_saturate_zero() {
        let mut ids = IdGen::new();
        let eff = BlipEffects {
            grayscale: true,
            ..BlipEffects::default()
        };
        let r = render_blip_effects(Some(&eff), &mut ids);
        assert!(r
            .filter_defs
            .contains("<feColorMatrix type=\"saturate\" values=\"0\" result=\"grayscale\"/>"));
        assert!(r.filter_attr.starts_with("filter=\"url(#blip-effect-"));
    }

    #[test]
    fn bi_level_inserts_grayscale_step_when_grayscale_off() {
        let mut ids = IdGen::new();
        let eff = BlipEffects {
            bi_level: Some(BiLevelEffect { threshold: 0.5 }),
            ..BlipEffects::default()
        };
        let r = render_blip_effects(Some(&eff), &mut ids);
        // A pre-step grayscale conversion is required because biLevel
        // expects a luminance-only input.
        assert!(r.filter_defs.contains("biLevelGray"));
        assert!(r.filter_defs.contains("<feFuncR type=\"discrete\""));
    }

    #[test]
    fn bi_level_skips_extra_grayscale_step_after_grayscale() {
        let mut ids = IdGen::new();
        let eff = BlipEffects {
            grayscale: true,
            bi_level: Some(BiLevelEffect { threshold: 0.5 }),
            ..BlipEffects::default()
        };
        let r = render_blip_effects(Some(&eff), &mut ids);
        assert!(!r.filter_defs.contains("biLevelGray"));
    }

    #[test]
    fn blur_uses_half_radius_as_std_deviation() {
        let mut ids = IdGen::new();
        let eff = BlipEffects {
            blur: Some(BlurEffect {
                radius: Emu::new(914_400),
                grow: false,
            }),
            ..BlipEffects::default()
        };
        let r = render_blip_effects(Some(&eff), &mut ids);
        // 914400 EMU = 96 px / 2 = 48 stdDeviation.
        assert!(r.filter_defs.contains("stdDeviation=\"48\""));
    }

    #[test]
    fn lum_emits_linear_component_transfer() {
        let mut ids = IdGen::new();
        let eff = BlipEffects {
            lum: Some(LumEffect {
                brightness: 0.2,
                contrast: 0.4,
            }),
            ..BlipEffects::default()
        };
        let r = render_blip_effects(Some(&eff), &mut ids);
        // slope = 1 + contrast; intercept = brightness - contrast/2.
        assert!(r.filter_defs.contains("slope=\"1.4\""));
        assert!(r.filter_defs.contains("intercept=\"0\""));
    }

    #[test]
    fn duotone_inserts_grayscale_step_when_off() {
        let mut ids = IdGen::new();
        let eff = BlipEffects {
            duotone: Some(DuotoneEffect {
                color1: rgb("#000000"),
                color2: rgb("#FFFFFF"),
            }),
            ..BlipEffects::default()
        };
        let r = render_blip_effects(Some(&eff), &mut ids);
        assert!(r.filter_defs.contains("duotoneGray"));
        assert!(r
            .filter_defs
            .contains("<feFuncR type=\"table\" tableValues=\"0 1\"/>"));
    }

    #[test]
    fn duotone_after_grayscale_omits_extra_step() {
        let mut ids = IdGen::new();
        let eff = BlipEffects {
            grayscale: true,
            duotone: Some(DuotoneEffect {
                color1: rgb("#000000"),
                color2: rgb("#FFFFFF"),
            }),
            ..BlipEffects::default()
        };
        let r = render_blip_effects(Some(&eff), &mut ids);
        assert!(!r.filter_defs.contains("duotoneGray"));
    }

    #[test]
    fn clr_change_to_transparent_emits_color_matrix() {
        let mut ids = IdGen::new();
        let eff = BlipEffects {
            clr_change: Some(ClrChangeEffect {
                clr_from: rgb("#FFFFFF"),
                clr_to: transparent("#000000"),
            }),
            ..BlipEffects::default()
        };
        let r = render_blip_effects(Some(&eff), &mut ids);
        assert!(r.filter_defs.contains("clrChangeResult"));
        // Two-stage filter: a mask-only feColorMatrix followed by a
        // feComposite "in" that re-applies the original colours.
        // Slope is negated (-20 -20 -20) and intercept positive
        // (3*1.0*20 = 60) so dark pixels stay opaque while near-
        // clrFrom pixels go transparent.
        assert!(r.filter_defs.contains("-20 -20 -20"));
        assert!(r.filter_defs.contains(" 60\""));
        assert!(r.filter_defs.contains("clrChangeMask"));
        assert!(r.filter_defs.contains("operator=\"in\""));
    }

    #[test]
    fn clr_change_to_opaque_target_is_no_op() {
        // Only alpha=0 is supported per TS; an opaque target is silently
        // ignored, leaving the chain empty (and hence no filter at all
        // when clrChange is the only effect set).
        let mut ids = IdGen::new();
        let eff = BlipEffects {
            clr_change: Some(ClrChangeEffect {
                clr_from: rgb("#FFFFFF"),
                clr_to: rgb("#FF0000"),
            }),
            ..BlipEffects::default()
        };
        let r = render_blip_effects(Some(&eff), &mut ids);
        assert!(r.filter_attr.is_empty());
        assert!(r.filter_defs.is_empty());
    }

    #[test]
    fn id_counter_advances_per_blip_effect_minted() {
        let mut ids = IdGen::new();
        let eff = BlipEffects {
            grayscale: true,
            ..BlipEffects::default()
        };
        let r1 = render_blip_effects(Some(&eff), &mut ids);
        let r2 = render_blip_effects(Some(&eff), &mut ids);
        assert!(r1.filter_attr.contains("blip-effect-0"));
        assert!(r2.filter_attr.contains("blip-effect-1"));
    }

    // Threshold table semantic: integer step where i/STEPS first crosses
    // the threshold becomes the 0→1 boundary.
    #[test]
    fn threshold_table_quantises_at_threshold() {
        // threshold 0.5: indices 0..7 are 0, 8..15 are 1.
        let t = build_threshold_table(0.5);
        assert_eq!(t, "0 0 0 0 0 0 0 0 1 1 1 1 1 1 1 1");
    }

    #[test]
    fn threshold_table_threshold_zero_makes_all_one() {
        let t = build_threshold_table(0.0);
        assert!(t.split_whitespace().all(|v| v == "1"));
    }

    #[test]
    fn threshold_table_threshold_one_makes_all_zero() {
        let t = build_threshold_table(1.0);
        assert!(t.split_whitespace().all(|v| v == "0"));
    }
}
