//! `<a:effectLst>` rendering.
//!
//! Direct port of. Emits
//! one `<filter>` definition aggregating all of the shape's effects
//! (soft edge, glow, outer shadow, inner shadow) in OOXML's compositing
//! order. The order is fixed by the spec and matches
//! `PowerPoint` rendering.
//!
//! Returns an empty result for `None` inputs *and* for an
//! `<a:effectLst>` whose four optional children are all `None` — both
//! are valid OOXML and produce no SVG filter.
//!
//! IDs are minted via [`IdGen`] (deterministic counters) instead of
//! `crypto.randomUUID()` so two renders of the same slide are bitwise
//! identical.

use std::fmt::Write as _;

use slideglance_model::EffectList;

use crate::color::{alpha_str, color_hex};
use crate::geometry::fmt::n;
use crate::id_gen::IdGen;

/// Filter attribute + `<defs>` produced by [`render_effects`].
///
/// `filter_attr` is appended to the element's group tag (`<g … filter="…">`),
/// and `filter_defs` is folded into the slide-level `<defs>` block.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EffectResult {
    /// `filter="url(#…)"` fragment, or empty when no filter is applied.
    pub filter_attr: String,
    /// `<filter id="…">…</filter>` definition, or empty.
    pub filter_defs: String,
}

/// Compute the SVG filter for a shape's `<a:effectLst>`.
///
/// Returns an empty result when:
/// - `effects` is `None`,
/// - `effects` is `Some(EffectList)` but every child is `None`.
///
/// Otherwise emits one `<filter>` whose primitives chain in the TS-fixed
/// order: softEdge → glow → outerShadow → innerShadow. Each primitive
/// chains via the SVG `result` / `in` graph through `lastResult`.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn render_effects(effects: Option<&EffectList>, ids: &mut IdGen) -> EffectResult {
    let Some(eff) = effects else {
        return EffectResult::default();
    };
    if eff.outer_shadow.is_none()
        && eff.inner_shadow.is_none()
        && eff.glow.is_none()
        && eff.soft_edge.is_none()
    {
        return EffectResult::default();
    }

    let mut primitives = String::new();
    let mut last_result: String = "SourceGraphic".to_string();

    if let Some(soft) = &eff.soft_edge {
        let r = soft.radius.to_pixels();
        let _ = write!(
            primitives,
            "<feGaussianBlur in=\"SourceAlpha\" stdDeviation=\"{}\" result=\"softEdgeMask\"/>",
            n(r)
        );
        primitives.push_str(
            "<feComposite in=\"SourceGraphic\" in2=\"softEdgeMask\" operator=\"in\" result=\"softEdgeResult\"/>",
        );
        last_result = "softEdgeResult".to_string();
    }

    if let Some(glow) = &eff.glow {
        let r = glow.radius.to_pixels();
        let hex = color_hex(&glow.color);
        let alpha = alpha_str(glow.color.alpha);
        if last_result != "SourceGraphic" {
            let _ = write!(
                primitives,
                "<feColorMatrix in=\"{last_result}\" type=\"matrix\" values=\"0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 1 0\" result=\"glowAlpha\"/>"
            );
        }
        let blur_in = if last_result == "SourceGraphic" {
            "SourceAlpha"
        } else {
            "glowAlpha"
        };
        let merge_in = last_result.clone();
        let _ = write!(
            primitives,
            "<feGaussianBlur in=\"{blur_in}\" stdDeviation=\"{}\" result=\"glowBlur\"/>",
            n(r)
        );
        let _ = write!(
            primitives,
            "<feFlood flood-color=\"{hex}\" flood-opacity=\"{alpha}\" result=\"glowColor\"/>"
        );
        primitives.push_str(
            "<feComposite in=\"glowColor\" in2=\"glowBlur\" operator=\"in\" result=\"glowFinal\"/>",
        );
        primitives.push_str("<feMerge result=\"glowMerge\">");
        primitives.push_str("<feMergeNode in=\"glowFinal\"/>");
        let _ = write!(primitives, "<feMergeNode in=\"{merge_in}\"/>");
        primitives.push_str("</feMerge>");
        last_result = "glowMerge".to_string();
    }

    if let Some(sh) = &eff.outer_shadow {
        let std_dev = sh.blur_radius.to_pixels() / 2.0;
        let dist = sh.distance.to_pixels();
        let dir_rad = (sh.direction * std::f64::consts::PI) / 180.0;
        let dx = round_2(dist * dir_rad.cos());
        let dy = round_2(dist * dir_rad.sin());
        let hex = color_hex(&sh.color);
        let alpha = alpha_str(sh.color.alpha);
        let merge_in = last_result.clone();

        let _ = write!(
            primitives,
            "<feGaussianBlur in=\"SourceAlpha\" stdDeviation=\"{}\" result=\"shadowBlur\"/>",
            n(std_dev)
        );
        let _ = write!(
            primitives,
            "<feOffset in=\"shadowBlur\" dx=\"{}\" dy=\"{}\" result=\"shadowOffset\"/>",
            n(dx),
            n(dy)
        );
        let _ = write!(
            primitives,
            "<feFlood flood-color=\"{hex}\" flood-opacity=\"{alpha}\" result=\"shadowColor\"/>"
        );
        primitives.push_str(
            "<feComposite in=\"shadowColor\" in2=\"shadowOffset\" operator=\"in\" result=\"shadowFinal\"/>",
        );
        primitives.push_str("<feMerge result=\"outerShadowMerge\">");
        primitives.push_str("<feMergeNode in=\"shadowFinal\"/>");
        let _ = write!(primitives, "<feMergeNode in=\"{merge_in}\"/>");
        primitives.push_str("</feMerge>");
        last_result = "outerShadowMerge".to_string();
    }

    if let Some(ish) = &eff.inner_shadow {
        let std_dev = ish.blur_radius.to_pixels() / 2.0;
        let dist = ish.distance.to_pixels();
        let dir_rad = (ish.direction * std::f64::consts::PI) / 180.0;
        let dx = round_2(dist * dir_rad.cos());
        let dy = round_2(dist * dir_rad.sin());
        let hex = color_hex(&ish.color);
        let alpha = alpha_str(ish.color.alpha);
        let source_in = last_result.clone();

        primitives.push_str(
            "<feComponentTransfer in=\"SourceAlpha\" result=\"innerShdwInverse\"><feFuncA type=\"table\" tableValues=\"1 0\"/></feComponentTransfer>",
        );
        let _ = write!(
            primitives,
            "<feGaussianBlur in=\"innerShdwInverse\" stdDeviation=\"{}\" result=\"innerShdwBlur\"/>",
            n(std_dev)
        );
        let _ = write!(
            primitives,
            "<feOffset in=\"innerShdwBlur\" dx=\"{}\" dy=\"{}\" result=\"innerShdwOffset\"/>",
            n(dx),
            n(dy)
        );
        let _ = write!(
            primitives,
            "<feFlood flood-color=\"{hex}\" flood-opacity=\"{alpha}\" result=\"innerShdwFill\"/>"
        );
        primitives.push_str(
            "<feComposite in=\"innerShdwFill\" in2=\"innerShdwOffset\" operator=\"in\" result=\"innerShdwColored\"/>",
        );
        primitives.push_str(
            "<feComposite in=\"innerShdwColored\" in2=\"SourceAlpha\" operator=\"in\" result=\"innerShdwClipped\"/>",
        );
        let _ = write!(
            primitives,
            "<feComposite in=\"innerShdwClipped\" in2=\"{source_in}\" operator=\"over\"/>"
        );
        // The spec assigns `lastResult = ""` here, signalling no
        // further chaining is possible. We mirror that by leaving
        // `last_result` at this empty marker, even though it is no
        // longer read after this branch.
        last_result.clear();
    }

    if primitives.is_empty() {
        return EffectResult::default();
    }

    // Keep `last_result` mention out of unused-warning territory: the
    // variable is read by the `glow`/`outer_shadow` branches and finally
    // cleared by `inner_shadow`; we don't propagate it past the filter.
    let _ = last_result;

    let id = ids.next_id("effect");
    let filter_defs = format!(
        "<filter id=\"{id}\" x=\"-50%\" y=\"-50%\" width=\"200%\" height=\"200%\" color-interpolation-filters=\"sRGB\">{primitives}</filter>"
    );

    EffectResult {
        filter_attr: format!("filter=\"url(#{id})\""),
        filter_defs,
    }
}

/// `Math.round(n * 100) / 100` — spec's per-effect rounding.
#[inline]
fn round_2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use slideglance_color::{ResolvedColor, Rgb};
    use slideglance_model::{Glow, InnerShadow, OuterShadow, SoftEdge};
    use slideglance_utils::Emu;

    fn rgb(hex: &str) -> ResolvedColor {
        ResolvedColor::new(Rgb::from_hex(hex).unwrap(), 1.0)
    }

    fn translucent(hex: &str, alpha: f64) -> ResolvedColor {
        ResolvedColor::new(Rgb::from_hex(hex).unwrap(), alpha)
    }

    #[test]
    fn none_yields_empty_result() {
        let mut ids = IdGen::new();
        let r = render_effects(None, &mut ids);
        assert!(r.filter_attr.is_empty());
        assert!(r.filter_defs.is_empty());
        assert_eq!(ids.peek(), 0);
    }

    #[test]
    fn empty_effect_list_yields_empty_result() {
        let mut ids = IdGen::new();
        let eff = EffectList::default();
        let r = render_effects(Some(&eff), &mut ids);
        assert!(r.filter_attr.is_empty());
        assert!(r.filter_defs.is_empty());
    }

    #[test]
    fn outer_shadow_emits_filter_with_blur_offset_flood_composite() {
        let mut ids = IdGen::new();
        let eff = EffectList {
            outer_shadow: Some(OuterShadow {
                blur_radius: Emu::new(127_000),
                distance: Emu::new(50_800),
                direction: 90.0,
                color: translucent("#000000", 0.1),
                alignment: "bl".to_string(),
                rotate_with_shape: false,
            }),
            ..EffectList::default()
        };
        let r = render_effects(Some(&eff), &mut ids);
        assert!(r.filter_attr.starts_with("filter=\"url(#effect-"));
        assert!(r.filter_defs.contains("<filter id=\"effect-0\""));
        assert!(r.filter_defs.contains("<feGaussianBlur in=\"SourceAlpha\""));
        assert!(r.filter_defs.contains("<feOffset"));
        assert!(r
            .filter_defs
            .contains("flood-color=\"#000000\" flood-opacity=\"0.1\""));
        assert!(r
            .filter_defs
            .contains("<feMerge result=\"outerShadowMerge\""));
    }

    #[test]
    fn outer_shadow_direction_drives_offsets() {
        let mut ids = IdGen::new();
        // Direction 0 (rightward) with non-zero distance gives dx>0, dy=0.
        let eff = EffectList {
            outer_shadow: Some(OuterShadow {
                blur_radius: Emu::new(0),
                distance: Emu::new(914_400),
                direction: 0.0,
                color: rgb("#000000"),
                alignment: "ctr".to_string(),
                rotate_with_shape: false,
            }),
            ..EffectList::default()
        };
        let r = render_effects(Some(&eff), &mut ids);
        // 914400 EMU = 96 px @ 96 DPI; cos(0)=1 sin(0)=0.
        assert!(r.filter_defs.contains("dx=\"96\""));
        assert!(r.filter_defs.contains("dy=\"0\""));
    }

    #[test]
    fn glow_after_soft_edge_promotes_glow_alpha() {
        let mut ids = IdGen::new();
        let eff = EffectList {
            soft_edge: Some(SoftEdge {
                radius: Emu::new(127_000),
            }),
            glow: Some(Glow {
                radius: Emu::new(127_000),
                color: rgb("#FFFF00"),
            }),
            ..EffectList::default()
        };
        let r = render_effects(Some(&eff), &mut ids);
        assert!(r
            .filter_defs
            .contains("<feColorMatrix in=\"softEdgeResult\""));
        assert!(r.filter_defs.contains("<feGaussianBlur in=\"glowAlpha\""));
        assert!(r.filter_defs.contains("flood-color=\"#FFFF00\""));
    }

    #[test]
    fn glow_first_uses_source_alpha_directly() {
        let mut ids = IdGen::new();
        let eff = EffectList {
            glow: Some(Glow {
                radius: Emu::new(127_000),
                color: rgb("#FF00FF"),
            }),
            ..EffectList::default()
        };
        let r = render_effects(Some(&eff), &mut ids);
        // No softEdge ahead of glow, so glow blurs SourceAlpha directly.
        assert!(r.filter_defs.contains("<feGaussianBlur in=\"SourceAlpha\""));
        // No glowAlpha colormatrix should be emitted.
        assert!(!r.filter_defs.contains("result=\"glowAlpha\""));
    }

    #[test]
    fn inner_shadow_emits_inverse_alpha_chain() {
        let mut ids = IdGen::new();
        let eff = EffectList {
            inner_shadow: Some(InnerShadow {
                blur_radius: Emu::new(0),
                distance: Emu::new(0),
                direction: 0.0,
                color: rgb("#000000"),
            }),
            ..EffectList::default()
        };
        let r = render_effects(Some(&eff), &mut ids);
        assert!(r.filter_defs.contains("innerShdwInverse"));
        assert!(r.filter_defs.contains("tableValues=\"1 0\""));
        assert!(r.filter_defs.contains("operator=\"over\""));
    }

    #[test]
    fn soft_edge_emits_alpha_blur_and_composite_in() {
        let mut ids = IdGen::new();
        let eff = EffectList {
            soft_edge: Some(SoftEdge {
                radius: Emu::new(127_000),
            }),
            ..EffectList::default()
        };
        let r = render_effects(Some(&eff), &mut ids);
        assert!(r.filter_defs.contains("<feGaussianBlur in=\"SourceAlpha\""));
        assert!(r
            .filter_defs
            .contains("operator=\"in\" result=\"softEdgeResult\""));
    }

    #[test]
    fn id_counter_advances_per_effect_minted() {
        let mut ids = IdGen::new();
        let eff = EffectList {
            outer_shadow: Some(OuterShadow {
                blur_radius: Emu::new(0),
                distance: Emu::new(0),
                direction: 0.0,
                color: rgb("#000000"),
                alignment: "ctr".to_string(),
                rotate_with_shape: false,
            }),
            ..EffectList::default()
        };
        let r1 = render_effects(Some(&eff), &mut ids);
        let r2 = render_effects(Some(&eff), &mut ids);
        assert!(r1.filter_attr.contains("effect-0"));
        assert!(r2.filter_attr.contains("effect-1"));
    }
}
