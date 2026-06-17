//! Slide-scoped renderer context for `<a:fld>` substitution.
//!
//! Mirrors. The key
//! differences from the spec port:
//!
//! 1. **No global mutable singleton.** TS uses module-level state; Rust
//!    threads `&SlideRenderContext` explicitly so the renderer stays
//!    parallel-safe.
//! 2. **Deterministic datetime fields.** TS calls
//!    `Date.toLocaleDateString()` / `toLocaleTimeString()` which depend
//!    on the host's `Intl` implementation and the wall-clock at render
//!    time. Rust requires the caller to supply an explicit
//!    [`Timestamp`] (`SlideRenderContext::timestamp`); the formatters
//!    themselves are locale-neutral so the same input always produces
//!    the same output. When `timestamp` is `None`, datetime fields
//!    return `None` and callers fall back to the placeholder text the
//! parser extracted (matching the unsupported-field behaviour TS
//!    uses for unknown types). This is an intentional divergence —
//!    deterministic output is non-negotiable for VRT and WASM ↔ native
//!    bit equivalence.

use slideglance_model::SlideHeaderFooter;

/// Field name produced by `<a:fld type="slidenum">`.
pub const FIELD_SLIDE_NUMBER: &str = "slidenum";

/// Calendar timestamp the renderer uses to resolve `datetime{N}` fields.
///
/// Pre-broken into components so the renderer never touches a system
/// clock or locale database. Hosts construct one explicitly (e.g. via
/// `chrono` / `time` on the Rust side, or `Date` field reads on the JS
/// side) and pass it in via [`SlideRenderContext::timestamp`].
///
/// Field semantics match `chrono::NaiveDateTime` ranges:
/// - `year` is the full Gregorian year (e.g. 2026).
/// - `month` is 1..=12.
/// - `day` is 1..=31 (caller-validated).
/// - `hour` is 0..=23.
/// - `minute` / `second` are 0..=59.
/// - `weekday` is 0=Monday .. 6=Sunday (ISO 8601 ordering).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timestamp {
    /// Full Gregorian year (e.g. 2026).
    pub year: i32,
    /// Month, 1..=12.
    pub month: u32,
    /// Day of month, 1..=31.
    pub day: u32,
    /// Hour, 0..=23 (24-hour clock).
    pub hour: u32,
    /// Minute, 0..=59.
    pub minute: u32,
    /// Second, 0..=59.
    pub second: u32,
    /// Day of week, 0=Monday .. 6=Sunday (ISO 8601).
    pub weekday: u8,
}

/// Per-slide rendering context. Constructed once per slide by the slide
/// renderer and passed by reference into element renderers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlideRenderContext {
    /// 1-based slide number for `slidenum` field substitution.
    pub slide_number: u32,
    /// Total number of slides in the deck. `None` when the renderer is
    /// invoked one slide at a time without that knowledge.
    pub total_slides: Option<u32>,
    /// Header/footer toggles inherited from the slide. Used to decide
    /// whether to render slide-number / datetime / footer placeholders.
    pub header_footer: Option<SlideHeaderFooter>,
    /// Caller-supplied wall-clock timestamp for `datetime{N}` field
    /// substitution. `None` -> datetime fields return `None` and the
    /// placeholder text the parser extracted is rendered verbatim.
    /// Required for deterministic output across native + WASM builds.
    pub timestamp: Option<Timestamp>,
}

impl SlideRenderContext {
    /// Construct a context with just a slide number — the most common case
    /// from TS parity tests.
    #[must_use]
    pub fn new(slide_number: u32) -> Self {
        Self {
            slide_number,
            total_slides: None,
            header_footer: None,
            timestamp: None,
        }
    }
}

/// Resolve a field type to its substituted text, or `None` if the field
/// is unknown or its prerequisites are missing.
///
/// `slidenum` is always implemented. `datetime{0..13}` is implemented
/// when [`SlideRenderContext::timestamp`] is `Some`; otherwise returns
/// `None` so the caller falls back to the placeholder text (TS-equivalent
/// behaviour for unknown fields).
#[must_use]
pub fn format_field(field_type: &str, ctx: &SlideRenderContext) -> Option<String> {
    if field_type == FIELD_SLIDE_NUMBER {
        return Some(ctx.slide_number.to_string());
    }
    let ts = ctx.timestamp?;
    format_datetime(field_type, &ts)
}

/// Convenience: when a renderer holds the context already and only needs the
/// slide-number string. Returns the literal slide number; never fails.
#[must_use]
pub fn slide_field_text(ctx: &SlideRenderContext) -> String {
    ctx.slide_number.to_string()
}

/// Locale-neutral datetime formatter for the OOXML `datetime{N}`
/// variants. Returns `None` for unknown variants.
///
/// Format choices follow `PowerPoint`'s `en-US` defaults but never
/// consult the host locale — output is byte-identical across machines
/// for the same `Timestamp`. See the module docs for the divergence
/// rationale.
fn format_datetime(field_type: &str, ts: &Timestamp) -> Option<String> {
    let out = match field_type {
        // datetime — short numeric date (M/D/YYYY).
        "datetime" => format!("{}/{}/{}", ts.month, ts.day, ts.year),
        // datetime1 — long date: "January 2, 2026"
        "datetime1" => format!("{} {}, {}", month_long(ts.month)?, ts.day, ts.year),
        // datetime2 — long date with weekday: "Monday, January 2, 2026"
        "datetime2" => format!(
            "{}, {} {}, {}",
            weekday_long(ts.weekday)?,
            month_long(ts.month)?,
            ts.day,
            ts.year
        ),
        // datetime3 — DD MMM YYYY
        "datetime3" => format!("{:02} {} {}", ts.day, month_short(ts.month)?, ts.year),
        // datetime4 — short date "M/D/YY"
        "datetime4" => format!("{}/{}/{:02}", ts.month, ts.day, ts.year.rem_euclid(100)),
        // datetime5 — D-MMM-YY
        "datetime5" => format!(
            "{}-{}-{:02}",
            ts.day,
            month_short(ts.month)?,
            ts.year.rem_euclid(100)
        ),
        // datetime6 — MMM DD, YYYY
        "datetime6" => format!("{} {:02}, {}", month_short(ts.month)?, ts.day, ts.year),
        // datetime7 — MMM-YY
        "datetime7" => format!("{}-{:02}", month_short(ts.month)?, ts.year.rem_euclid(100)),
        // datetime8 — short date + 24h time "M/D/YYYY HH:MM:SS"
        "datetime8" => format!(
            "{}/{}/{} {:02}:{:02}:{:02}",
            ts.month, ts.day, ts.year, ts.hour, ts.minute, ts.second
        ),
        // datetime9 — short date + HH:MM
        "datetime9" => format!(
            "{}/{}/{} {:02}:{:02}",
            ts.month, ts.day, ts.year, ts.hour, ts.minute
        ),
        // datetime10 / datetime13 — full 24h time "HH:MM:SS"
        // (datetime13 is intentionally an alias of datetime10 to mirror
        // PowerPoint's redundant entries in the field-type table).
        "datetime10" | "datetime13" => {
            format!("{:02}:{:02}:{:02}", ts.hour, ts.minute, ts.second)
        }
        // datetime11 — 12-hour time "h:MM AM/PM"
        "datetime11" => {
            let (h12, suffix) = to_12_hour(ts.hour);
            format!("{h12}:{:02} {suffix}", ts.minute)
        }
        // datetime12 — 24h "HH:MM"
        "datetime12" => format!("{:02}:{:02}", ts.hour, ts.minute),
        _ => return None,
    };
    Some(out)
}

fn month_long(m: u32) -> Option<&'static str> {
    Some(match m {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => return None,
    })
}

fn month_short(m: u32) -> Option<&'static str> {
    Some(match m {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => return None,
    })
}

fn weekday_long(w: u8) -> Option<&'static str> {
    Some(match w {
        0 => "Monday",
        1 => "Tuesday",
        2 => "Wednesday",
        3 => "Thursday",
        4 => "Friday",
        5 => "Saturday",
        6 => "Sunday",
        _ => return None,
    })
}

fn to_12_hour(h: u32) -> (u32, &'static str) {
    match h {
        0 => (12, "AM"),
        1..=11 => (h, "AM"),
        12 => (12, "PM"),
        _ => (h - 12, "PM"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts() -> Timestamp {
        // 2026-01-02 (Friday) 13:04:05 — distinguishable in every variant.
        Timestamp {
            year: 2026,
            month: 1,
            day: 2,
            hour: 13,
            minute: 4,
            second: 5,
            weekday: 4, // Friday
        }
    }

    fn ctx_with_ts(t: Timestamp) -> SlideRenderContext {
        let mut c = SlideRenderContext::new(7);
        c.timestamp = Some(t);
        c
    }

    #[test]
    fn slidenum_substitution() {
        let ctx = SlideRenderContext::new(7);
        assert_eq!(format_field(FIELD_SLIDE_NUMBER, &ctx).as_deref(), Some("7"));
    }

    #[test]
    fn unknown_field_returns_none() {
        let ctx = SlideRenderContext::new(1);
        assert_eq!(format_field("unrecognized", &ctx), None);
    }

    #[test]
    fn datetime_without_timestamp_returns_none() {
        // Without a caller-supplied timestamp, datetime fields fall back
        // to the placeholder text — preserves prior behaviour.
        let ctx = SlideRenderContext::new(1);
        for v in [
            "datetime",
            "datetime1",
            "datetime2",
            "datetime3",
            "datetime4",
            "datetime5",
            "datetime6",
            "datetime7",
            "datetime8",
            "datetime9",
            "datetime10",
            "datetime11",
            "datetime12",
            "datetime13",
        ] {
            assert!(
                format_field(v, &ctx).is_none(),
                "{v} unexpectedly substituted"
            );
        }
    }

    #[test]
    fn datetime_short_numeric() {
        let ctx = ctx_with_ts(ts());
        assert_eq!(format_field("datetime", &ctx).as_deref(), Some("1/2/2026"));
    }

    #[test]
    fn datetime1_long_date() {
        let ctx = ctx_with_ts(ts());
        assert_eq!(
            format_field("datetime1", &ctx).as_deref(),
            Some("January 2, 2026")
        );
    }

    #[test]
    fn datetime2_long_with_weekday() {
        let ctx = ctx_with_ts(ts());
        assert_eq!(
            format_field("datetime2", &ctx).as_deref(),
            Some("Friday, January 2, 2026")
        );
    }

    #[test]
    fn datetime3_dd_mmm_yyyy() {
        let ctx = ctx_with_ts(ts());
        assert_eq!(
            format_field("datetime3", &ctx).as_deref(),
            Some("02 Jan 2026")
        );
    }

    #[test]
    fn datetime4_short_with_two_digit_year() {
        let ctx = ctx_with_ts(ts());
        assert_eq!(format_field("datetime4", &ctx).as_deref(), Some("1/2/26"));
    }

    #[test]
    fn datetime5_d_mmm_yy() {
        let ctx = ctx_with_ts(ts());
        assert_eq!(format_field("datetime5", &ctx).as_deref(), Some("2-Jan-26"));
    }

    #[test]
    fn datetime6_mmm_dd_yyyy() {
        let ctx = ctx_with_ts(ts());
        assert_eq!(
            format_field("datetime6", &ctx).as_deref(),
            Some("Jan 02, 2026")
        );
    }

    #[test]
    fn datetime7_mmm_yy() {
        let ctx = ctx_with_ts(ts());
        assert_eq!(format_field("datetime7", &ctx).as_deref(), Some("Jan-26"));
    }

    #[test]
    fn datetime8_date_and_full_time() {
        let ctx = ctx_with_ts(ts());
        assert_eq!(
            format_field("datetime8", &ctx).as_deref(),
            Some("1/2/2026 13:04:05")
        );
    }

    #[test]
    fn datetime9_date_and_hhmm() {
        let ctx = ctx_with_ts(ts());
        assert_eq!(
            format_field("datetime9", &ctx).as_deref(),
            Some("1/2/2026 13:04")
        );
    }

    #[test]
    fn datetime10_full_24h_time() {
        let ctx = ctx_with_ts(ts());
        assert_eq!(
            format_field("datetime10", &ctx).as_deref(),
            Some("13:04:05")
        );
    }

    #[test]
    fn datetime11_12_hour_time_pm() {
        let ctx = ctx_with_ts(ts());
        assert_eq!(format_field("datetime11", &ctx).as_deref(), Some("1:04 PM"));
    }

    #[test]
    fn datetime11_12_hour_midnight_is_am() {
        let mut t = ts();
        t.hour = 0;
        let ctx = ctx_with_ts(t);
        assert_eq!(
            format_field("datetime11", &ctx).as_deref(),
            Some("12:04 AM")
        );
    }

    #[test]
    fn datetime11_12_hour_noon_is_pm() {
        let mut t = ts();
        t.hour = 12;
        let ctx = ctx_with_ts(t);
        assert_eq!(
            format_field("datetime11", &ctx).as_deref(),
            Some("12:04 PM")
        );
    }

    #[test]
    fn datetime12_24h_hhmm() {
        let ctx = ctx_with_ts(ts());
        assert_eq!(format_field("datetime12", &ctx).as_deref(), Some("13:04"));
    }

    #[test]
    fn datetime13_24h_hhmmss() {
        let ctx = ctx_with_ts(ts());
        assert_eq!(
            format_field("datetime13", &ctx).as_deref(),
            Some("13:04:05")
        );
    }

    #[test]
    fn datetime_invalid_month_returns_none() {
        // Month 0 / 13 means caller-validated invariant violation;
        // formatter signals via None instead of panicking.
        let mut t = ts();
        t.month = 0;
        let ctx = ctx_with_ts(t);
        assert!(format_field("datetime1", &ctx).is_none());
    }

    #[test]
    fn datetime_two_digit_year_handles_negative_modulo() {
        let mut t = ts();
        t.year = 2007;
        let ctx = ctx_with_ts(t);
        assert_eq!(format_field("datetime4", &ctx).as_deref(), Some("1/2/07"));
    }

    #[test]
    fn slide_field_text_returns_decimal() {
        let ctx = SlideRenderContext::new(42);
        assert_eq!(slide_field_text(&ctx), "42");
    }

    #[test]
    fn context_optional_fields_default_none() {
        let ctx = SlideRenderContext::new(1);
        assert!(ctx.total_slides.is_none());
        assert!(ctx.header_footer.is_none());
        assert!(ctx.timestamp.is_none());
    }
}
