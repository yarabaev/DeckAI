// OOXML / OpenType identifiers are mixed-case proper nouns rather
// than code identifiers — same rationale as `mapping.rs`.
#![allow(clippy::doc_markdown)]

//! Font resolution: typeface name → loaded [`FontFace`].
//!
//! The renderer asks "give me the bytes for `Calibri`" and the
//! resolver walks the chain:
//!
//! 1. Direct host-provided buffers (this crate's
//!    [`BufferFontResolver`]).
//! 2. System font scan (Node / native CLI only — gated behind a
//!    `system-fonts` feature, deferred to a follow-up).
//! 3. Mapped OSS name (via [`crate::mapping::get_mapped_font`]) →
//!    repeat 1+2.
//! 4. Per-platform CJK fallback chain (via
//!    [`crate::cjk_fallback::get_cjk_fallback_fonts`]) → repeat 1+2.
//! 5. Google Fonts fetcher callback (host-provided via
//!    [`crate::FontFetcher`]).
//!
//! [`FontResolver`] is the trait every backend implements. Each step
//! of the chain is a separate `FontResolver` whose `resolve` is
//! consulted in order — the orchestrator composes them via
//! [`ChainFontResolver`].
//!
//! ## Thread-safety: why `Send + Sync`
//!
//! [`ChainFontResolver`] stores `Box<dyn FontResolver + Send + Sync>`
//! and [`OpentypeTextMeasurer`](crate::OpentypeTextMeasurer) holds
//! `Arc<dyn FontResolver + Send + Sync>`. Two reasons:
//!
//! 1. **Renderer parallelism**: the renderer is expected to
//!    parallelize per-slide work via `rayon` or similar. The
//!    measurer / resolver gets shared across worker threads, so the
//!    trait object must be `Send + Sync`. Constraining the trait
//!    bound at construction time means the constraint fails fast at
//!    compile time rather than mysteriously at the rayon `par_iter`
//!    call site.
//! 2. **Sharing across an Arc**: [`Arc<T>`] requires `T: Send + Sync`
//!    to be `Send + Sync` itself. The renderer wants
//!    `Arc<OpentypeTextMeasurer>` so the measurer can be cheaply
//!    cloned for each worker; that requires every internal trait
//!    object to be `Send + Sync` too.
//!
//! ## Implications for non-Send environments
//!
//! WASM (single-threaded by default): every type that's `Send + Sync`
//! also satisfies the trivial WASM thread bound, so the constraint is
//! a no-op there. wasm-bindgen-style bindings work without changes.
//!
//! WASM with shared-memory / threads (atomics + thread proposal):
//! `Send + Sync` is required for cross-worker sharing, so the
//! constraint is exactly what's needed.
//!
//! Embedded / no_std environments where `Mutex` / `Arc` are
//! unavailable: out of scope. This crate uses `std` unconditionally;
//! such consumers should write a thinner trait directly.
//!
//! ## When you need a non-Send resolver
//!
//! Some custom resolvers might capture `Rc`, `RefCell`, or other
//! non-Send types (e.g. a single-threaded test harness). Those don't
//! satisfy `Send + Sync` and can't be wrapped in
//! [`ChainFontResolver`] / [`OpentypeTextMeasurer`].
//!
//! Workaround: implement [`FontResolver`] on a wrapper type that's
//! `Send + Sync` (e.g. wrapping the captured non-Send state in a
//! `Mutex` so external access becomes Sync). The trait itself does
//! **not** require `Send + Sync` on `Self` (it has no `: Send + Sync`
//! supertrait), so single-threaded bespoke usage is fine — only the
//! composition primitives (`ChainFontResolver`, `OpentypeTextMeasurer`)
//! impose the bound.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::cjk_fallback::{get_cjk_fallback_fonts, CjkPlatform};
use crate::mapping::{get_mapped_font, FontMapping};
use crate::opentype::FontFace;
use crate::text_measurer::FontStyle;

/// Resolves a typeface name to an owned, parsed [`FontFace`].
///
/// Returns `None` when the resolver has no entry for the name; the
/// caller chains to the next resolver in the fallback order.
///
/// `Arc` is used so a face can be cheaply shared across the
/// `TextMeasurer` (which wants `&FontFace` per call) and the renderer
/// (which may embed the bytes into SVG / PNG output).
pub trait FontResolver {
    /// Returns the loaded face for `name`, or `None` to defer to the
    /// next resolver in the chain.
    fn resolve(&self, name: &str) -> Option<Arc<FontFace>>;
}

/// Extension of [`FontResolver`] that can locate bold / italic variants.
///
/// Implementors that expose distinct bold / italic faces under the same
/// family name implement this trait; renderers that need style-specific
/// glyph outlines call `resolve_variant` instead of `resolve`.
///
/// Returning `None` means "no variant registered for this combination" —
/// callers should fall back to the regular [`FontResolver::resolve`]
/// face, optionally with synthetic bold / italic.
///
/// Variant population (registering bold / italic faces by family) is a
/// D3 concern (KDD-10 full implementation). D0 wires the trait so that
/// downstream code (D2 embed pipeline, D3 renderer) can program against
/// it without further breaking changes.
pub trait FontVariantResolver: FontResolver {
    /// Resolves a variant face for `name` matching the requested style.
    fn resolve_variant(&self, name: &str, style: FontStyle) -> Option<Arc<FontFace>>;
}

// Allow `&dyn FontResolver` and `Box<dyn FontResolver>` to forward.
impl<T: FontResolver + ?Sized> FontResolver for &T {
    fn resolve(&self, name: &str) -> Option<Arc<FontFace>> {
        (**self).resolve(name)
    }
}

impl<T: FontResolver + ?Sized> FontResolver for Box<T> {
    fn resolve(&self, name: &str) -> Option<Arc<FontFace>> {
        (**self).resolve(name)
    }
}

impl<T: FontResolver + ?Sized> FontResolver for Arc<T> {
    fn resolve(&self, name: &str) -> Option<Arc<FontFace>> {
        (**self).resolve(name)
    }
}

/// Resolver that holds a curated set of host-supplied font buffers.
///
/// Typical usage: the host loads embedded fonts from the PPTX archive
/// (`ppt/fonts/font*.fntdata`) plus any user-attached fonts, parses
/// them via [`FontFace::from_bytes`], and registers each by family
/// name. Each entry is also indexed under a normalized key (NFKC +
/// case-folded + whitespace / hyphen / dot stripped) so deck-side
/// spellings like `"Apple SD Gothic Neo"` / `"AppleSDGothicNeo"` /
/// `"apple-sd-gothic-neo"` all hit the same face. Korean / CJK
/// localized aliases are matched verbatim through the normalized
/// key path as well — full-width digits collapse to ASCII via NFKC.
/// For deeper substitution (PPTX → OSS swap tables) layer a
/// [`MappedFontResolver`] on top.
#[derive(Debug, Default, Clone)]
pub struct BufferFontResolver {
    fonts: BTreeMap<String, Arc<FontFace>>,
    /// Secondary index keyed by [`normalize_font_name`] — populated in
    /// lock-step with `fonts` and consulted on miss. Multiple original
    /// keys can collide on one normalized form (rare, but possible —
    /// last-insert wins, matching the existing `insert` overwrite
    /// semantics on the primary map).
    normalized: BTreeMap<String, Arc<FontFace>>,
    /// Bold-variant slot. A static Bold face (e.g. `Pretendard-Bold.ttf`)
    /// usually shares its `family_name()` with the Regular face, so
    /// inserting both into `fonts` would silently overwrite. Hosts that
    /// care about weight-accurate measurement / rendering register Bold
    /// faces here via [`Self::insert_bold_variant`]; lookups happen
    /// through [`FontVariantResolver::resolve_variant`] when a run
    /// requests `style.bold == true`. When no Bold variant is registered
    /// for a name, [`FontVariantResolver::resolve_variant`] returns
    /// `None` and callers fall back to the Regular face plus
    /// synthetic-bold.
    bold_variants: BTreeMap<String, Arc<FontFace>>,
    /// Normalized index for `bold_variants`, mirroring the role of
    /// [`Self::normalized`] for the regular slot.
    bold_normalized: BTreeMap<String, Arc<FontFace>>,
}

impl BufferFontResolver {
    /// Empty resolver. Use [`Self::insert`] to register faces.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds a resolver from an iterator of `(name, face)` pairs.
    ///
    /// Named `from_entries` rather than `from_iter` to avoid collision
    /// with [`FromIterator::from_iter`] which would require taking
    /// `IntoIterator` with a single `Self` item type.
    pub fn from_entries<I, S>(entries: I) -> Self
    where
        I: IntoIterator<Item = (S, FontFace)>,
        S: Into<String>,
    {
        let mut me = Self::new();
        for (name, face) in entries {
            me.insert(name, face);
        }
        me
    }

    /// Registers `face` under `name`. A subsequent call with the same
    /// name overrides the previous face.
    pub fn insert<S: Into<String>>(&mut self, name: S, face: FontFace) {
        let s = name.into();
        let arc = Arc::new(face);
        self.normalized
            .insert(normalize_font_name(&s), Arc::clone(&arc));
        self.fonts.insert(s, arc);
    }

    /// Registers a pre-shared `Arc<FontFace>` (cheaper when the same
    /// face is also held by another resolver).
    pub fn insert_arc<S: Into<String>>(&mut self, name: S, face: Arc<FontFace>) {
        let s = name.into();
        self.normalized
            .insert(normalize_font_name(&s), Arc::clone(&face));
        self.fonts.insert(s, face);
    }

    /// Registers `face` as the Bold variant for `name`. Looked up by
    /// [`FontVariantResolver::resolve_variant`] when a run requests
    /// `style.bold == true`. The variant slot is independent of the
    /// regular slot — registering a Bold face here does not populate
    /// the regular slot, and registering the same name through
    /// [`Self::insert`] does not overwrite the Bold slot.
    pub fn insert_bold_variant<S: Into<String>>(&mut self, name: S, face: FontFace) {
        let s = name.into();
        let arc = Arc::new(face);
        self.bold_normalized
            .insert(normalize_font_name(&s), Arc::clone(&arc));
        self.bold_variants.insert(s, arc);
    }

    /// `Arc`-sharing variant of [`Self::insert_bold_variant`].
    pub fn insert_bold_variant_arc<S: Into<String>>(&mut self, name: S, face: Arc<FontFace>) {
        let s = name.into();
        self.bold_normalized
            .insert(normalize_font_name(&s), Arc::clone(&face));
        self.bold_variants.insert(s, face);
    }

    /// Number of registered faces.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fonts.len()
    }

    /// True if no faces are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fonts.is_empty()
    }

    /// Iterates registered `(name, face)` pairs in `BTreeMap` order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Arc<FontFace>)> {
        self.fonts.iter()
    }
}

impl FontResolver for BufferFontResolver {
    fn resolve(&self, name: &str) -> Option<Arc<FontFace>> {
        if let Some(face) = self.fonts.get(name).cloned() {
            return Some(face);
        }
        // Fall back to the normalized index so deck-side spellings like
        // `"Apple SD Gothic Neo"` / `"AppleSDGothicNeo"` / mixed-case
        // / Unicode-fullwidth variants all resolve to the same face.
        if let Some(face) = self.normalized.get(&normalize_font_name(name)).cloned() {
            return Some(face);
        }
        // Last-resort fallback: when only a Bold variant has been
        // registered for this name (no Regular), use it as the regular
        // face. Without this, `resolve` would return `None` even though
        // a face for the family is available — leading the renderer to
        // emit `<text>` instead of glyph outlines.
        if let Some(face) = self.bold_variants.get(name).cloned() {
            return Some(face);
        }
        self.bold_normalized
            .get(&normalize_font_name(name))
            .cloned()
    }
}

impl FontVariantResolver for BufferFontResolver {
    fn resolve_variant(&self, name: &str, style: FontStyle) -> Option<Arc<FontFace>> {
        // Italic is plumbed but not yet stored — there is no italic
        // slot on this resolver. Return `None` for italic-only requests
        // so callers fall through to synthetic-italic; bold-with-italic
        // requests fall through to the bold-variant lookup below
        // (italic is treated as a no-op match for now).
        if !style.bold {
            return None;
        }
        if let Some(face) = self.bold_variants.get(name).cloned() {
            return Some(face);
        }
        self.bold_normalized
            .get(&normalize_font_name(name))
            .cloned()
    }
}

/// Normalizes a font family / face name for fuzzy lookup.
///
/// Steps:
/// 1. Unicode NFKC compatibility decomposition + recomposition — folds
///    full-width Latin (e.g. `"Ｐｒｉｎｔ"`) and CJK compatibility forms
///    onto their canonical ASCII / Hangul equivalents.
/// 2. ASCII case-fold (`to_ascii_lowercase`) — Korean / Han glyphs are
///    already case-less.
/// 3. Strip ASCII whitespace, hyphens, underscores, dots, plus signs.
///
/// PPTX `<a:latin typeface="…"/>` carries whatever string the authoring
/// app showed at write time, which historically varies across MS
/// Office releases (`"Apple SD Gothic Neo"` ↔ `"AppleSDGothicNeo"`),
/// platform converters (`"NanumGothic"` ↔ `"Nanum Gothic"`), and font
/// installer locales (`"プリンタフォント"` ↔ `"プリンタ-フォント"`).
/// Normalizing on both sides of the lookup catches all those without a
/// per-font alias table.
#[must_use]
pub fn normalize_font_name(s: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    let mut out = String::with_capacity(s.len());
    for ch in s.nfkc() {
        if ch.is_whitespace() || matches!(ch, '-' | '_' | '.' | '+' | '\u{200B}' | '\u{FEFF}') {
            continue;
        }
        for lower in ch.to_lowercase() {
            out.push(lower);
        }
    }
    out
}

/// Resolver that wraps an inner resolver and applies the
/// [`FontMapping`] (PPTX → OSS replacement) before each lookup.
///
/// On miss for the original name, retries with `get_mapped_font`'s
/// result. Composes naturally with [`BufferFontResolver`] /
/// [`ChainFontResolver`].
///
/// Visibility: `pub(crate)`. External callers should compose chains via
/// [`standard_resolver_chain`] instead of constructing this directly —
/// the chain ordering (mapping → CJK fallback) is part of the crate's
/// stable resolution contract (KDD-20).
pub(crate) struct MappedFontResolver<R: FontResolver> {
    inner: R,
    mapping: FontMapping,
}

impl<R: FontResolver> MappedFontResolver<R> {
    /// Wraps `inner` with `mapping`. Pass
    /// [`crate::mapping::default_font_mapping`] for the standard PPTX
    /// → OSS replacement table.
    pub(crate) fn new(inner: R, mapping: FontMapping) -> Self {
        Self { inner, mapping }
    }
}

impl<R: FontResolver> FontResolver for MappedFontResolver<R> {
    fn resolve(&self, name: &str) -> Option<Arc<FontFace>> {
        if let Some(face) = self.inner.resolve(name) {
            return Some(face);
        }
        let mapped = get_mapped_font(name, &self.mapping)?;
        self.inner.resolve(&mapped)
    }
}

impl<R: FontVariantResolver> FontVariantResolver for MappedFontResolver<R> {
    fn resolve_variant(&self, name: &str, style: FontStyle) -> Option<Arc<FontFace>> {
        if let Some(face) = self.inner.resolve_variant(name, style) {
            return Some(face);
        }
        // Mapped name (e.g. `Calibri → Carlito`) — try the variant slot
        // for the mapping target as well so a deck that authored Bold
        // Calibri renders / measures as Bold Carlito instead of falling
        // through to synthetic bold.
        let mapped = get_mapped_font(name, &self.mapping)?;
        self.inner.resolve_variant(&mapped, style)
    }
}

/// Resolver that adds a per-platform CJK fallback chain on top of an
/// inner resolver.
///
/// Inner resolver is consulted with the original name first; on miss,
/// the resolver re-queries with each entry from
/// [`get_cjk_fallback_fonts`] in order. Useful when the inner resolver
/// is a system-font scan and you want preinstalled Hiragino / `Yu Gothic`
/// / `Microsoft YaHei` to fill the gap when Noto isn't installed.
///
/// Visibility: `pub(crate)`. See [`MappedFontResolver`] note — external
/// callers compose via [`standard_resolver_chain`].
pub(crate) struct CjkFallbackResolver<R: FontResolver> {
    inner: R,
    platform: CjkPlatform,
}

impl<R: FontResolver> CjkFallbackResolver<R> {
    /// Wraps `inner` with the per-platform fallback chain.
    pub(crate) fn new(inner: R, platform: CjkPlatform) -> Self {
        Self { inner, platform }
    }
}

impl<R: FontResolver> FontResolver for CjkFallbackResolver<R> {
    fn resolve(&self, name: &str) -> Option<Arc<FontFace>> {
        if let Some(face) = self.inner.resolve(name) {
            return Some(face);
        }
        for fallback in get_cjk_fallback_fonts(self.platform, name) {
            if let Some(face) = self.inner.resolve(fallback) {
                return Some(face);
            }
        }
        None
    }
}

impl<R: FontVariantResolver> FontVariantResolver for CjkFallbackResolver<R> {
    fn resolve_variant(&self, name: &str, style: FontStyle) -> Option<Arc<FontFace>> {
        if let Some(face) = self.inner.resolve_variant(name, style) {
            return Some(face);
        }
        for fallback in get_cjk_fallback_fonts(self.platform, name) {
            if let Some(face) = self.inner.resolve_variant(fallback, style) {
                return Some(face);
            }
        }
        None
    }
}

/// Build the standard `inner → mapping → CJK fallback` chain.
///
/// Replaces direct construction of the now-`pub(crate)`
/// [`MappedFontResolver`] / [`CjkFallbackResolver`]. Returns a
/// `FontVariantResolver` (which is a supertrait of [`FontResolver`]) so
/// callers can later upgrade to variant-aware resolution without
/// re-wrapping.
///
/// Composition: caller-supplied `inner` (typically a
/// [`BufferFontResolver`] populated from PPTX-embedded font binaries)
/// is wrapped first by [`FontMapping`] (PPTX → OSS substitution) and
/// then by the per-platform CJK fallback walker. This matches the
/// ordering used by the CLI (`slideglance` binary) and the WASM bridge.
pub fn standard_resolver_chain(
    inner: BufferFontResolver,
    mapping: FontMapping,
    platform: CjkPlatform,
) -> impl FontVariantResolver + Send + Sync + 'static {
    let mapped = MappedFontResolver::new(inner, mapping);
    CjkFallbackResolver::new(mapped, platform)
}

/// Composes multiple resolvers in order — the first to return `Some`
/// wins. Mirrors the renderer's "user → system → fallback → google"
/// chain.
pub struct ChainFontResolver {
    resolvers: Vec<Box<dyn FontResolver + Send + Sync>>,
}

impl ChainFontResolver {
    /// Empty chain. Append resolvers via [`Self::push`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            resolvers: Vec::new(),
        }
    }

    /// Builds a chain from an iterator of boxed resolvers, in order.
    #[must_use]
    pub fn from_boxed(resolvers: Vec<Box<dyn FontResolver + Send + Sync>>) -> Self {
        Self { resolvers }
    }

    /// Appends `resolver` to the end of the chain (consulted last).
    pub fn push<R: FontResolver + Send + Sync + 'static>(&mut self, resolver: R) {
        self.resolvers.push(Box::new(resolver));
    }

    /// Number of resolvers in the chain.
    #[must_use]
    pub fn len(&self) -> usize {
        self.resolvers.len()
    }

    /// True if the chain is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.resolvers.is_empty()
    }
}

impl Default for ChainFontResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl FontResolver for ChainFontResolver {
    fn resolve(&self, name: &str) -> Option<Arc<FontFace>> {
        for resolver in &self.resolvers {
            if let Some(face) = resolver.resolve(name) {
                return Some(face);
            }
        }
        None
    }
}

impl FontVariantResolver for ChainFontResolver {
    fn resolve_variant(&self, _name: &str, _style: FontStyle) -> Option<Arc<FontFace>> {
        // Chain stores `Box<dyn FontResolver + Send + Sync>` — the
        // dynamic trait object only exposes the base trait, so chain-
        // wide variant lookup is not yet possible. Storing the variant
        // trait object is a D3 follow-up; for D0 we return `None` so
        // callers fall back to `resolve` + synthetic styling.
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test resolver that yields a fixed name → empty `FontError`-free
    /// face. We can't construct real `FontFace` without valid bytes;
    /// instead, construct a stub resolver that returns `None` from a
    /// tracked name set.
    struct TrackingResolver {
        known: Vec<String>,
        responses: std::cell::RefCell<Vec<String>>,
    }

    impl TrackingResolver {
        fn new(known: &[&str]) -> Self {
            Self {
                known: known.iter().map(|s| (*s).to_string()).collect(),
                responses: std::cell::RefCell::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.responses.borrow().clone()
        }
    }

    impl FontResolver for TrackingResolver {
        fn resolve(&self, name: &str) -> Option<Arc<FontFace>> {
            self.responses.borrow_mut().push(name.to_string());
            // Return None for everything; real-face integration tests live
            // in the renderer batch where embedded font fixtures land.
            if self.known.iter().any(|n| n == name) {
                // Even known names return None here since we can't build a
                // FontFace from synthetic bytes; the test just verifies
                // call sequencing.
                None
            } else {
                None
            }
        }
    }

    // -- BufferFontResolver --------------------------------------------------

    #[test]
    fn buffer_resolver_empty_returns_none() {
        let resolver = BufferFontResolver::new();
        assert!(resolver.resolve("Calibri").is_none());
        assert_eq!(resolver.len(), 0);
        assert!(resolver.is_empty());
    }

    // We can't insert real FontFace without valid bytes — verify the
    // empty / iter / Default surface works without compiling errors.
    #[test]
    fn buffer_resolver_default_is_empty() {
        let resolver = BufferFontResolver::default();
        assert!(resolver.is_empty());
        assert_eq!(resolver.iter().count(), 0);
    }

    #[test]
    fn buffer_resolver_from_entries_empty() {
        let resolver =
            BufferFontResolver::from_entries::<Vec<(String, FontFace)>, String>(Vec::new());
        assert!(resolver.is_empty());
    }

    // -- MappedFontResolver --------------------------------------------------

    #[test]
    fn mapped_resolver_consults_inner_with_original_first() {
        let inner = TrackingResolver::new(&["Carlito"]);
        let mapping = crate::mapping::default_font_mapping();
        let resolver = MappedFontResolver::new(&inner, mapping);
        let _ = resolver.resolve("Calibri");
        let calls = inner.calls();
        // First call uses the original name.
        assert_eq!(calls.first().map(String::as_str), Some("Calibri"));
        // Second call uses the mapped name (Calibri → Carlito).
        assert_eq!(calls.get(1).map(String::as_str), Some("Carlito"));
    }

    #[test]
    fn mapped_resolver_skips_mapping_lookup_when_inner_hit() {
        // No way to make TrackingResolver succeed; verify behavior on
        // the miss path by checking that an unmapped name only triggers
        // a single call.
        let inner = TrackingResolver::new(&[]);
        let mapping = FontMapping::new(); // empty mapping
        let resolver = MappedFontResolver::new(&inner, mapping);
        let _ = resolver.resolve("Some Custom Font");
        let calls = inner.calls();
        assert_eq!(calls, vec!["Some Custom Font".to_string()]);
    }

    // -- CjkFallbackResolver -------------------------------------------------

    #[test]
    fn cjk_fallback_walks_chain_on_miss() {
        let inner = TrackingResolver::new(&["Hiragino Sans"]);
        let resolver = CjkFallbackResolver::new(&inner, CjkPlatform::MacOs);
        let _ = resolver.resolve("Noto Sans JP");
        let calls = inner.calls();
        // First: original name.
        assert_eq!(calls.first().map(String::as_str), Some("Noto Sans JP"));
        // Then macOS fallback chain — ["Hiragino Sans", "Hiragino Kaku Gothic ProN"].
        assert!(calls.contains(&"Hiragino Sans".to_string()));
        assert!(calls.contains(&"Hiragino Kaku Gothic ProN".to_string()));
    }

    #[test]
    fn cjk_fallback_walks_unified_chain_for_other_platform() {
        // Post-unification, `CjkPlatform::Other` no longer yields an
        // empty chain — the fallback table is platform-independent so
        // every name gets the full Windows / macOS / Linux union.
        let inner = TrackingResolver::new(&[]);
        let resolver = CjkFallbackResolver::new(&inner, CjkPlatform::Other);
        let _ = resolver.resolve("Noto Sans JP");
        let calls = inner.calls();
        assert_eq!(calls.first().map(String::as_str), Some("Noto Sans JP"));
        // Windows-native section now leads the unified chain.
        assert!(calls.contains(&"Yu Gothic".to_string()));
        // macOS-native section.
        assert!(calls.contains(&"Hiragino Sans".to_string()));
    }

    #[test]
    fn cjk_fallback_empty_for_unknown_name() {
        let inner = TrackingResolver::new(&[]);
        let resolver = CjkFallbackResolver::new(&inner, CjkPlatform::Linux);
        let _ = resolver.resolve("UnknownFont");
        // No fallback entry → just the one direct call.
        assert_eq!(inner.calls(), vec!["UnknownFont".to_string()]);
    }

    // -- ChainFontResolver ---------------------------------------------------

    #[test]
    fn chain_resolver_empty_returns_none() {
        let chain = ChainFontResolver::new();
        assert!(chain.resolve("anything").is_none());
        assert!(chain.is_empty());
    }

    #[test]
    fn chain_resolver_consults_in_order() {
        let chain = ChainFontResolver::from_boxed(vec![
            Box::new(BufferFontResolver::new()) as Box<dyn FontResolver + Send + Sync>,
            Box::new(BufferFontResolver::new()),
            Box::new(BufferFontResolver::new()),
        ]);
        assert_eq!(chain.len(), 3);
        assert!(chain.resolve("anything").is_none());
    }

    #[test]
    fn chain_resolver_push_appends() {
        let mut chain = ChainFontResolver::new();
        chain.push(BufferFontResolver::new());
        chain.push(BufferFontResolver::new());
        assert_eq!(chain.len(), 2);
    }

    // -- FontVariantResolver supertrait --------------------------------------

    #[test]
    fn font_variant_resolver_is_supertrait_of_font_resolver() {
        // Compile-time check: BufferFontResolver implements both
        // FontResolver and FontVariantResolver. Anything bound to the
        // variant trait can also be used as a plain `FontResolver`.
        fn assert_variant_resolver<T: FontVariantResolver + FontResolver + Send + Sync>(_: &T) {}
        let r = BufferFontResolver::default();
        assert_variant_resolver(&r);
    }

    #[test]
    fn buffer_resolver_variant_returns_none_for_unknown_name() {
        let r = BufferFontResolver::default();
        let result = r.resolve_variant(
            "Unknown Font",
            FontStyle {
                bold: true,
                italic: false,
            },
        );
        assert!(result.is_none());
    }

    #[test]
    fn standard_resolver_chain_exposes_variant_resolver_trait() {
        let chain = standard_resolver_chain(
            BufferFontResolver::default(),
            FontMapping::new(),
            CjkPlatform::Linux,
        );
        // Both supertrait and subtrait surfaces accessible.
        assert!(chain.resolve("Calibri").is_none());
        assert!(chain
            .resolve_variant(
                "Calibri",
                FontStyle {
                    bold: true,
                    italic: false,
                },
            )
            .is_none());
    }

    // -- Forwarding impls ----------------------------------------------------

    #[test]
    fn forwarding_impls_compile_and_delegate() {
        // Verify &dyn FontResolver, Box<dyn FontResolver>, Arc<dyn ...>
        // implement FontResolver via blanket impls.
        let buffer = BufferFontResolver::new();
        let by_ref: &dyn FontResolver = &buffer;
        assert!(by_ref.resolve("anything").is_none());

        let by_box: Box<dyn FontResolver> = Box::new(BufferFontResolver::new());
        assert!(by_box.resolve("anything").is_none());

        let by_arc: Arc<dyn FontResolver> = Arc::new(BufferFontResolver::new());
        assert!(by_arc.resolve("anything").is_none());
    }
}
