//! Renderer error types.
//!
//! The slide renderer (and the per-element renderers reachable from it)
//! returns [`Result`] so the caller never receives a silently divergent
//! SVG. The spec at
//! never returns errors — it best-effort renders. Rust diverges
//! deliberately: every code path that hits an unsupported feature
//! surfaces [`RendererError::NotImplemented`] so future Phase-A / Phase-B
//! work can land without changing the public signature again.

use std::fmt;

/// Errors returned by the slide-level renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RendererError {
    /// A code path is reached for a feature whose visual contribution is not
    /// yet implemented in this build. The static `&str` identifies the
    /// feature so the caller (or test) can branch on it.
    ///
    /// Currently produced for:
    /// - `"effects"` — any non-empty `<a:effectLst>` (shadow / glow / softEdge)
    /// - `"spAutofit"` — `<a:bodyPr autoFit="spAutoFit"/>` body autofit mode
    NotImplemented(&'static str),
}

impl fmt::Display for RendererError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotImplemented(feature) => {
                write!(f, "renderer feature not implemented: {feature}")
            }
        }
    }
}

impl std::error::Error for RendererError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_includes_feature_name() {
        let err = RendererError::NotImplemented("effects");
        assert_eq!(err.to_string(), "renderer feature not implemented: effects");
    }

    #[test]
    fn equality_by_feature_tag() {
        assert_eq!(
            RendererError::NotImplemented("effects"),
            RendererError::NotImplemented("effects")
        );
        assert_ne!(
            RendererError::NotImplemented("effects"),
            RendererError::NotImplemented("spAutofit")
        );
    }
}
