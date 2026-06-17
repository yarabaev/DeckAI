//! Resolves [`ColorRef`] values into concrete [`ResolvedColor`] values.

use crate::rgb::ResolvedColor;
use crate::scheme::{ColorMap, ColorRef, ColorScheme};
use crate::transforms::apply_color_transforms;

/// Pairs a [`ColorScheme`] with a [`ColorMap`] and resolves [`ColorRef`]
/// values against them.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ColorResolver {
    /// The 12 theme colors.
    pub scheme: ColorScheme,
    /// The mapping from document-level names to theme slots.
    pub map: ColorMap,
}

impl ColorResolver {
    /// Creates a resolver bound to a given scheme and color map.
    #[inline]
    #[must_use]
    pub const fn new(scheme: ColorScheme, map: ColorMap) -> Self {
        Self { scheme, map }
    }

    /// Resolves a [`ColorRef`] into a concrete [`ResolvedColor`].
    ///
    /// The base RGB is determined by the variant:
    /// - `Srgb` uses its embedded RGB directly.
    /// - `Scheme` looks up the name through the [`ColorMap`] then the
    ///   [`ColorScheme`] (falling back to `dk1` for unknown names).
    /// - `System` uses its `lastClr` value.
    ///
    /// All variants then have their inline transforms applied. The initial
    /// alpha is `1.0`; an explicit `<a:alpha>` transform overrides it.
    #[must_use]
    pub fn resolve(&self, color_ref: &ColorRef) -> ResolvedColor {
        let (rgb, transform) = match color_ref {
            ColorRef::Srgb { rgb, transform } => (*rgb, transform),
            ColorRef::Scheme { name, transform } => {
                let key = self.map.resolve_name(name);
                (self.scheme.get(key), transform)
            }
            ColorRef::System { last, transform } => (*last, transform),
            // Unknown preset names fall back to black, matching the
            // resolver's existing convention for unknown scheme references.
            // Callers that need TS-parity skip-on-unknown semantics (e.g.
            // the duotone parser) gate on the preset lookup before
            // constructing the ColorRef.
            ColorRef::Preset { name, transform } => (
                crate::presets::resolve_preset(name).unwrap_or(crate::rgb::Rgb::new(0, 0, 0)),
                transform,
            ),
        };
        apply_color_transforms(ResolvedColor::opaque(rgb), transform)
    }
}
