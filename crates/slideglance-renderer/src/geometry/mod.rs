//! Geometry rendering — preset and custom shape -> SVG element string.
//!
//! Direct port of. Output is a single
//! SVG element (`<rect>`, `<ellipse>`, `<polygon>`, `<path>`, or a `<g>`
//! wrapper for multi-path custom geometries) with no fill/stroke attributes
//! — those are injected later by the shape renderer via prefix replacement,
//! so this layer must keep the leading tag self-closing-friendly.

use slideglance_model::{CustomGeometryPath, Geometry};

pub(crate) mod fmt;
mod preset;

pub use preset::preset_geometry_svg;

/// Render a [`Geometry`] inside the local `width x height` box.
///
/// Returns an empty string only when both branches fail to produce output —
/// the spec instead falls back to `<rect width=W height=H/>`, which
/// we mirror so callers can rely on a non-empty result.
#[must_use]
pub fn render_geometry(geometry: &Geometry, width: f64, height: f64) -> String {
    match geometry {
        Geometry::Preset(p) => preset_geometry_svg(&p.preset, width, height, &p.adjust_values),
        Geometry::Custom(c) if !c.paths.is_empty() => {
            render_custom_geometry(&c.paths, width, height)
        }
        Geometry::Custom(_) => fallback_rect(width, height),
    }
}

fn render_custom_geometry(paths: &[CustomGeometryPath], shape_w: f64, shape_h: f64) -> String {
    if paths.len() == 1 {
        return render_custom_path(&paths[0], shape_w, shape_h);
    }
    let mut out = String::from("<g>");
    for p in paths {
        out.push_str(&render_custom_path(p, shape_w, shape_h));
    }
    out.push_str("</g>");
    out
}

fn render_custom_path(path: &CustomGeometryPath, shape_w: f64, shape_h: f64) -> String {
    let scale_x = if path.width > 0.0 {
        shape_w / path.width
    } else {
        1.0
    };
    let scale_y = if path.height > 0.0 {
        shape_h / path.height
    } else {
        1.0
    };
    // Bake the scale into the path coordinates rather than emitting a
    // `transform="scale(sx, sy)"` attribute. SVG `transform` scales the
    // stroke alongside the geometry, and on shapes whose path coords are
    // in OOXML EMU-style ranges (e.g. 0..3024000) the resulting scale
    // factor is on the order of 1e-4. Applied to a `stroke-width=1`,
    // that collapses the stroke to ~1e-4 SVG units — invisible on the
    // raster output. resvg also doesn't honour
    // `vector-effect="non-scaling-stroke"`, so the only portable fix is
    // to multiply each x/y coord through ahead of time and emit the
    // path with an identity transform.
    let scaled = scale_path_d(&path.commands, scale_x, scale_y);
    format!("<path d=\"{scaled}\"/>")
}

/// Multiply each x/y coordinate in an SVG path-data string by `sx` / `sy`.
///
/// Recognises the command letters our [`CustomGeometryPath`] builder emits
/// (`M`, `L`, `C`, `Q`, `A`, `Z`) plus their lowercase relative variants for
/// safety. For the elliptical-arc command (`A`), the radii (rx, ry) scale
/// with x/y, the rotation/large-arc/sweep flags pass through untouched, and
/// the end-point coords scale with x/y. Numbers can be separated by spaces
/// or commas.
fn scale_path_d(d: &str, sx: f64, sy: f64) -> String {
    let mut out = String::with_capacity(d.len());
    // Per-command coordinate role pattern: each entry maps an argument
    // index (within one full instance of the command) to either Some(true)
    // for an x-coord, Some(false) for a y-coord, or None for a flag/scalar
    // that must pass through unchanged.
    let arc_pattern: &[Option<bool>] = &[
        Some(true),  // rx
        Some(false), // ry
        None,        // x-axis-rotation
        None,        // large-arc-flag
        None,        // sweep-flag
        Some(true),  // x
        Some(false), // y
    ];
    let tokens = d.split([' ', ',']).filter(|s| !s.is_empty());
    let mut current_cmd: char = ' ';
    let mut pattern: &[Option<bool>] = &[];
    let mut arg_idx: usize = 0;
    for tok in tokens {
        let first = tok.chars().next().unwrap();
        if first.is_ascii_alphabetic() {
            current_cmd = first;
            arg_idx = 0;
            pattern = match first {
                'M' | 'm' | 'L' | 'l' | 'T' | 't' => &[Some(true), Some(false)],
                'H' | 'h' => &[Some(true)],
                'V' | 'v' => &[Some(false)],
                'C' | 'c' => &[
                    Some(true),
                    Some(false),
                    Some(true),
                    Some(false),
                    Some(true),
                    Some(false),
                ],
                'S' | 's' | 'Q' | 'q' => &[Some(true), Some(false), Some(true), Some(false)],
                'A' | 'a' => arc_pattern,
                _ => &[],
            };
            // Bare command token (no embedded number).
            if tok.len() == 1 {
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push(first);
                continue;
            }
            // Compact form: command letter immediately followed by digits
            // (e.g. "M0"). Emit the letter, then fall through to handle
            // the trailing number as the first argument.
            if !out.is_empty() {
                out.push(' ');
            }
            out.push(first);
            let rest = &tok[1..];
            push_scaled_arg(&mut out, rest, current_cmd, pattern, &mut arg_idx, sx, sy);
            continue;
        }
        if pattern.is_empty() {
            // Stray number with no command context — pass through.
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(tok);
            continue;
        }
        out.push(' ');
        push_scaled_arg(&mut out, tok, current_cmd, pattern, &mut arg_idx, sx, sy);
    }
    out
}

fn push_scaled_arg(
    out: &mut String,
    tok: &str,
    cmd: char,
    pattern: &[Option<bool>],
    arg_idx: &mut usize,
    sx: f64,
    sy: f64,
) {
    let role = pattern[*arg_idx % pattern.len()];
    *arg_idx += 1;
    match role {
        Some(true) => match tok.parse::<f64>() {
            Ok(v) => out.push_str(&fmt::n(v * sx)),
            Err(_) => out.push_str(tok),
        },
        Some(false) => match tok.parse::<f64>() {
            Ok(v) => out.push_str(&fmt::n(v * sy)),
            Err(_) => out.push_str(tok),
        },
        None => out.push_str(tok),
    }
    let _ = cmd; // reserved for future per-command corrections
}

#[inline]
pub(crate) fn fallback_rect(w: f64, h: f64) -> String {
    format!("<rect width=\"{}\" height=\"{}\"/>", fmt::n(w), fmt::n(h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use slideglance_model::{CustomGeometry, PresetGeometry};
    use std::collections::BTreeMap;

    fn preset(name: &str) -> Geometry {
        Geometry::Preset(PresetGeometry {
            preset: name.to_string(),
            adjust_values: BTreeMap::new(),
        })
    }

    #[test]
    fn preset_rect_dispatch() {
        let svg = render_geometry(&preset("rect"), 100.0, 50.0);
        assert_eq!(svg, "<rect width=\"100\" height=\"50\"/>");
    }

    #[test]
    fn unknown_preset_falls_back_to_rect() {
        let svg = render_geometry(&preset("noSuchShape"), 100.0, 50.0);
        assert_eq!(svg, "<rect width=\"100\" height=\"50\"/>");
    }

    #[test]
    fn empty_custom_falls_back_to_rect() {
        let geo = Geometry::Custom(CustomGeometry { paths: vec![] });
        assert_eq!(
            render_geometry(&geo, 80.0, 40.0),
            "<rect width=\"80\" height=\"40\"/>"
        );
    }

    #[test]
    fn single_custom_path_emits_one_element() {
        let geo = Geometry::Custom(CustomGeometry {
            paths: vec![CustomGeometryPath {
                width: 100.0,
                height: 100.0,
                commands: "M 0 0 L 100 100 Z".to_string(),
            }],
        });
        let svg = render_geometry(&geo, 200.0, 100.0);
        // Coordinates are baked at scale_x=2, scale_y=1; no transform attr.
        assert_eq!(svg, "<path d=\"M 0 0 L 200 100 Z\"/>");
    }

    #[test]
    fn custom_path_arc_scales_radii_and_endpoint() {
        // A elliptical-arc command has 7 args: rx ry rotation large-arc sweep ex ey.
        // Radii/endpoints scale; flags pass through.
        let geo = Geometry::Custom(CustomGeometry {
            paths: vec![CustomGeometryPath {
                width: 100.0,
                height: 100.0,
                commands: "M 0 0 A 50 25 0 0 1 100 50".to_string(),
            }],
        });
        let svg = render_geometry(&geo, 200.0, 50.0);
        // sx=2, sy=0.5
        assert_eq!(svg, "<path d=\"M 0 0 A 100 12.5 0 0 1 200 25\"/>");
    }

    #[test]
    fn multi_custom_paths_wrapped_in_group() {
        let geo = Geometry::Custom(CustomGeometry {
            paths: vec![
                CustomGeometryPath {
                    width: 100.0,
                    height: 100.0,
                    commands: "M 0 0".to_string(),
                },
                CustomGeometryPath {
                    width: 100.0,
                    height: 100.0,
                    commands: "M 50 50".to_string(),
                },
            ],
        });
        let svg = render_geometry(&geo, 100.0, 100.0);
        assert!(svg.starts_with("<g>"));
        assert!(svg.ends_with("</g>"));
        assert_eq!(svg.matches("<path").count(), 2);
    }

    #[test]
    fn zero_dimension_custom_uses_unit_scale() {
        let geo = Geometry::Custom(CustomGeometry {
            paths: vec![CustomGeometryPath {
                width: 0.0,
                height: 0.0,
                commands: "M 0 0 L 10 5".to_string(),
            }],
        });
        // sx=sy=1 → coordinates pass through unchanged, no transform attr.
        let svg = render_geometry(&geo, 100.0, 100.0);
        assert_eq!(svg, "<path d=\"M 0 0 L 10 5\"/>");
    }
}
