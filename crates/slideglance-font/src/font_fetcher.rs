// OOXML / OpenType identifiers are mixed-case proper nouns rather
// than code identifiers — same rationale as `mapping.rs`.
#![allow(clippy::doc_markdown)]

//! Host-supplied font fetcher (Google Fonts integration point).
//!
//! The host decides:
//!
//! - **Where to fetch from** — Google Fonts CSS API, an internal CDN,
//!   a precomputed bundle, etc.
//! - **CORS / User-Agent / authentication** — these are inherently
//!   host concerns; the conversion library cannot make policy choices
//!   on behalf of every consumer.
//!
//! [`FontFetcher`] is sync-only. The renderer's wrap / measurement
//! passes do not need to suspend awaiting font bytes, and adding
//! `async` here forces every caller into an async context. Pre-fetch
//! via the sync API before invoking conversion.
//!
//! [`FetcherFontResolver`] adapts the trait to the [`FontResolver`]
//! interface and caches successful loads so a presentation that uses
//! the same family in many slides only fetches once.
//!
//! ## Cache policy
//!
//! The cache uses `Mutex<BTreeMap<String, CacheEntry>>` where each
//! entry records *why* a slot was filled:
//!
//! - [`CacheEntry::Hit(face)`] — fetcher returned bytes that parsed
//!   into a usable [`FontFace`]. Permanent cache hit.
//! - [`CacheEntry::Missing`] — fetcher returned `None` (host has no
//!   bytes for this family). Soft negative cache — host code that
//!   adds new font sources can call [`FetcherFontResolver::forget`]
//!   or [`FetcherFontResolver::clear_negative_cache`] to retry.
//! - [`CacheEntry::ParseFail`] — fetcher returned bytes but
//!   `parse_font_data` rejected them (corrupt / unsupported tables).
//!   Hard negative cache — bytes from this name will not improve on
//!   retry within the session, so we don't re-attempt without
//!   explicit `forget(name)`.
//!
//! [`FetcherFontResolver::resolve`] returns `None` for both
//! `Missing` and `ParseFail`. The distinction matters only for cache
//! invalidation:
//!
//! - [`FetcherFontResolver::clear_cache`] — drops everything.
//! - [`FetcherFontResolver::clear_negative_cache`] — drops only
//!   `Missing` entries (allows retrying after the host adds new
//!   sources).
//! - [`FetcherFontResolver::forget`] — drops a single entry by name
//!   (selective retry for a specific family, including `ParseFail`
//!   entries).

use std::collections::BTreeMap;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, Mutex};

use crate::font_resolver::{FontResolver, FontVariantResolver};
use crate::opentype::FontFace;
use crate::text_measurer::FontStyle;
use crate::ttc::parse_font_data;

/// Maximum number of entries in the font cache.
/// When full, new entries are not inserted (fail-open: resolution
/// still returns the correct value, but the result is not cached).
const MAX_CACHE_ENTRIES: usize = 1024;

/// Host callback that returns ttf / otf / ttc bytes for a font family.
///
/// Returns `None` when the family is not available (e.g. the host has
/// no Google Fonts entry for it). The renderer treats this as "give
/// up on this family — defer to the next stage in the chain."
pub trait FontFetcher: Send + Sync {
    /// Fetches the bytes for `family`.
    fn fetch(&self, family: &str) -> Option<Vec<u8>>;
}

// Free closures of the right shape automatically implement the trait.
impl<F> FontFetcher for F
where
    F: Fn(&str) -> Option<Vec<u8>> + Send + Sync,
{
    fn fetch(&self, family: &str) -> Option<Vec<u8>> {
        self(family)
    }
}

/// Cached state for a single font name.
#[derive(Clone, Debug)]
enum CacheEntry {
    /// Fetcher returned bytes that parsed into a usable face.
    Hit(Arc<FontFace>),
    /// Fetcher returned `None` — host has no bytes (yet). Soft
    /// negative: clear via `clear_negative_cache` or `forget`.
    Missing,
    /// Fetcher returned bytes but parsing failed. Hard negative:
    /// retry only via explicit `forget(name)`.
    ParseFail,
}

impl CacheEntry {
    fn as_face(&self) -> Option<Arc<FontFace>> {
        match self {
            Self::Hit(face) => Some(Arc::clone(face)),
            Self::Missing | Self::ParseFail => None,
        }
    }

    fn is_missing(&self) -> bool {
        matches!(self, Self::Missing)
    }
}

/// Resolver that calls a [`FontFetcher`] on miss and caches the
/// resulting state — see module rustdoc for cache-policy details.
pub struct FetcherFontResolver<F: FontFetcher> {
    fetcher: F,
    cache: Mutex<BTreeMap<String, CacheEntry>>,
}

impl<F: FontFetcher> FetcherFontResolver<F> {
    /// Wraps `fetcher` with a per-resolver cache.
    pub fn new(fetcher: F) -> Self {
        Self {
            fetcher,
            cache: Mutex::new(BTreeMap::new()),
        }
    }

    /// Number of cached entries (Hit + Missing + ParseFail).
    pub fn cache_len(&self) -> usize {
        self.cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    }

    /// Number of `Missing` entries — fetcher returned `None`. These
    /// are the "soft" negatives that [`Self::clear_negative_cache`]
    /// targets.
    pub fn missing_count(&self) -> usize {
        let cache = self
            .cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        cache.values().filter(|e| e.is_missing()).count()
    }

    /// Number of `ParseFail` entries — fetcher returned bytes that
    /// failed to parse. Cleared only by [`Self::forget`] or
    /// [`Self::clear_cache`].
    pub fn parse_fail_count(&self) -> usize {
        let cache = self
            .cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        cache
            .values()
            .filter(|e| matches!(e, CacheEntry::ParseFail))
            .count()
    }

    /// Drops all cached entries. Subsequent `resolve` calls will hit
    /// the fetcher again for every name.
    pub fn clear_cache(&self) {
        self.cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }

    /// Drops only `Missing` entries — entries where the fetcher
    /// returned `None`. Use after the host adds new font sources
    /// (e.g. user uploaded a font) so previously-missing families can
    /// be retried.
    ///
    /// `ParseFail` entries are **not** cleared because re-fetching
    /// the same byte source rarely succeeds; use [`Self::forget`]
    /// when you specifically know the bytes have changed.
    pub fn clear_negative_cache(&self) {
        let mut cache = self
            .cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        cache.retain(|_, entry| !entry.is_missing());
    }

    /// Drops a single entry by name. Subsequent `resolve(name)` will
    /// re-invoke the fetcher.
    ///
    /// Use this when you know a specific family's bytes have changed
    /// (e.g. user replaced a corrupted font with a fresh download —
    /// the `ParseFail` cache entry needs to clear before the retry).
    /// Returns `true` if an entry existed.
    pub fn forget(&self, name: &str) -> bool {
        let mut cache = self
            .cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        cache.remove(name).is_some()
    }
}

impl<F: FontFetcher> FontResolver for FetcherFontResolver<F> {
    fn resolve(&self, name: &str) -> Option<Arc<FontFace>> {
        // Cache hit — including negative cache (Missing / ParseFail).
        {
            let cache = self
                .cache
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(entry) = cache.get(name) {
                return entry.as_face();
            }
        }

        // Wrap the fetcher call in catch_unwind so a panicking callback
        // cannot propagate out to the caller or poison the cache mutex.
        // On panic, treat the result as ParseFail and cache it so the
        // same panicking callback is not re-invoked for this name.
        let fetch_result = panic::catch_unwind(AssertUnwindSafe(|| self.fetcher.fetch(name)));
        let entry = match fetch_result {
            Err(_panic_payload) => CacheEntry::ParseFail,
            Ok(None) => CacheEntry::Missing,
            Ok(Some(data)) => match parse_font_data(data) {
                Ok(mut faces) if !faces.is_empty() => {
                    CacheEntry::Hit(Arc::new(faces.swap_remove(0)))
                }
                // Empty Ok or Err — bytes were unusable.
                Ok(_) | Err(_) => CacheEntry::ParseFail,
            },
        };

        let face = entry.as_face();
        let mut cache = self
            .cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if cache.len() < MAX_CACHE_ENTRIES {
            cache.insert(name.to_string(), entry);
        }
        // Return the resolved face regardless of whether it was cached.
        face
    }
}

impl<F: FontFetcher> FontVariantResolver for FetcherFontResolver<F> {
    fn resolve_variant(&self, _name: &str, _style: FontStyle) -> Option<Arc<FontFace>> {
        // Variant registration is a D3 concern (KDD-10). The fetcher
        // only returns bytes for a typeface name; per-style variant
        // slots are not maintained here. Return `None` so callers fall
        // back to `resolve` + synthetic styling, matching the other
        // resolvers in this crate.
        None
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    /// Test fetcher that records call count + returns bytes from a
    /// fixed map.
    struct StubFetcher {
        call_count: AtomicUsize,
        responses: BTreeMap<String, Option<Vec<u8>>>,
    }

    impl StubFetcher {
        fn new() -> Self {
            Self {
                call_count: AtomicUsize::new(0),
                responses: BTreeMap::new(),
            }
        }

        fn with(mut self, name: &str, bytes: Option<Vec<u8>>) -> Self {
            self.responses.insert(name.to_string(), bytes);
            self
        }
    }

    impl FontFetcher for StubFetcher {
        fn fetch(&self, family: &str) -> Option<Vec<u8>> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            self.responses.get(family).and_then(Clone::clone)
        }
    }

    // -- Closure impl works as a fetcher -------------------------------------

    #[test]
    fn closure_satisfies_fetcher_trait() {
        let fetcher = |family: &str| {
            if family == "Test" {
                Some(b"fake".to_vec())
            } else {
                None
            }
        };
        // Make sure the closure's return type matches via the trait.
        let bytes: Option<Vec<u8>> = fetcher.fetch("Test");
        assert_eq!(bytes.as_deref(), Some(&b"fake"[..]));
    }

    // -- Negative cache prevents repeat calls --------------------------------

    #[test]
    fn negative_cache_prevents_repeated_fetcher_calls() {
        let fetcher = StubFetcher::new();
        let resolver = FetcherFontResolver::new(fetcher);
        // First lookup: miss → fetcher called once.
        assert!(resolver.resolve("Missing").is_none());
        // Second lookup for same name: cache hit → fetcher not called.
        assert!(resolver.resolve("Missing").is_none());
        assert_eq!(resolver.cache_len(), 1);
    }

    // -- Invalid bytes cache as None -----------------------------------------

    #[test]
    fn invalid_bytes_cache_as_negative() {
        let fetcher = StubFetcher::new().with("BadFont", Some(b"not a font".to_vec()));
        let resolver = FetcherFontResolver::new(fetcher);
        // First call: fetcher returns invalid bytes; parse fails; cached
        // as None.
        assert!(resolver.resolve("BadFont").is_none());
        assert_eq!(resolver.cache_len(), 1);
        // Second call: cache hit, no re-fetch.
        assert!(resolver.resolve("BadFont").is_none());
    }

    // -- clear_cache forces re-fetch -----------------------------------------

    #[test]
    fn clear_cache_drops_entries() {
        let fetcher = StubFetcher::new();
        let resolver = FetcherFontResolver::new(fetcher);
        let _ = resolver.resolve("First");
        let _ = resolver.resolve("Second");
        assert_eq!(resolver.cache_len(), 2);
        resolver.clear_cache();
        assert_eq!(resolver.cache_len(), 0);
    }

    // -- FontFetcher delegates correctly through the trait surface -----------

    #[test]
    fn fetcher_call_count_increments_per_unique_name() {
        let fetcher = StubFetcher::new();
        let resolver = FetcherFontResolver::new(fetcher);
        let _ = resolver.resolve("Alpha");
        let _ = resolver.resolve("Beta");
        let _ = resolver.resolve("Alpha"); // cached → no call.
                                           // Need a way to grab the count after wrapping.
                                           // Since FetcherFontResolver consumes the fetcher, we can't
                                           // peek. Instead, verify cache_len reflects the unique calls.
        assert_eq!(resolver.cache_len(), 2);
    }

    #[test]
    fn returns_none_for_unknown_family() {
        let fetcher = |_: &str| None;
        let resolver = FetcherFontResolver::new(fetcher);
        assert!(resolver.resolve("Anything").is_none());
    }

    // -- Cache-policy distinctions: Missing vs ParseFail --------------------

    #[test]
    fn missing_vs_parse_fail_counted_separately() {
        // "Missing": fetcher returns None.
        // "ParseFail": fetcher returns invalid bytes.
        let fetcher = StubFetcher::new()
            .with("AbsentFont", None)
            .with("CorruptFont", Some(b"not a valid font".to_vec()));
        let resolver = FetcherFontResolver::new(fetcher);
        assert!(resolver.resolve("AbsentFont").is_none());
        assert!(resolver.resolve("CorruptFont").is_none());
        assert_eq!(resolver.missing_count(), 1);
        assert_eq!(resolver.parse_fail_count(), 1);
        assert_eq!(resolver.cache_len(), 2);
    }

    #[test]
    fn clear_negative_cache_drops_only_missing() {
        let fetcher = StubFetcher::new()
            .with("AbsentFont", None)
            .with("CorruptFont", Some(b"not a valid font".to_vec()));
        let resolver = FetcherFontResolver::new(fetcher);
        let _ = resolver.resolve("AbsentFont");
        let _ = resolver.resolve("CorruptFont");
        assert_eq!(resolver.cache_len(), 2);

        resolver.clear_negative_cache();

        // Missing entry dropped, ParseFail kept.
        assert_eq!(resolver.cache_len(), 1);
        assert_eq!(resolver.missing_count(), 0);
        assert_eq!(resolver.parse_fail_count(), 1);
    }

    #[test]
    fn forget_drops_specific_entry() {
        let fetcher = StubFetcher::new()
            .with("AbsentFont", None)
            .with("CorruptFont", Some(b"not a valid font".to_vec()));
        let resolver = FetcherFontResolver::new(fetcher);
        let _ = resolver.resolve("AbsentFont");
        let _ = resolver.resolve("CorruptFont");
        assert_eq!(resolver.cache_len(), 2);

        // Forget the ParseFail entry — clear_negative_cache wouldn't.
        assert!(resolver.forget("CorruptFont"));
        assert_eq!(resolver.cache_len(), 1);
        assert_eq!(resolver.parse_fail_count(), 0);

        // Forget unknown name returns false.
        assert!(!resolver.forget("NeverFetched"));
    }

    #[test]
    fn clear_cache_drops_everything_including_parse_fail() {
        let fetcher = StubFetcher::new()
            .with("AbsentFont", None)
            .with("CorruptFont", Some(b"not a valid font".to_vec()));
        let resolver = FetcherFontResolver::new(fetcher);
        let _ = resolver.resolve("AbsentFont");
        let _ = resolver.resolve("CorruptFont");
        resolver.clear_cache();
        assert_eq!(resolver.cache_len(), 0);
        assert_eq!(resolver.missing_count(), 0);
        assert_eq!(resolver.parse_fail_count(), 0);
    }

    #[test]
    fn forget_then_re_resolve_calls_fetcher_again() {
        // After forget(), the entry is gone — next resolve hits the
        // fetcher again. Verify by checking the resolver's behavior:
        // first call yields None (cached), forget, re-call yields
        // None again (re-fetched, also None).
        let fetcher = StubFetcher::new();
        let resolver = FetcherFontResolver::new(fetcher);
        assert!(resolver.resolve("X").is_none());
        assert_eq!(resolver.cache_len(), 1);
        assert!(resolver.forget("X"));
        assert_eq!(resolver.cache_len(), 0);
        assert!(resolver.resolve("X").is_none());
        assert_eq!(resolver.cache_len(), 1);
    }

    // -- Panicking fetcher tests (IP-19-B) -----------------------------------

    #[test]
    fn panicking_fetcher_does_not_poison_cache_mutex() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        // First call triggers a panic; subsequent calls must still work.
        let panicked = Arc::new(AtomicBool::new(false));
        let panicked_clone = Arc::clone(&panicked);
        let fetcher = move |family: &str| -> Option<Vec<u8>> {
            if family == "PanicFont" && !panicked_clone.load(Ordering::SeqCst) {
                panicked_clone.store(true, Ordering::SeqCst);
                panic!("simulated fetcher panic");
            }
            None
        };
        let resolver = FetcherFontResolver::new(fetcher);

        // First call: fetcher panics. resolve() must return None, not propagate panic.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            resolver.resolve("PanicFont")
        }));
        // Inner catch_unwind must have caught the panic — outer sees Ok.
        assert!(result.is_ok(), "panic must not propagate out of resolve()");
        assert!(result.unwrap().is_none());

        // Second call for a different name: resolver still functional.
        let result2 = resolver.resolve("AnotherFont");
        assert!(result2.is_none()); // mutex not poisoned
    }

    #[test]
    fn panicking_fetcher_result_is_parse_fail_in_cache() {
        // After a fetcher panic, the entry should be treated as ParseFail
        // so subsequent calls for the same name hit the cache (no re-panic).
        use std::sync::atomic::{AtomicUsize, Ordering as O};
        use std::sync::Arc;

        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = Arc::clone(&call_count);
        let fetcher = move |family: &str| -> Option<Vec<u8>> {
            call_count_clone.fetch_add(1, O::SeqCst);
            #[allow(clippy::manual_assert)]
            if family == "PanicFont" {
                panic!("simulated fetcher panic");
            }
            None
        };
        let resolver = FetcherFontResolver::new(fetcher);

        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            resolver.resolve("PanicFont")
        }));
        // Second resolve for the same name: must not call the fetcher again.
        let _ = resolver.resolve("PanicFont");
        assert_eq!(
            call_count.load(O::SeqCst),
            1,
            "fetcher called more than once"
        );
    }

    // -- Bounded cache tests (IP-20) -----------------------------------------

    #[test]
    fn cache_bounded_at_max_entries() {
        // Insert MAX_CACHE_ENTRIES + 1 distinct names.
        // The (MAX + 1)th name must still resolve (fail-open),
        // but the cache size must not exceed MAX_CACHE_ENTRIES.
        const N: usize = 1025;
        let resolver = FetcherFontResolver::new(|_: &str| None);
        for i in 0..N {
            let name = format!("Font{i}");
            let _ = resolver.resolve(&name);
        }
        assert!(
            resolver.cache_len() <= 1024,
            "cache exceeded 1024: {}",
            resolver.cache_len()
        );
    }

    #[test]
    fn cache_full_still_resolves_new_name() {
        // Even when the cache is full, resolve must return a value
        // (None for missing fonts) — not an error.
        let resolver = FetcherFontResolver::new(|_: &str| None);
        for i in 0..1025 {
            let name = format!("Font{i}");
            let _ = resolver.resolve(&name);
        }
        // 1025th distinct name: cache is full, but resolution returns None (not Err).
        let result = resolver.resolve("OverflowFont");
        assert!(result.is_none()); // correct result returned despite full cache
    }
}
