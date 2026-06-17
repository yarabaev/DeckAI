// Same rationale as `mapping.rs` / `fallback_metrics.rs` — OOXML / OS
// font names are mixed-case proper nouns, not code identifiers.
#![allow(clippy::doc_markdown)]

//! Per-OS local CJK fallback chains for mapped Noto family names.
//!
//! When [`crate::mapping::get_mapped_font`] yields e.g. `Noto Sans JP`
//! but the host has no Noto installed, the renderer walks the chain
//! returned here and registers each entry as the next-best face.
//!
//! ## Ordering rule (project-explicit)
//!
//! Each chain is ordered:
//!
//! 1. **OS-preinstalled fonts first** — fonts that ship with the OS
//!    distribution itself (Apple's macOS, Microsoft's Windows, the
//!    Noto / `WenQuanYi` packages typically pulled in by Linux desktop
//!    environments). These are "almost certainly available" without
//!    user / admin action.
//! 2. **Commonly-installed OSS / community fonts** — popular OSS
//!    families a host *might* have installed but that are not part of
//!    the OS image (Pretendard, Nanum, Source Han, IPA on Linux).
//!    These are "likely available on developer machines".
//! 3. **Pan-CJK Noto aliases** — last resort, in case the host has
//!    only the pan-CJK Noto file installed instead of the language-
//!    standalone variant.
//!
//! The previous revision (Rust port) put Pretendard at the top of the
//! macOS Korean chain even though Pretendard is not preinstalled —
//! Apple SD Gothic Neo is. The new ordering corrects that.
//!
//! ## CJK Script Equality
//!
//! All four CJK scripts (Jpan / Hang / Hans / Hant) get full
//! coverage on every supported platform. This is a project-explicit
//! divergence from TS, which had no Chinese entries and no Linux
//! Japanese entries. Adding them ensures no script is silently
//! dropped at the resolver level.
//!
//! Linux Japanese was previously empty (TS quirk: "Linux Noto installs
//! ship JP directly so the chain rarely fires"). We add Source Han
//! Sans JP / Serif JP / IPAGothic / IPAMincho / pan-CJK aliases for
//! symmetry — costs ~10 entries, prevents a silently-empty chain when
//! a Linux user has Noto in a non-default location.

/// Operating system enumeration for fallback selection.
///
/// Maps to [`std::env::consts::OS`] strings via [`CjkPlatform::current`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CjkPlatform {
    /// Apple macOS.
    MacOs,
    /// Microsoft Windows.
    Windows,
    /// Linux distributions.
    Linux,
    /// Any other target — yields empty fallback lists (matching the TS
    /// `default` branch in ).
    Other,
}

impl CjkPlatform {
    /// Resolves the platform from the host running this binary.
    ///
    /// Mirrors the TS `platform` from `node:os` — `"darwin"` →
    /// `MacOs`, `"win32"` → `Windows`, `"linux"` → `Linux`, anything else
    /// → `Other`.
    #[must_use]
    pub fn current() -> Self {
        match std::env::consts::OS {
            "macos" => Self::MacOs,
            "windows" => Self::Windows,
            "linux" => Self::Linux,
            _ => Self::Other,
        }
    }
}

/// Returns a unified CJK fallback chain for `mapped_font_name`,
/// concatenating every platform's preinstalled / OSS / pan-CJK
/// fallbacks in a single order so the resulting SVG `font-family`
/// chain — and, on the native viewer, the `OpentypeTextMeasurer`'s
/// resolver chain — is the same regardless of which OS is parsing.
///
/// CSS / WebView font matching picks the first listed family the host
/// actually has, so the unified chain naturally degrades:
///
/// - Windows host: `Malgun Gothic` / `Yu Gothic` / `Microsoft YaHei`
///   wins (preinstalled).
/// - macOS host: `Apple SD Gothic Neo` / `Hiragino Sans` /
///   `PingFang SC` wins (preinstalled).
/// - macOS host with Office installed: the Windows-native family
///   installed via Office wins (matches PowerPoint's render of the
///   same deck on Windows).
/// - Linux host: Noto / Source Han / IPA wins (commonly installed via
///   distro fontconfig defaults).
///
/// Order: Windows-native first (most decks are authored on Windows so
/// the original PPT font is the closest visual match), macOS-native
/// second, Linux community OSS / pan-CJK third. Within each section
/// the per-OS table's existing order is preserved. Duplicates are
/// filtered out.
///
/// `platform` is currently ignored — kept in the signature for source
/// compatibility with callers that already pass `CjkPlatform::current()`
/// and to leave room for future per-platform reordering. Returns an
/// owned `Vec` rather than a `&'static [&str]` because the unified
/// chain is materialised on the fly. Callers iterate the result and
/// the allocation cost is negligible (under ten string-pointers per
/// lookup).
#[must_use]
pub fn get_cjk_fallback_fonts(_platform: CjkPlatform, mapped_font_name: &str) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    let mut seen: std::collections::HashSet<&'static str> = std::collections::HashSet::new();
    let push_chain = |chain: &'static [&'static str],
                      out: &mut Vec<&'static str>,
                      seen: &mut std::collections::HashSet<&'static str>| {
        for f in chain {
            if seen.insert(*f) {
                out.push(*f);
            }
        }
    };
    let lookup = |table: &'static [(&'static str, &'static [&'static str])]| {
        for (key, fallbacks) in table {
            if *key == mapped_font_name {
                return *fallbacks;
            }
        }
        &[] as &'static [&'static str]
    };
    push_chain(lookup(WINDOWS_FALLBACKS), &mut out, &mut seen);
    push_chain(lookup(MACOS_FALLBACKS), &mut out, &mut seen);
    push_chain(lookup(LINUX_FALLBACKS), &mut out, &mut seen);
    out
}

// ---------------------------------------------------------------------------
// macOS fallbacks
// ---------------------------------------------------------------------------

// Order in every chain: OS-preinstalled → community OSS → pan-CJK
// Noto alias.
const MACOS_FALLBACKS: &[(&str, &[&str])] = &[
    // ── PPTX-authored CJK typefaces map to the local macOS prerequisites.
    // PowerPoint decks frequently declare "Yu Gothic" / "Meiryo" / "MS
    // Gothic" / "맑은 고딕" / "微软雅黑" directly in `<a:latin>` or
    // `<a:ea typeface="…">`. Listing those names here lets the
    // font-family chain emit the macOS-installed equivalent first, so
    // the SVG renders with the deck's intended visual weight without
    // forcing a font swap on the user's machine.
    (
        "Yu Gothic",
        &[
            "Hiragino Sans",
            "Hiragino Kaku Gothic ProN",
            "Hiragino Kaku Gothic Pro",
        ],
    ),
    (
        "Yu Gothic UI",
        &["Hiragino Sans", "Hiragino Kaku Gothic ProN"],
    ),
    (
        "Yu Gothic Medium",
        &["Hiragino Sans", "Hiragino Kaku Gothic ProN"],
    ),
    (
        "Yu Gothic Light",
        &["Hiragino Sans", "Hiragino Kaku Gothic ProN"],
    ),
    (
        "Yu Gothic Bold",
        &["Hiragino Sans", "Hiragino Kaku Gothic ProN"],
    ),
    (
        "Yu Mincho",
        &["Hiragino Mincho ProN", "Hiragino Mincho Pro"],
    ),
    ("Meiryo", &["Hiragino Sans", "Hiragino Kaku Gothic ProN"]),
    ("Meiryo UI", &["Hiragino Sans", "Hiragino Kaku Gothic ProN"]),
    ("MS Gothic", &["Hiragino Sans", "Hiragino Kaku Gothic ProN"]),
    (
        "MS PGothic",
        &["Hiragino Sans", "Hiragino Kaku Gothic ProN"],
    ),
    (
        "MS UI Gothic",
        &["Hiragino Sans", "Hiragino Kaku Gothic ProN"],
    ),
    (
        "MS Mincho",
        &["Hiragino Mincho ProN", "Hiragino Mincho Pro"],
    ),
    (
        "MS PMincho",
        &["Hiragino Mincho ProN", "Hiragino Mincho Pro"],
    ),
    // Fullwidth forms — PowerPoint themes authored in Japanese locales
    // emit the typeface name with fullwidth Latin characters (U+FF21
    // FULLWIDTH LATIN CAPITAL LETTER A etc.). Cover both spellings so
    // the lookup hits regardless of which form the deck stores.
    (
        "ＭＳ Ｐゴシック",
        &["Hiragino Sans", "Hiragino Kaku Gothic ProN"],
    ),
    (
        "ＭＳ ゴシック",
        &["Hiragino Sans", "Hiragino Kaku Gothic ProN"],
    ),
    (
        "ＭＳ Ｐ明朝",
        &["Hiragino Mincho ProN", "Hiragino Mincho Pro"],
    ),
    (
        "ＭＳ 明朝",
        &["Hiragino Mincho ProN", "Hiragino Mincho Pro"],
    ),
    (
        "游ゴシック",
        &[
            "Hiragino Sans",
            "Hiragino Kaku Gothic ProN",
            "Hiragino Kaku Gothic Pro",
        ],
    ),
    ("游明朝", &["Hiragino Mincho ProN", "Hiragino Mincho Pro"]),
    ("メイリオ", &["Hiragino Sans", "Hiragino Kaku Gothic ProN"]),
    (
        "Malgun Gothic",
        &["Apple SD Gothic Neo", "AppleSDGothicNeo", "AppleGothic"],
    ),
    ("MalgunGothic", &["Apple SD Gothic Neo", "AppleSDGothicNeo"]),
    (
        "맑은 고딕",
        &["Apple SD Gothic Neo", "AppleSDGothicNeo", "AppleGothic"],
    ),
    ("Batang", &["AppleMyungjo", "Apple Myungjo"]),
    ("바탕", &["AppleMyungjo", "Apple Myungjo"]),
    ("Gulim", &["Apple SD Gothic Neo", "AppleGothic"]),
    ("굴림", &["Apple SD Gothic Neo", "AppleGothic"]),
    ("Dotum", &["Apple SD Gothic Neo", "AppleGothic"]),
    ("돋움", &["Apple SD Gothic Neo", "AppleGothic"]),
    // ── HY (한양정보통신) commercial fonts shipped with HWP / Office on
    // Windows. Not redistributable; map onto the closest macOS prereq
    // by glyph weight. HY신명조 = serif, HY헤드라인M = display gothic.
    (
        "HY신명조",
        &["AppleMyungjo", "Apple Myungjo", "Nanum Myeongjo"],
    ),
    (
        "HY헤드라인M",
        &[
            "Apple SD Gothic Neo",
            "Pretendard Black",
            "Pretendard ExtraBold",
            "AppleGothic",
        ],
    ),
    (
        "HY그래픽M",
        &["Apple SD Gothic Neo", "Pretendard Bold", "AppleGothic"],
    ),
    (
        "HY견고딕",
        &["Apple SD Gothic Neo", "Pretendard Black", "AppleGothic"],
    ),
    // ── KoPub (한국출판인회의) free family. Dotum (sans / 돋움) variant
    // isn't on Google Fonts; route to Apple SD Gothic Neo at matching
    // weight. KoPub Batang (서체) the install script does pull when
    // available, so the inner BufferFontResolver should hit before us
    // for the Batang ones.
    (
        "KoPub돋움체 Bold",
        &[
            "Apple SD Gothic Neo Bold",
            "Apple SD Gothic Neo",
            "Pretendard Bold",
        ],
    ),
    (
        "KoPub돋움체 Medium",
        &["Apple SD Gothic Neo", "Pretendard Medium"],
    ),
    (
        "KoPub돋움체 Light",
        &[
            "Apple SD Gothic Neo Light",
            "Apple SD Gothic Neo",
            "Pretendard Light",
        ],
    ),
    (
        "KoPubWorld돋움체 Bold",
        &[
            "Apple SD Gothic Neo Bold",
            "Apple SD Gothic Neo",
            "Pretendard Bold",
        ],
    ),
    (
        "KoPubWorld돋움체 Medium",
        &["Apple SD Gothic Neo", "Pretendard Medium"],
    ),
    (
        "KoPubWorld돋움체 Light",
        &[
            "Apple SD Gothic Neo Light",
            "Apple SD Gothic Neo",
            "Pretendard Light",
        ],
    ),
    // ── Adobe Source Han Sans / 본고딕 — installed locally as the VF
    // collection ("Source Han Sans K VF" / "본고딕 VF"). Bridge the
    // bare-name spellings the deck uses to the actual face name.
    (
        "Source Han Sans KR",
        &[
            "Source Han Sans K VF",
            "Source Han Sans K",
            "본고딕 VF",
            "본고딕",
        ],
    ),
    ("Source Han Sans K", &["Source Han Sans K VF", "본고딕 VF"]),
    ("본고딕", &["본고딕 VF", "Source Han Sans K VF"]),
    // ── Yoondesign / Rix / Monotype — paid; route to closest free.
    (
        "-윤고딕340",
        &[
            "Apple SD Gothic Neo",
            "Pretendard",
            "Spoqa Han Sans Neo Medium",
        ],
    ),
    (
        "Rix모던고딕 B",
        &[
            "Apple SD Gothic Neo Bold",
            "Pretendard Bold",
            "Spoqa Han Sans Neo Bold",
        ],
    ),
    (
        "Monotype Sorts",
        &["Wingdings", "Wingdings 2", "Apple Symbols"],
    ),
    ("Microsoft YaHei", &["PingFang SC", "Heiti SC", "STHeiti"]),
    ("Microsoft YaHei UI", &["PingFang SC", "Heiti SC"]),
    ("微软雅黑", &["PingFang SC", "Heiti SC", "STHeiti"]),
    ("SimSun", &["Songti SC", "STSong"]),
    ("宋体", &["Songti SC", "STSong"]),
    ("SimHei", &["PingFang SC", "Heiti SC", "STHeiti"]),
    ("黑体", &["PingFang SC", "Heiti SC"]),
    (
        "Microsoft JhengHei",
        &["PingFang TC", "PingFang HK", "Heiti TC"],
    ),
    ("Microsoft JhengHei UI", &["PingFang TC", "Heiti TC"]),
    ("微軟正黑體", &["PingFang TC", "PingFang HK", "Heiti TC"]),
    ("PMingLiU", &["Songti TC", "LiSong Pro", "BiauKai"]),
    ("新細明體", &["Songti TC", "LiSong Pro"]),
    ("MingLiU", &["Songti TC", "LiSong Pro"]),
    ("細明體", &["Songti TC", "LiSong Pro"]),
    // Japanese gothic — Hiragino is preinstalled since macOS 10.4.
    (
        "Noto Sans JP",
        &[
            // OS-preinstalled
            "Hiragino Sans",
            "Hiragino Kaku Gothic ProN",
            "Hiragino Kaku Gothic Pro",
            // Community OSS
            "Source Han Sans JP",
            // Pan-CJK
            "Noto Sans CJK JP",
        ],
    ),
    (
        "Noto Sans CJK JP",
        &[
            "Hiragino Sans",
            "Hiragino Kaku Gothic ProN",
            "Hiragino Kaku Gothic Pro",
            "Source Han Sans JP",
        ],
    ),
    // Japanese mincho — Hiragino Mincho preinstalled.
    (
        "Noto Serif JP",
        &[
            "Hiragino Mincho ProN",
            "Hiragino Mincho Pro",
            "Source Han Serif JP",
            "Noto Serif CJK JP",
        ],
    ),
    (
        "Noto Serif CJK JP",
        &[
            "Hiragino Mincho ProN",
            "Hiragino Mincho Pro",
            "Source Han Serif JP",
        ],
    ),
    // Korean sans-serif — Apple SD Gothic Neo preinstalled since
    // macOS 10.10. Pretendard / Nanum are community OSS.
    (
        "Noto Sans KR",
        &[
            // OS-preinstalled
            "Apple SD Gothic Neo",
            "AppleSDGothicNeo",
            "AppleGothic",
            // Community OSS
            "Pretendard",
            "Pretendard Variable",
            "NanumGothic",
            "NanumBarunGothic",
            "Source Han Sans KR",
            // Pan-CJK
            "Noto Sans CJK KR",
        ],
    ),
    (
        "Noto Sans CJK KR",
        &[
            "Apple SD Gothic Neo",
            "AppleSDGothicNeo",
            "AppleGothic",
            "Pretendard",
            "Pretendard Variable",
            "NanumGothic",
            "NanumBarunGothic",
            "Source Han Sans KR",
        ],
    ),
    // Korean serif — AppleMyungjo preinstalled.
    (
        "Noto Serif KR",
        &[
            "AppleMyungjo",
            "Apple Myungjo",
            "NanumMyeongjo",
            "Nanum Myeongjo",
            "Source Han Serif KR",
            "Noto Serif CJK KR",
        ],
    ),
    (
        "Noto Serif CJK KR",
        &[
            "AppleMyungjo",
            "Apple Myungjo",
            "NanumMyeongjo",
            "Nanum Myeongjo",
            "Source Han Serif KR",
        ],
    ),
    // Chinese Simplified — PingFang SC preinstalled since macOS 10.11.
    (
        "Noto Sans SC",
        &[
            "PingFang SC",
            "Heiti SC",
            "STHeiti",
            "Source Han Sans SC",
            "Noto Sans CJK SC",
        ],
    ),
    (
        "Noto Sans CJK SC",
        &["PingFang SC", "Heiti SC", "STHeiti", "Source Han Sans SC"],
    ),
    // Chinese Simplified serif — Songti SC / STSong preinstalled.
    (
        "Noto Serif SC",
        &[
            "Songti SC",
            "STSong",
            "Source Han Serif SC",
            "Noto Serif CJK SC",
        ],
    ),
    (
        "Noto Serif CJK SC",
        &["Songti SC", "STSong", "Source Han Serif SC"],
    ),
    // Chinese Traditional — PingFang TC preinstalled since macOS 10.11.
    (
        "Noto Sans TC",
        &[
            "PingFang TC",
            "PingFang HK",
            "Heiti TC",
            "Source Han Sans TC",
            "Noto Sans CJK TC",
        ],
    ),
    (
        "Noto Sans CJK TC",
        &[
            "PingFang TC",
            "PingFang HK",
            "Heiti TC",
            "Source Han Sans TC",
        ],
    ),
    // Chinese Traditional serif — Songti TC / LiSong preinstalled.
    (
        "Noto Serif TC",
        &[
            "Songti TC",
            "LiSong Pro",
            "BiauKai",
            "Source Han Serif TC",
            "Noto Serif CJK TC",
        ],
    ),
    (
        "Noto Serif CJK TC",
        &["Songti TC", "LiSong Pro", "BiauKai", "Source Han Serif TC"],
    ),
];

// ---------------------------------------------------------------------------
// Windows fallbacks
// ---------------------------------------------------------------------------

const WINDOWS_FALLBACKS: &[(&str, &[&str])] = &[
    // ── PPTX-authored CJK typefaces map to local Windows variants of
    // the same family. PowerPoint authors freely between "Yu Gothic" /
    // "Yu Gothic UI" / "Yu Gothic Medium" / "Meiryo" — listing the
    // siblings here means the SVG renders with whichever variant the
    // host actually has installed.
    (
        "Yu Gothic",
        &["Yu Gothic UI", "Yu Gothic Medium", "Meiryo", "MS Gothic"],
    ),
    ("Yu Gothic UI", &["Yu Gothic", "Yu Gothic Medium", "Meiryo"]),
    ("Yu Gothic Medium", &["Yu Gothic", "Yu Gothic UI", "Meiryo"]),
    ("Yu Gothic Light", &["Yu Gothic", "Yu Gothic UI", "Meiryo"]),
    ("Yu Mincho", &["MS Mincho"]),
    ("Meiryo", &["Yu Gothic", "Yu Gothic UI", "MS Gothic"]),
    ("Meiryo UI", &["Meiryo", "Yu Gothic UI", "Yu Gothic"]),
    ("MS Gothic", &["Yu Gothic", "Yu Gothic UI", "Meiryo"]),
    ("MS PGothic", &["MS Gothic", "Yu Gothic", "Meiryo"]),
    ("MS UI Gothic", &["MS Gothic", "Yu Gothic UI", "Meiryo"]),
    ("MS Mincho", &["Yu Mincho", "MS PMincho"]),
    ("MS PMincho", &["MS Mincho", "Yu Mincho"]),
    // Fullwidth Japanese typeface names — PowerPoint themes authored
    // in Japanese locales sometimes store these forms.
    (
        "ＭＳ Ｐゴシック",
        &["MS PGothic", "MS Gothic", "Yu Gothic UI", "Meiryo"],
    ),
    (
        "ＭＳ ゴシック",
        &["MS Gothic", "MS PGothic", "Yu Gothic UI", "Meiryo"],
    ),
    ("ＭＳ Ｐ明朝", &["MS PMincho", "MS Mincho", "Yu Mincho"]),
    ("ＭＳ 明朝", &["MS Mincho", "MS PMincho", "Yu Mincho"]),
    (
        "游ゴシック",
        &["Yu Gothic", "Yu Gothic UI", "Yu Gothic Medium"],
    ),
    ("游明朝", &["Yu Mincho", "MS Mincho"]),
    (
        "メイリオ",
        &["Meiryo", "Meiryo UI", "Yu Gothic", "MS Gothic"],
    ),
    (
        "Malgun Gothic",
        &["MalgunGothic", "맑은 고딕", "Dotum", "Gulim"],
    ),
    (
        "MalgunGothic",
        &["Malgun Gothic", "맑은 고딕", "Dotum", "Gulim"],
    ),
    (
        "맑은 고딕",
        &["Malgun Gothic", "MalgunGothic", "Dotum", "Gulim"],
    ),
    ("Batang", &["바탕", "Gungsuh", "궁서"]),
    ("Gulim", &["굴림", "Dotum", "돋움"]),
    ("Dotum", &["돋움", "Gulim", "굴림"]),
    (
        "Microsoft YaHei",
        &["Microsoft YaHei UI", "微软雅黑", "SimHei", "黑体"],
    ),
    (
        "Microsoft YaHei UI",
        &["Microsoft YaHei", "微软雅黑", "SimHei"],
    ),
    (
        "微软雅黑",
        &["Microsoft YaHei", "Microsoft YaHei UI", "SimHei"],
    ),
    ("SimSun", &["宋体", "NSimSun", "FangSong"]),
    ("宋体", &["SimSun", "NSimSun"]),
    ("SimHei", &["Microsoft YaHei", "黑体"]),
    ("黑体", &["SimHei", "Microsoft YaHei"]),
    (
        "Microsoft JhengHei",
        &["Microsoft JhengHei UI", "微軟正黑體", "PMingLiU"],
    ),
    (
        "Microsoft JhengHei UI",
        &["Microsoft JhengHei", "微軟正黑體"],
    ),
    (
        "微軟正黑體",
        &["Microsoft JhengHei", "Microsoft JhengHei UI"],
    ),
    ("PMingLiU", &["新細明體", "MingLiU", "細明體"]),
    ("新細明體", &["PMingLiU", "MingLiU"]),
    ("MingLiU", &["細明體", "PMingLiU"]),
    ("細明體", &["MingLiU", "PMingLiU"]),
    // Japanese gothic — Yu Gothic preinstalled since Windows 8.1.
    ("Noto Sans JP", &["Yu Gothic", "Meiryo", "MS Gothic"]),
    ("Noto Sans CJK JP", &["Yu Gothic", "Meiryo", "MS Gothic"]),
    // Japanese mincho — Yu Mincho preinstalled.
    ("Noto Serif JP", &["Yu Mincho", "MS Mincho"]),
    ("Noto Serif CJK JP", &["Yu Mincho", "MS Mincho"]),
    // Korean sans-serif: Pretendard, then Windows preinstalled.
    (
        "Noto Sans KR",
        &[
            "Pretendard",
            "Malgun Gothic",
            "MalgunGothic",
            "맑은 고딕",
            "Dotum",
            "Gulim",
            "Noto Sans CJK KR",
        ],
    ),
    (
        "Noto Sans CJK KR",
        &[
            "Pretendard",
            "Malgun Gothic",
            "MalgunGothic",
            "맑은 고딕",
            "Dotum",
            "Gulim",
        ],
    ),
    // Korean serif
    (
        "Noto Serif KR",
        &["Batang", "바탕", "Gungsuh", "궁서", "Noto Serif CJK KR"],
    ),
    ("Noto Serif CJK KR", &["Batang", "바탕", "Gungsuh", "궁서"]),
    // Chinese Simplified — added per CJK Script Equality.
    // Microsoft YaHei has shipped with Windows since Vista as the default
    // Chinese sans; SimSun / SimHei are legacy preinstalls.
    (
        "Noto Sans SC",
        &[
            "Microsoft YaHei",
            "Microsoft YaHei UI",
            "微软雅黑",
            "SimHei",
            "黑体",
            "Source Han Sans SC",
            "Noto Sans CJK SC",
        ],
    ),
    (
        "Noto Sans CJK SC",
        &[
            "Microsoft YaHei",
            "Microsoft YaHei UI",
            "微软雅黑",
            "SimHei",
            "黑体",
            "Source Han Sans SC",
        ],
    ),
    (
        "Noto Serif SC",
        &[
            "SimSun",
            "宋体",
            "NSimSun",
            "FangSong",
            "Source Han Serif SC",
            "Noto Serif CJK SC",
        ],
    ),
    (
        "Noto Serif CJK SC",
        &["SimSun", "宋体", "NSimSun", "Source Han Serif SC"],
    ),
    // Chinese Traditional — added per CJK Script Equality.
    (
        "Noto Sans TC",
        &[
            "Microsoft JhengHei",
            "Microsoft JhengHei UI",
            "微軟正黑體",
            "Source Han Sans TC",
            "Noto Sans CJK TC",
        ],
    ),
    (
        "Noto Sans CJK TC",
        &[
            "Microsoft JhengHei",
            "Microsoft JhengHei UI",
            "微軟正黑體",
            "Source Han Sans TC",
        ],
    ),
    (
        "Noto Serif TC",
        &[
            "PMingLiU",
            "新細明體",
            "MingLiU",
            "細明體",
            "Source Han Serif TC",
            "Noto Serif CJK TC",
        ],
    ),
    (
        "Noto Serif CJK TC",
        &[
            "PMingLiU",
            "新細明體",
            "MingLiU",
            "細明體",
            "Source Han Serif TC",
        ],
    ),
];

// ---------------------------------------------------------------------------
// Linux fallbacks
// ---------------------------------------------------------------------------

// Order: OS-preinstalled (rare on Linux — most distros ship Noto via
// fontconfig defaults), then community OSS (Source Han, IPA,
// `WenQuanYi`, Nanum), then pan-CJK Noto alias.
const LINUX_FALLBACKS: &[(&str, &[&str])] = &[
    // ── PPTX-authored CJK typefaces map to Noto / Source Han / IPA on
    // Linux — the families most distros ship out of the box. Listing
    // PPTX names here lets a Linux viewer fall through to whichever
    // pan-CJK family the user has installed without rewriting the SVG.
    (
        "Yu Gothic",
        &["Noto Sans CJK JP", "Source Han Sans JP", "IPAGothic"],
    ),
    ("Yu Gothic UI", &["Noto Sans CJK JP", "Source Han Sans JP"]),
    (
        "Yu Mincho",
        &["Noto Serif CJK JP", "Source Han Serif JP", "IPAMincho"],
    ),
    (
        "Meiryo",
        &["Noto Sans CJK JP", "Source Han Sans JP", "IPAGothic"],
    ),
    (
        "MS Gothic",
        &["Noto Sans CJK JP", "Source Han Sans JP", "IPAGothic"],
    ),
    (
        "MS Mincho",
        &["Noto Serif CJK JP", "Source Han Serif JP", "IPAMincho"],
    ),
    // Fullwidth Japanese forms — same fallback chain.
    (
        "ＭＳ Ｐゴシック",
        &["Noto Sans CJK JP", "Source Han Sans JP", "IPAGothic"],
    ),
    (
        "ＭＳ ゴシック",
        &["Noto Sans CJK JP", "Source Han Sans JP", "IPAGothic"],
    ),
    (
        "ＭＳ Ｐ明朝",
        &["Noto Serif CJK JP", "Source Han Serif JP", "IPAMincho"],
    ),
    (
        "ＭＳ 明朝",
        &["Noto Serif CJK JP", "Source Han Serif JP", "IPAMincho"],
    ),
    (
        "游ゴシック",
        &["Noto Sans CJK JP", "Source Han Sans JP", "IPAGothic"],
    ),
    (
        "游明朝",
        &["Noto Serif CJK JP", "Source Han Serif JP", "IPAMincho"],
    ),
    (
        "メイリオ",
        &["Noto Sans CJK JP", "Source Han Sans JP", "IPAGothic"],
    ),
    (
        "Malgun Gothic",
        &["Noto Sans CJK KR", "Source Han Sans KR", "NanumGothic"],
    ),
    (
        "맑은 고딕",
        &["Noto Sans CJK KR", "Source Han Sans KR", "NanumGothic"],
    ),
    (
        "Batang",
        &["Noto Serif CJK KR", "Source Han Serif KR", "NanumMyeongjo"],
    ),
    (
        "Microsoft YaHei",
        &[
            "Noto Sans CJK SC",
            "Source Han Sans SC",
            "WenQuanYi Micro Hei",
        ],
    ),
    ("微软雅黑", &["Noto Sans CJK SC", "Source Han Sans SC"]),
    ("SimSun", &["Noto Serif CJK SC", "Source Han Serif SC"]),
    ("SimHei", &["Noto Sans CJK SC", "Source Han Sans SC"]),
    (
        "Microsoft JhengHei",
        &["Noto Sans CJK TC", "Source Han Sans TC"],
    ),
    ("微軟正黑體", &["Noto Sans CJK TC", "Source Han Sans TC"]),
    ("PMingLiU", &["Noto Serif CJK TC", "Source Han Serif TC"]),
    ("MingLiU", &["Noto Serif CJK TC", "Source Han Serif TC"]),
    // Japanese sans — added per CJK Script Equality (Rust port omitted
    // these). Adobe Source Han / IPA are the typical community OSS
    // packages; pan-CJK Noto is the last-resort alias.
    (
        "Noto Sans JP",
        &[
            "Source Han Sans JP",
            "IPAGothic",
            "IPAexGothic",
            "Noto Sans CJK JP",
        ],
    ),
    (
        "Noto Sans CJK JP",
        &["Source Han Sans JP", "IPAGothic", "IPAexGothic"],
    ),
    // Japanese serif
    (
        "Noto Serif JP",
        &[
            "Source Han Serif JP",
            "IPAMincho",
            "IPAexMincho",
            "Noto Serif CJK JP",
        ],
    ),
    (
        "Noto Serif CJK JP",
        &["Source Han Serif JP", "IPAMincho", "IPAexMincho"],
    ),
    // Korean sans-serif
    (
        "Noto Sans KR",
        &[
            "Source Han Sans KR",
            "Pretendard",
            "NanumGothic",
            "NanumBarunGothic",
            "Noto Sans CJK KR",
        ],
    ),
    (
        "Noto Sans CJK KR",
        &[
            "Source Han Sans KR",
            "Pretendard",
            "NanumGothic",
            "NanumBarunGothic",
        ],
    ),
    // Korean serif
    (
        "Noto Serif KR",
        &["Source Han Serif KR", "NanumMyeongjo", "Noto Serif CJK KR"],
    ),
    (
        "Noto Serif CJK KR",
        &["Source Han Serif KR", "NanumMyeongjo"],
    ),
    // Chinese Simplified — added per CJK Script Equality.
    (
        "Noto Sans SC",
        &[
            "Source Han Sans SC",
            "WenQuanYi Zen Hei",
            "WenQuanYi Micro Hei",
            "Noto Sans CJK SC",
        ],
    ),
    (
        "Noto Sans CJK SC",
        &[
            "Source Han Sans SC",
            "WenQuanYi Zen Hei",
            "WenQuanYi Micro Hei",
        ],
    ),
    (
        "Noto Serif SC",
        &["Source Han Serif SC", "Noto Serif CJK SC"],
    ),
    ("Noto Serif CJK SC", &["Source Han Serif SC"]),
    // Chinese Traditional — added per CJK Script Equality.
    (
        "Noto Sans TC",
        &[
            "Source Han Sans TC",
            "WenQuanYi Zen Hei",
            "WenQuanYi Micro Hei",
            "Noto Sans CJK TC",
        ],
    ),
    (
        "Noto Sans CJK TC",
        &[
            "Source Han Sans TC",
            "WenQuanYi Zen Hei",
            "WenQuanYi Micro Hei",
        ],
    ),
    (
        "Noto Serif TC",
        &["Source Han Serif TC", "Noto Serif CJK TC"],
    ),
    ("Noto Serif CJK TC", &["Source Han Serif TC"]),
];

#[cfg(test)]
mod tests {
    use super::*;

    // -- Unified chain: Windows-native first, then macOS, then Linux -------

    #[test]
    fn unified_japanese_sans_starts_with_yu_gothic() {
        let chain = get_cjk_fallback_fonts(CjkPlatform::MacOs, "Noto Sans JP");
        assert_eq!(chain.first(), Some(&"Yu Gothic"));
        assert!(chain.contains(&"Hiragino Sans"));
        assert!(chain.contains(&"Source Han Sans JP"));
        assert!(chain.contains(&"Noto Sans CJK JP"));
    }

    #[test]
    fn unified_japanese_mincho_starts_with_yu_mincho() {
        let chain = get_cjk_fallback_fonts(CjkPlatform::MacOs, "Noto Serif JP");
        assert_eq!(chain.first(), Some(&"Yu Mincho"));
        assert!(chain.contains(&"Hiragino Mincho ProN"));
        assert!(chain.contains(&"Source Han Serif JP"));
    }

    #[test]
    fn unified_korean_sans_contains_native_for_every_platform() {
        let chain = get_cjk_fallback_fonts(CjkPlatform::MacOs, "Noto Sans KR");
        assert_eq!(chain.first(), Some(&"Pretendard"));
        assert!(chain.contains(&"Apple SD Gothic Neo"));
        assert!(chain.contains(&"NanumGothic"));
    }

    #[test]
    fn unified_korean_serif_contains_native_for_every_platform() {
        let chain = get_cjk_fallback_fonts(CjkPlatform::MacOs, "Noto Serif KR");
        assert_eq!(chain.first(), Some(&"Batang"));
        assert!(chain.contains(&"AppleMyungjo"));
        assert!(chain.contains(&"Source Han Serif KR"));
    }

    #[test]
    fn unified_korean_pptx_authored_includes_malgun_and_apple() {
        // PPTX-authored "맑은 고딕" — must include both Windows-native
        // (Malgun Gothic, the original PPT font) and macOS-native
        // (Apple SD Gothic Neo) so the deck renders identically on
        // either host. Office on macOS also installs Malgun Gothic, in
        // which case the WebView prefers it and matches PowerPoint.
        let chain = get_cjk_fallback_fonts(CjkPlatform::MacOs, "맑은 고딕");
        assert!(chain.contains(&"Malgun Gothic"));
        assert!(chain.contains(&"Apple SD Gothic Neo"));
    }

    #[test]
    fn unified_chinese_simplified_starts_with_microsoft_yahei() {
        let chain = get_cjk_fallback_fonts(CjkPlatform::MacOs, "Noto Sans SC");
        assert_eq!(chain.first(), Some(&"Microsoft YaHei"));
        assert!(chain.contains(&"PingFang SC"));
        assert!(chain.contains(&"Source Han Sans SC"));
    }

    #[test]
    fn unified_chinese_traditional_starts_with_microsoft_jhenghei() {
        let chain = get_cjk_fallback_fonts(CjkPlatform::MacOs, "Noto Sans TC");
        assert_eq!(chain.first(), Some(&"Microsoft JhengHei"));
        assert!(chain.contains(&"PingFang TC"));
        assert!(chain.contains(&"Source Han Sans TC"));
    }

    #[test]
    fn unified_chain_is_deduplicated() {
        // `Apple SD Gothic Neo` appears in BOTH the macOS chain (as a
        // direct preinstalled fallback) and the Linux community OSS
        // section (via the cross-OS Pretendard / NanumGothic chain).
        // The unifier dedups so each name appears at most once.
        let chain = get_cjk_fallback_fonts(CjkPlatform::MacOs, "Noto Sans KR");
        let occurrences = chain
            .iter()
            .filter(|f| **f == "Apple SD Gothic Neo")
            .count();
        assert_eq!(occurrences, 1, "duplicate entries leak through unifier");
    }

    #[test]
    fn unified_chain_ignores_platform_argument() {
        for (a, b) in [
            (CjkPlatform::MacOs, CjkPlatform::Windows),
            (CjkPlatform::MacOs, CjkPlatform::Linux),
            (CjkPlatform::Windows, CjkPlatform::Linux),
            (CjkPlatform::MacOs, CjkPlatform::Other),
        ] {
            assert_eq!(
                get_cjk_fallback_fonts(a, "Noto Sans JP"),
                get_cjk_fallback_fonts(b, "Noto Sans JP"),
            );
        }
    }

    // -- Common edge cases --------------------------------------------------

    #[test]
    fn unknown_mapping_returns_empty() {
        assert!(
            get_cjk_fallback_fonts(CjkPlatform::MacOs, "Unknown Font").is_empty(),
            "no entries should match an unknown family name"
        );
    }

    #[test]
    fn other_platform_still_returns_unified_chain() {
        // `CjkPlatform::Other` no longer short-circuits to empty —
        // the unifier ignores `platform` entirely.
        let chain = get_cjk_fallback_fonts(CjkPlatform::Other, "Noto Sans JP");
        assert!(!chain.is_empty());
        assert!(chain.contains(&"Yu Gothic"));
    }

    // -- CJK Script Equality structural assertion --------------------------

    #[test]
    fn cjk_equality_every_platform_covers_every_script() {
        // Each platform must have at least one entry for each of the
        // four CJK scripts × two styles = 8 cells. This is the
        // structural guarantee of the project's CJK Script Equality
        // rule applied to the fallback chain.
        for platform in [CjkPlatform::MacOs, CjkPlatform::Windows, CjkPlatform::Linux] {
            for cell in [
                "Noto Sans JP",
                "Noto Serif JP",
                "Noto Sans KR",
                "Noto Serif KR",
                "Noto Sans SC",
                "Noto Serif SC",
                "Noto Sans TC",
                "Noto Serif TC",
            ] {
                assert!(
                    !get_cjk_fallback_fonts(platform, cell).is_empty(),
                    "{platform:?} / {cell} chain is empty",
                );
            }
        }
    }

    // -- CjkPlatform::current resolves a non-panicking value ------------------

    #[test]
    fn current_platform_resolves() {
        // Smoke test: regardless of the host running the test, `current()`
        // must produce a CjkPlatform value (no panics, no unreachable).
        let _ = CjkPlatform::current();
    }
}
