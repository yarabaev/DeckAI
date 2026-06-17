//! Single-entry-point text pipeline (KDD-3).
//!
//! [`TextEngine`] bundles a [`TextMeasurer`], an optional
//! [`FontResolver`], and the resulting [`RenderMode`] dispatch decision
//! into one shareable object. Construct via [`TextEngineBuilder`] —
//! the renderer (D3) and the wasm bridge (D4) consume the assembled
//! engine instead of threading three separate fields through every
//! call.
//!
//! D0 ships the skeleton: builder + render-mode classification + Arc
//! conversion. The full integration into the renderer / wasm pipelines
//! lands in D2/D3/D4.

use std::sync::Arc;

use crate::font_resolver::{FontResolver, FontVariantResolver};
use crate::text_measurer::{HeuristicTextMeasurer, TextMeasurer};

/// Rendering mode chosen for a conversion run.
///
/// `PathMode` requires a [`FontResolver`] (variant-aware or not) and
/// emits glyph outlines as SVG `<path>` elements — the only mode resvg
/// can rasterize without a system fontdb. `TextMode` emits `<text>`
/// elements; no resolver is needed but text may render with substitute
/// fonts (or vanish entirely in viewers without the declared faces).
///
/// This enum lives in `slideglance-font` per Plan Review Resolution **R-C3**:
/// `slideglance::convert::RenderMode` re-exports this type so there is one
/// authoritative definition shared by every layer.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum RenderMode {
    /// Glyph outlines (`<path>`). Requires a resolver.
    PathMode,
    /// Text elements (`<text>`). No resolver required.
    #[default]
    TextMode,
}

/// Assembled text pipeline: measurer + optional resolver + render mode.
///
/// Constructed via [`TextEngineBuilder`]. The renderer and wasm
/// bindings consume the engine through the public accessors:
/// [`TextEngine::measurer`], [`TextEngine::resolver`],
/// [`TextEngine::variant_resolver`], and [`TextEngine::render_mode`].
pub struct TextEngine {
    measurer: Arc<dyn TextMeasurer + Send + Sync>,
    resolver: Option<Arc<dyn FontResolver + Send + Sync>>,
    variant_resolver: Option<Arc<dyn FontVariantResolver + Send + Sync>>,
    render_mode: RenderMode,
}

impl TextEngine {
    /// Borrowed measurer suitable for the renderer's wrap pass.
    #[must_use]
    pub fn measurer(&self) -> &(dyn TextMeasurer + Send + Sync) {
        self.measurer.as_ref()
    }

    /// Optional path-mode resolver. `None` → text mode.
    #[must_use]
    pub fn resolver(&self) -> Option<&Arc<dyn FontResolver + Send + Sync>> {
        self.resolver.as_ref()
    }

    /// Optional variant-aware resolver supplied separately. When the
    /// builder receives a variant resolver, this returns `Some`; when
    /// only the base resolver was supplied, it returns `None` and
    /// callers fall back to [`Self::resolver`] + synthetic styling.
    #[must_use]
    pub fn variant_resolver(&self) -> Option<&Arc<dyn FontVariantResolver + Send + Sync>> {
        self.variant_resolver.as_ref()
    }

    /// The render mode determined at build time.
    #[must_use]
    pub fn render_mode(&self) -> RenderMode {
        self.render_mode
    }
}

/// Builder for [`TextEngine`].
///
/// Decomposed builder pattern (per the layer rule that
/// `slideglance-font ← slideglance`): the builder accepts the inputs it can express
/// without depending on `slideglance` types. The owner crate (`slideglance`)
/// projects its `FontConfig` into these fields when assembling the
/// engine for a conversion call.
pub struct TextEngineBuilder {
    measurer: Option<Arc<dyn TextMeasurer + Send + Sync>>,
    resolver: Option<Box<dyn FontResolver + Send + Sync>>,
    variant_resolver: Option<Box<dyn FontVariantResolver + Send + Sync>>,
}

impl TextEngineBuilder {
    /// Empty builder — defaults to [`HeuristicTextMeasurer`] and no
    /// resolver (i.e. text-mode).
    #[must_use]
    pub fn new() -> Self {
        Self {
            measurer: None,
            resolver: None,
            variant_resolver: None,
        }
    }

    /// Override the default heuristic measurer.
    #[must_use]
    pub fn with_measurer(mut self, m: Arc<dyn TextMeasurer + Send + Sync>) -> Self {
        self.measurer = Some(m);
        self
    }

    /// Provide a path-mode font resolver. Setting this flips the built
    /// engine into [`RenderMode::PathMode`].
    #[must_use]
    pub fn with_resolver(mut self, r: Box<dyn FontResolver + Send + Sync>) -> Self {
        self.resolver = Some(r);
        self
    }

    /// Provide a variant-aware resolver in addition to (or instead of)
    /// the base resolver. Variant resolvers also count as a regular
    /// `FontResolver` thanks to the supertrait, so supplying one is
    /// enough to enable path mode.
    #[must_use]
    pub fn with_variant_resolver(mut self, r: Box<dyn FontVariantResolver + Send + Sync>) -> Self {
        self.variant_resolver = Some(r);
        self
    }

    /// Finalize the engine. Default measurer is
    /// [`HeuristicTextMeasurer`]. Render mode is [`RenderMode::PathMode`]
    /// when *any* resolver was supplied, else [`RenderMode::TextMode`].
    #[must_use]
    pub fn build(self) -> TextEngine {
        let render_mode = if self.resolver.is_some() || self.variant_resolver.is_some() {
            RenderMode::PathMode
        } else {
            RenderMode::TextMode
        };
        let measurer: Arc<dyn TextMeasurer + Send + Sync> = self
            .measurer
            .unwrap_or_else(|| Arc::new(HeuristicTextMeasurer));
        let resolver: Option<Arc<dyn FontResolver + Send + Sync>> = self
            .resolver
            .map(|b| -> Arc<dyn FontResolver + Send + Sync> { Arc::from(b) });
        let variant_resolver: Option<Arc<dyn FontVariantResolver + Send + Sync>> = self
            .variant_resolver
            .map(|b| -> Arc<dyn FontVariantResolver + Send + Sync> { Arc::from(b) });
        TextEngine {
            measurer,
            resolver,
            variant_resolver,
            render_mode,
        }
    }
}

impl Default for TextEngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font_resolver::BufferFontResolver;

    #[test]
    fn render_mode_default_is_text_mode() {
        assert_eq!(RenderMode::default(), RenderMode::TextMode);
    }

    #[test]
    fn text_engine_builder_default_build_succeeds() {
        let engine = TextEngineBuilder::new().build();
        assert_eq!(engine.render_mode(), RenderMode::TextMode);
        assert!(engine.resolver().is_none());
        assert!(engine.variant_resolver().is_none());
    }

    #[test]
    fn text_engine_builder_with_measurer_keeps_text_mode() {
        let measurer: Arc<dyn TextMeasurer + Send + Sync> = Arc::new(HeuristicTextMeasurer);
        let engine = TextEngineBuilder::new().with_measurer(measurer).build();
        assert_eq!(engine.render_mode(), RenderMode::TextMode);
    }

    #[test]
    fn text_engine_builder_with_resolver_flips_to_path_mode() {
        let resolver: Box<dyn FontResolver + Send + Sync> = Box::new(BufferFontResolver::default());
        let engine = TextEngineBuilder::new().with_resolver(resolver).build();
        assert_eq!(engine.render_mode(), RenderMode::PathMode);
        assert!(engine.resolver().is_some());
    }

    #[test]
    fn text_engine_builder_with_variant_resolver_also_flips_to_path_mode() {
        let resolver: Box<dyn FontVariantResolver + Send + Sync> =
            Box::new(BufferFontResolver::default());
        let engine = TextEngineBuilder::new()
            .with_variant_resolver(resolver)
            .build();
        assert_eq!(engine.render_mode(), RenderMode::PathMode);
        assert!(engine.variant_resolver().is_some());
    }
}
