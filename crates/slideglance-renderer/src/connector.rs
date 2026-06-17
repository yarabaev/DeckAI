//! `<p:cxnSp>` connector rendering.
//!
//! Direct port of 's
//! `renderConnector`. Connectors are stroke-only — `fill` is forced to
//! `"none"` so the geometry path acts as a line/curve.
//!
//! When the geometry renderer cannot resolve the preset (returns an
//! empty string), the spec falls back to a simple `<line
//! x1="0" y1="0" x2="w" y2="h"/>`. The Rust port matches that fallback
//! exactly so connectors with unsupported preset names still produce
//! diff-stable output.

use std::fmt::Write as _;

use slideglance_model::ConnectorElement;

use crate::effects::render_effects;
use crate::fill::{render_markers, render_outline_attrs};
use crate::geometry::fmt::n;
use crate::geometry::render_geometry;
use crate::id_gen::IdGen;
use crate::render_result::RenderResult;
use crate::transform::{build_object_name_attr, build_transform_attr};

/// Render one [`ConnectorElement`].
///
/// Connectors do not carry a text body, so the only TS-renderable
/// feature surface here is geometry + outline + markers + effects;
/// none of these can fail. Returns a plain [`RenderResult`].
///
/// `font_size_correction` is the cumulative inverse of every ancestor
/// `<g transform="scale(...)">` factor — see [`render_outline_attrs`]'s
/// `stroke_scale` for the rationale. Pass `1.0` for a connector that is
/// not nested inside a scaled group.
#[must_use]
pub fn render_connector(
    connector: &ConnectorElement,
    ids: &mut IdGen,
    font_size_correction: f64,
) -> RenderResult {
    let effect_result = render_effects(connector.effects.as_ref(), ids);

    let transform = &connector.transform;
    let w = transform.extent_width.to_pixels();
    let h = transform.extent_height.to_pixels();

    let transform_attr = build_transform_attr(transform);
    let outline_result =
        render_outline_attrs(connector.outline.as_ref(), ids, font_size_correction);
    let marker_result = render_markers(connector.outline.as_ref(), ids);

    let geometry_svg = render_geometry(&connector.geometry, w, h);

    let mut defs = String::new();
    defs.push_str(&outline_result.defs);
    defs.push_str(&marker_result.defs);
    defs.push_str(&effect_result.filter_defs);

    let filter_attr = if effect_result.filter_attr.is_empty() {
        String::new()
    } else {
        format!(" {}", effect_result.filter_attr)
    };
    let marker_attr_str = combine_marker_attrs(&marker_result.start_attr, &marker_result.end_attr);
    let name_attr = build_object_name_attr(connector.object_name.as_deref());
    let sp_id_attr = crate::svg_builder::build_sp_id_attr(connector.sp_id);

    let mut content = String::new();
    let _ = write!(
        content,
        "<g{sp_id_attr} transform=\"{transform_attr}\"{filter_attr}{name_attr}>"
    );

    if geometry_svg.is_empty() {
        // TS fallback: simple straight line from (0,0) to (w, h).
        let _ = write!(
            content,
            "<line x1=\"0\" y1=\"0\" x2=\"{}\" y2=\"{}\" {} fill=\"none\"{marker_attr_str}/>",
            n(w),
            n(h),
            outline_result.attrs
        );
    } else {
        let inline = format!("{} fill=\"none\"{marker_attr_str}", outline_result.attrs);
        content.push_str(&inject_attrs_after_tag(&geometry_svg, &inline));
    }

    content.push_str("</g>");

    RenderResult { content, defs }
}

fn combine_marker_attrs(start_attr: &str, end_attr: &str) -> String {
    match (start_attr.is_empty(), end_attr.is_empty()) {
        (true, true) => String::new(),
        (false, true) => format!(" {start_attr}"),
        (true, false) => format!(" {end_attr}"),
        (false, false) => format!(" {start_attr} {end_attr}"),
    }
}

fn inject_attrs_after_tag(svg: &str, attrs: &str) -> String {
    let bytes = svg.as_bytes();
    if bytes.first() != Some(&b'<') {
        return svg.to_string();
    }
    let mut i = 1;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_alphanumeric() || c == b'_' {
            i += 1;
        } else {
            break;
        }
    }
    if i == 1 {
        return svg.to_string();
    }
    let mut out = String::with_capacity(svg.len() + attrs.len() + 1);
    out.push_str(&svg[..i]);
    out.push(' ');
    out.push_str(attrs);
    out.push_str(&svg[i..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use slideglance_color::{ResolvedColor, Rgb};
    use slideglance_model::{
        ArrowEndpoint, ArrowSize, ArrowType, ConnectorElement, DashStyle, EffectList, Geometry,
        Glow, Outline, OutlineFill, PresetGeometry, SolidFill, Transform,
    };
    use slideglance_utils::Emu;
    use std::collections::BTreeMap;

    fn line_geometry() -> Geometry {
        Geometry::Preset(PresetGeometry {
            preset: "line".to_string(),
            adjust_values: BTreeMap::new(),
        })
    }

    fn opaque(hex: &str) -> ResolvedColor {
        ResolvedColor::new(Rgb::from_hex(hex).unwrap(), 1.0)
    }

    fn solid_outline() -> Outline {
        Outline {
            width: Emu::new(12_700),
            fill: Some(OutlineFill::Solid(SolidFill {
                color: opaque("#000000"),
            })),
            dash_style: DashStyle::Solid,
            custom_dash: None,
            line_cap: None,
            line_join: None,
            head_end: None,
            tail_end: None,
        }
    }

    fn basic_connector() -> ConnectorElement {
        ConnectorElement {
            sp_id: None,
            transform: Transform {
                offset_x: Emu::new(0),
                offset_y: Emu::new(0),
                extent_width: Emu::new(914_400),
                extent_height: Emu::new(914_400),
                ..Transform::default()
            },
            geometry: line_geometry(),
            outline: Some(solid_outline()),
            effects: None,
            alt_text: None,
            object_name: None,
            hidden: false,
        }
    }

    #[test]
    fn straight_line_emits_stroke_with_fill_none() {
        let mut ids = IdGen::new();
        let r = render_connector(&basic_connector(), &mut ids, 1.0);
        assert!(r.content.starts_with("<g transform=\""), "{}", r.content);
        assert!(r.content.contains("stroke=\"#000000\""));
        assert!(r.content.contains("fill=\"none\""));
        assert!(r.content.ends_with("</g>"));
    }

    #[test]
    fn connector_emits_data_sp_id_when_present() {
        let mut c = basic_connector();
        c.sp_id = Some(7);
        let mut ids = IdGen::new();
        let r = render_connector(&c, &mut ids, 1.0);
        assert!(
            r.content.contains("data-sp-id=\"7\""),
            "data-sp-id missing: {}",
            r.content
        );
        assert!(
            r.content.starts_with("<g data-sp-id=\"7\" transform=\""),
            "unexpected ordering: {}",
            r.content
        );
    }

    #[test]
    fn connector_omits_data_sp_id_when_none() {
        let mut c = basic_connector();
        c.sp_id = None;
        let mut ids = IdGen::new();
        let r = render_connector(&c, &mut ids, 1.0);
        assert!(
            !r.content.contains("data-sp-id"),
            "data-sp-id should be absent: {}",
            r.content
        );
    }

    #[test]
    fn unknown_preset_falls_back_to_rect_with_stroke() {
        // The geometry renderer's catch-all returns a `<rect>` for any
        // unknown preset, so the connector's outline + fill="none"
        // attributes are inlined into that rect. The line-fallback
        // branch in render_connector is reserved for the case where
        // render_geometry returns a literally empty string — currently
        // unreachable from the geometry renderer but kept for parity
        // with.
        let mut c = basic_connector();
        c.geometry = Geometry::Preset(PresetGeometry {
            preset: "definitelyNoSuchPreset".to_string(),
            adjust_values: BTreeMap::new(),
        });
        let mut ids = IdGen::new();
        let r = render_connector(&c, &mut ids, 1.0);
        assert!(r.content.contains("<rect stroke-width="), "{}", r.content);
        assert!(r.content.contains("fill=\"none\""));
    }

    #[test]
    fn marker_endpoints_are_emitted_in_attrs_and_defs() {
        let mut c = basic_connector();
        let outline = c.outline.as_mut().unwrap();
        outline.tail_end = Some(ArrowEndpoint {
            ty: ArrowType::Triangle,
            width: ArrowSize::Med,
            length: ArrowSize::Med,
        });
        let mut ids = IdGen::new();
        let r = render_connector(&c, &mut ids, 1.0);
        assert!(
            r.content.contains("marker-end=\"url(#marker-"),
            "{}",
            r.content
        );
        assert!(r.defs.contains("<marker"));
    }

    #[test]
    fn populated_effects_emit_filter_attribute_and_def() {
        let mut c = basic_connector();
        c.effects = Some(EffectList {
            glow: Some(Glow {
                radius: Emu::new(127_000),
                color: opaque("#FFFF00"),
            }),
            ..EffectList::default()
        });
        let mut ids = IdGen::new();
        let r = render_connector(&c, &mut ids, 1.0);
        assert!(
            r.content.contains(" filter=\"url(#effect-"),
            "{}",
            r.content
        );
        assert!(r.defs.contains("<filter id=\"effect-"));
    }

    #[test]
    fn empty_effect_list_passes_through() {
        let mut c = basic_connector();
        c.effects = Some(EffectList::default());
        let mut ids = IdGen::new();
        let r = render_connector(&c, &mut ids, 1.0);
        assert!(!r.content.contains("filter="));
    }

    #[test]
    fn object_name_appears_as_data_attribute() {
        let mut c = basic_connector();
        c.object_name = Some("Connector 1".to_string());
        let mut ids = IdGen::new();
        let r = render_connector(&c, &mut ids, 1.0);
        assert!(r.content.contains("data-object-name=\"Connector 1\""));
    }
}
