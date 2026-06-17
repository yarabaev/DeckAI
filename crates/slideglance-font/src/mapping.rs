// Wrapping every product / family name in `<backticks>` would litter the
// rustdoc table cells without adding clarity for OOXML font names, which
// are inherently mixed-case proper nouns rather than code identifiers.
#![allow(clippy::doc_markdown)]

//! User-overridable font name mapping.
//!
//! Maps PPTX font family names (e.g. `Calibri`, `Yu Gothic`,
//! `맑은 고딕`, `微软雅黑`, `PMingLiU`) onto a curated set of OSS
//! replacement fonts so the renderer can resolve a usable typeface
//! without a network round-trip and without leaking the original
//! Microsoft / Apple / Hanyang / Founder licensed bytes.
//!
//! ## Mapping rules (project-explicit, **not** TS parity)
//!
//! This module intentionally diverges from
//! . The TS table grew
//! organically with a Japanese-leaning bias and arbitrary inclusions /
//! omissions; we replace it with a principled table:
//!
//! ### Classification axes
//!
//! 1. **Script** — `Latin` / `Japanese (Jpan)` / `Korean (Hang)` /
//!    `Simplified Chinese (Hans)` / `Traditional Chinese (Hant)`.
//! 2. **Style** — `sans-serif` (gothic / heiti / 黑体) /
//!    `serif` (mincho / song / myeongjo / kai / 楷).
//!
//! ### Mapping targets
//!
//! - **Latin**: Microsoft Core fonts → Google Crosextra metric-clones.
//!   Each pair is **bit-exact metric-compatible** so PowerPoint layout
//!   (line breaks, frame size) survives the swap.
//!
//!   | original | target |
//!   |----------|--------|
//!   | Calibri / Calibri Light | Carlito |
//!   | Arial / Helvetica       | Arimo |
//!   | Times / Times New Roman | Tinos |
//!   | Courier / Courier New   | Cousine |
//!   | Cambria                 | Caladea |
//!
//!   Tahoma / Verdana / Georgia / Consolas etc. have **no** Crosextra
//!   metric-clone — they are intentionally **left unmapped** so the
//!   resolver chains to the system / Google Fonts lookup rather than
//!   silently substituting a metric-divergent face.
//!
//! - **CJK**: every script + style cell maps to one consistent Noto
//!   target. Eight cells, eight Noto faces.
//!
//!   | script / style       | target          |
//!   |----------------------|-----------------|
//!   | Japanese sans-serif  | `Noto Sans JP`  |
//!   | Japanese serif       | `Noto Serif JP` |
//!   | Korean sans-serif    | `Noto Sans KR`  |
//!   | Korean serif         | `Noto Serif KR` |
//!   | Simplified Chinese sans-serif | `Noto Sans SC`  |
//!   | Simplified Chinese serif      | `Noto Serif SC` |
//!   | Traditional Chinese sans-serif | `Noto Sans TC`  |
//!   | Traditional Chinese serif      | `Noto Serif TC` |
//!
//! The spec's `Noto Serif CJK JP` quirk for Japanese mincho
//! (TS keeps pan-CJK while everything else uses standalone) is
//!   removed — every CJK cell uses the language-standalone Noto file.
//!
//! ### Mapping keys (sources surveyed for each cell)
//!
//! - **Microsoft** — Office preinstalled families (Latin, Mincho,
//!   Gothic, Yu, Meiryo, Malgun, Dotum, Gulim, Batang, Gungsuh,
//!   Microsoft YaHei / JhengHei, SimSun / SimHei, MingLiU, PMingLiU,
//!   DFKai-SB, …). All localized aliases (`MS 明朝` ↔ `MS Mincho`,
//!   `맑은 고딕` ↔ `Malgun Gothic`, `微軟正黑體` ↔ `Microsoft JhengHei`).
//! - **Apple / macOS** — Hiragino (Pro / ProN / Std / StdN), Apple SD
//!   Gothic Neo, AppleGothic / AppleMyungjo, PingFang SC / TC / HK,
//!   Heiti / Songti / STHeiti / STSong / STFangsong / STKaiti.
//! - **System legacy** — Dotum / Gulim / Batang / Gungsuh + their
//!   `Che` variants and Hangul aliases.
//! - **Naver Nanum** — NanumGothic / NanumBarunGothic / NanumSquare /
//!   NanumSquareRound / NanumGothicCoding / NanumMyeongjo + Hangul
//!   aliases.
//! - **Adobe Source Han** — every language tag (J / JP / Japanese /
//!   K / KR / Korean / CN / SC / Simplified Chinese / TW / TC / HK /
//!   Traditional Chinese) for both `Source Han Sans` and
//!   `Source Han Serif`, plus Japanese / Chinese display names
//!   (`源ノ角ゴシック`, `源ノ明朝`, `思源黑体`, `思源宋体`).
//! - **Community** — Pretendard (+ JP variant routed to JP),
//!   Spoqa Han Sans, M PLUS 1p, IPA Gothic / Mincho, KoPub, 함초롬바탕
//!   / 함초롬돋움, HCR Batang / Dotum.
//! - **Hanyang (HY)** — Korean publishing-industry standard
//!   (`HY견고딕`, `HYHeadLineM`, `HY견명조`, `HYMyeongJo`, `HY궁서`).
//! - **Founder (FZ)** — Chinese publishing-industry standard
//!   (`FZHei-B01`, `FZShuSong-Z01`, 方正黑体 / 方正书宋).
//!
//! ### Pan-CJK alias normalization
//!
//! `Noto {Sans,Serif} CJK {JP,KR,SC,TC,HK}` → `Noto {Sans,Serif} {JP,KR,SC,TC}`
//! (HK collapses to TC). PowerPoint occasionally emits the pan-CJK
//! names directly; the renderer loads the language-standalone files,
//! so the alias mapping ensures resolution succeeds.
//!
//! ## Lookup semantics
//!
//! [`get_mapped_font`] resolution order (unchanged from previous
//! revisions):
//!
//! 1. Exact match on the raw input.
//! 2. Full-width → half-width normalization (`U+FF01..U+FF5E` → ASCII,
//!    `U+3000` → space), then exact match. PowerPoint themes
//!    occasionally emit `ＭＳ Ｐゴシック` instead of `MS Pゴシック`.
//! 3. ASCII-lowercase comparison after full-width normalization,
//!    against every key in the table.
//!
//! Returning `None` means "no entry" — callers chain to system /
//! Google Fonts fallback.

use std::collections::BTreeMap;

/// PPTX font name → OSS replacement font name table.
///
/// Stored as a `BTreeMap` so iteration is deterministic.
pub type FontMapping = BTreeMap<String, String>;

/// The default mapping table — single source of truth.
///
/// Both [`default_font_mapping`] and consumer test assertions read
/// from this slice, so adding / removing an entry in one place keeps
/// every consumer in sync.
#[allow(clippy::too_many_lines)] // Flat mapping data — splitting hurts grep
pub const DEFAULT_FONT_MAPPING: &[(&str, &str)] = &[
    // ============================================================
    // Latin — Microsoft Core → Google Crosextra metric-clones.
    // Each pair is bit-exact metric-compatible.
    // ============================================================
    ("Calibri", "Carlito"),
    ("Calibri Light", "Carlito"),
    ("Arial", "Arimo"),
    ("Helvetica", "Arimo"),
    ("Times", "Tinos"),
    ("Times New Roman", "Tinos"),
    ("Courier", "Cousine"),
    ("Courier New", "Cousine"),
    ("Cambria", "Caladea"),
    // ============================================================
    // Japanese — sans-serif (gothic) → Noto Sans JP
    // ============================================================
    // Microsoft
    ("MS Gothic", "Noto Sans JP"),
    ("MS ゴシック", "Noto Sans JP"),
    ("MS PGothic", "Noto Sans JP"),
    ("MS Pゴシック", "Noto Sans JP"),
    ("MS UI Gothic", "Noto Sans JP"),
    ("Meiryo", "Noto Sans JP"),
    ("Meiryo UI", "Noto Sans JP"),
    ("メイリオ", "Noto Sans JP"),
    ("Yu Gothic", "Noto Sans JP"),
    ("Yu Gothic UI", "Noto Sans JP"),
    ("Yu Gothic Medium", "Noto Sans JP"),
    ("游ゴシック", "Noto Sans JP"),
    ("游ゴシック Medium", "Noto Sans JP"),
    ("HG創英角ｺﾞｼｯｸUB", "Noto Sans JP"),
    ("HGS創英角ｺﾞｼｯｸUB", "Noto Sans JP"),
    ("HGP創英角ｺﾞｼｯｸUB", "Noto Sans JP"),
    ("HG丸ｺﾞｼｯｸM-PRO", "Noto Sans JP"),
    // Apple / macOS
    ("Hiragino Sans", "Noto Sans JP"),
    ("Hiragino Kaku Gothic Pro", "Noto Sans JP"),
    ("Hiragino Kaku Gothic ProN", "Noto Sans JP"),
    ("Hiragino Kaku Gothic Std", "Noto Sans JP"),
    ("Hiragino Kaku Gothic StdN", "Noto Sans JP"),
    ("Hiragino Maru Gothic Pro", "Noto Sans JP"),
    ("Hiragino Maru Gothic ProN", "Noto Sans JP"),
    ("ヒラギノ角ゴ Pro", "Noto Sans JP"),
    ("ヒラギノ角ゴ ProN", "Noto Sans JP"),
    ("ヒラギノ丸ゴ Pro", "Noto Sans JP"),
    ("ヒラギノ丸ゴ ProN", "Noto Sans JP"),
    // Adobe Source Han / community
    ("Source Han Sans", "Noto Sans JP"),
    ("Source Han Sans J", "Noto Sans JP"),
    ("Source Han Sans JP", "Noto Sans JP"),
    ("Source Han Sans Japanese", "Noto Sans JP"),
    ("源ノ角ゴシック", "Noto Sans JP"),
    ("源ノ角ゴシック JP", "Noto Sans JP"),
    ("IPAGothic", "Noto Sans JP"),
    ("IPA Gothic", "Noto Sans JP"),
    ("IPAPGothic", "Noto Sans JP"),
    ("IPAexGothic", "Noto Sans JP"),
    ("M PLUS 1p", "Noto Sans JP"),
    ("M PLUS Rounded 1c", "Noto Sans JP"),
    ("Pretendard JP", "Noto Sans JP"),
    // ============================================================
    // Japanese — serif (mincho) → Noto Serif JP
    // ============================================================
    // Microsoft
    ("MS Mincho", "Noto Serif JP"),
    ("MS 明朝", "Noto Serif JP"),
    ("MS PMincho", "Noto Serif JP"),
    ("MS P明朝", "Noto Serif JP"),
    ("Yu Mincho", "Noto Serif JP"),
    ("Yu Mincho Light", "Noto Serif JP"),
    ("Yu Mincho Demibold", "Noto Serif JP"),
    ("游明朝", "Noto Serif JP"),
    ("游明朝 Light", "Noto Serif JP"),
    ("游明朝 Demibold", "Noto Serif JP"),
    ("HG明朝B", "Noto Serif JP"),
    ("HGS明朝B", "Noto Serif JP"),
    ("HGP明朝B", "Noto Serif JP"),
    ("HG明朝E", "Noto Serif JP"),
    // Apple / macOS
    ("Hiragino Mincho Pro", "Noto Serif JP"),
    ("Hiragino Mincho ProN", "Noto Serif JP"),
    ("Hiragino Mincho Std", "Noto Serif JP"),
    ("Hiragino Mincho StdN", "Noto Serif JP"),
    ("ヒラギノ明朝 Pro", "Noto Serif JP"),
    ("ヒラギノ明朝 ProN", "Noto Serif JP"),
    // Adobe Source Han / community
    ("Source Han Serif", "Noto Serif JP"),
    ("Source Han Serif J", "Noto Serif JP"),
    ("Source Han Serif JP", "Noto Serif JP"),
    ("Source Han Serif Japanese", "Noto Serif JP"),
    ("源ノ明朝", "Noto Serif JP"),
    ("源ノ明朝 JP", "Noto Serif JP"),
    ("IPAMincho", "Noto Serif JP"),
    ("IPA Mincho", "Noto Serif JP"),
    ("IPAPMincho", "Noto Serif JP"),
    ("IPAexMincho", "Noto Serif JP"),
    // ============================================================
    // Korean — sans-serif (gothic) → Noto Sans KR
    // ============================================================
    // Microsoft
    ("Malgun Gothic", "Noto Sans KR"),
    ("MalgunGothic", "Noto Sans KR"),
    ("맑은 고딕", "Noto Sans KR"),
    ("맑은고딕", "Noto Sans KR"),
    // Apple / macOS
    ("Apple SD Gothic Neo", "Noto Sans KR"),
    ("AppleSDGothicNeo", "Noto Sans KR"),
    ("Apple Gothic", "Noto Sans KR"),
    ("AppleGothic", "Noto Sans KR"),
    ("애플고딕", "Noto Sans KR"),
    // System legacy
    ("Dotum", "Noto Sans KR"),
    ("DotumChe", "Noto Sans KR"),
    ("돋움", "Noto Sans KR"),
    ("돋움체", "Noto Sans KR"),
    ("Gulim", "Noto Sans KR"),
    ("GulimChe", "Noto Sans KR"),
    ("굴림", "Noto Sans KR"),
    ("굴림체", "Noto Sans KR"),
    // Naver Nanum
    ("NanumGothic", "Noto Sans KR"),
    ("Nanum Gothic", "Noto Sans KR"),
    ("나눔고딕", "Noto Sans KR"),
    ("NanumBarunGothic", "Noto Sans KR"),
    ("Nanum Barun Gothic", "Noto Sans KR"),
    ("나눔바른고딕", "Noto Sans KR"),
    ("NanumSquare", "Noto Sans KR"),
    ("Nanum Square", "Noto Sans KR"),
    ("나눔스퀘어", "Noto Sans KR"),
    ("NanumSquareRound", "Noto Sans KR"),
    ("나눔스퀘어라운드", "Noto Sans KR"),
    ("NanumGothicCoding", "Noto Sans KR"),
    ("나눔고딕코딩", "Noto Sans KR"),
    // Adobe Source Han / community
    ("Source Han Sans K", "Noto Sans KR"),
    ("Source Han Sans KR", "Noto Sans KR"),
    ("Source Han Sans Korean", "Noto Sans KR"),
    ("본고딕", "Noto Sans KR"),
    ("Pretendard", "Noto Sans KR"),
    ("Pretendard Variable", "Noto Sans KR"),
    ("Spoqa Han Sans", "Noto Sans KR"),
    ("Spoqa Han Sans Neo", "Noto Sans KR"),
    ("KoPub Dotum", "Noto Sans KR"),
    ("KoPubDotum", "Noto Sans KR"),
    ("KoPubDotum_Pro", "Noto Sans KR"),
    ("KoPub돋움체", "Noto Sans KR"),
    ("함초롬돋움", "Noto Sans KR"),
    ("HCR Dotum", "Noto Sans KR"),
    // Hanyang (HY) — gothic
    ("HY견고딕", "Noto Sans KR"),
    ("HYGothic", "Noto Sans KR"),
    ("HY GoThic", "Noto Sans KR"),
    ("HY중고딕", "Noto Sans KR"),
    ("HY헤드라인M", "Noto Sans KR"),
    ("HYHeadLineM", "Noto Sans KR"),
    ("HY그래픽M", "Noto Sans KR"),
    ("HYGraphic-Medium", "Noto Sans KR"),
    ("HY얕은샘물M", "Noto Sans KR"),
    ("HYShortSamulM", "Noto Sans KR"),
    // ============================================================
    // Korean — serif (myeongjo / batang) → Noto Serif KR
    // ============================================================
    // Microsoft / system
    ("Batang", "Noto Serif KR"),
    ("BatangChe", "Noto Serif KR"),
    ("바탕", "Noto Serif KR"),
    ("바탕체", "Noto Serif KR"),
    ("Gungsuh", "Noto Serif KR"),
    ("GungsuhChe", "Noto Serif KR"),
    ("궁서", "Noto Serif KR"),
    ("궁서체", "Noto Serif KR"),
    // Apple / macOS
    ("AppleMyungjo", "Noto Serif KR"),
    ("Apple Myungjo", "Noto Serif KR"),
    ("애플명조", "Noto Serif KR"),
    // Naver Nanum
    ("NanumMyeongjo", "Noto Serif KR"),
    ("Nanum Myeongjo", "Noto Serif KR"),
    ("나눔명조", "Noto Serif KR"),
    // Adobe Source Han / community
    ("Source Han Serif K", "Noto Serif KR"),
    ("Source Han Serif KR", "Noto Serif KR"),
    ("Source Han Serif Korean", "Noto Serif KR"),
    ("본명조", "Noto Serif KR"),
    ("KoPub Batang", "Noto Serif KR"),
    ("KoPubBatang", "Noto Serif KR"),
    ("KoPubBatang_Pro", "Noto Serif KR"),
    ("KoPub바탕체", "Noto Serif KR"),
    ("함초롬바탕", "Noto Serif KR"),
    ("HCR Batang", "Noto Serif KR"),
    // Hanyang (HY) — myeongjo / kai
    ("HY견명조", "Noto Serif KR"),
    ("HYMyeongJo", "Noto Serif KR"),
    ("HY신명조", "Noto Serif KR"),
    ("HY궁서", "Noto Serif KR"),
    ("HYGungSo-Bold", "Noto Serif KR"),
    // ============================================================
    // Simplified Chinese — sans-serif (heiti / 黑体) → Noto Sans SC
    // ============================================================
    // Microsoft
    ("Microsoft YaHei", "Noto Sans SC"),
    ("Microsoft YaHei UI", "Noto Sans SC"),
    ("微软雅黑", "Noto Sans SC"),
    ("SimHei", "Noto Sans SC"),
    ("黑体", "Noto Sans SC"),
    ("DengXian", "Noto Sans SC"),
    ("DengXian Light", "Noto Sans SC"),
    ("等线", "Noto Sans SC"),
    // Apple / macOS
    ("PingFang SC", "Noto Sans SC"),
    ("苹方-简", "Noto Sans SC"),
    ("Heiti SC", "Noto Sans SC"),
    ("Heiti SC Light", "Noto Sans SC"),
    ("Heiti SC Medium", "Noto Sans SC"),
    ("STHeiti", "Noto Sans SC"),
    ("STHeiti Light", "Noto Sans SC"),
    ("华文黑体", "Noto Sans SC"),
    ("STXihei", "Noto Sans SC"),
    ("华文细黑", "Noto Sans SC"),
    // Founder (Fangzheng)
    ("FZHei-B01", "Noto Sans SC"),
    ("FZLanTingHei-S-DB", "Noto Sans SC"),
    ("方正黑体", "Noto Sans SC"),
    ("方正兰亭黑", "Noto Sans SC"),
    // Adobe Source Han / community
    ("Source Han Sans SC", "Noto Sans SC"),
    ("Source Han Sans CN", "Noto Sans SC"),
    ("Source Han Sans Simplified Chinese", "Noto Sans SC"),
    ("思源黑体", "Noto Sans SC"),
    ("思源黑体 CN", "Noto Sans SC"),
    // ============================================================
    // Simplified Chinese — serif (Song / Kai) → Noto Serif SC
    // ============================================================
    // Microsoft
    ("SimSun", "Noto Serif SC"),
    ("宋体", "Noto Serif SC"),
    ("NSimSun", "Noto Serif SC"),
    ("新宋体", "Noto Serif SC"),
    ("FangSong", "Noto Serif SC"),
    ("仿宋", "Noto Serif SC"),
    ("STFangsong", "Noto Serif SC"),
    ("华文仿宋", "Noto Serif SC"),
    ("KaiTi", "Noto Serif SC"),
    ("楷体", "Noto Serif SC"),
    ("STKaiti", "Noto Serif SC"),
    ("华文楷体", "Noto Serif SC"),
    // Apple / macOS
    ("Songti SC", "Noto Serif SC"),
    ("宋体-简", "Noto Serif SC"),
    ("STSong", "Noto Serif SC"),
    ("华文宋体", "Noto Serif SC"),
    // Founder
    ("FZShuSong-Z01", "Noto Serif SC"),
    ("方正书宋", "Noto Serif SC"),
    // Adobe Source Han / community
    ("Source Han Serif SC", "Noto Serif SC"),
    ("Source Han Serif CN", "Noto Serif SC"),
    ("Source Han Serif Simplified Chinese", "Noto Serif SC"),
    ("思源宋体", "Noto Serif SC"),
    ("思源宋体 CN", "Noto Serif SC"),
    // ============================================================
    // Traditional Chinese — sans-serif → Noto Sans TC
    // ============================================================
    // Microsoft
    ("Microsoft JhengHei", "Noto Sans TC"),
    ("Microsoft JhengHei UI", "Noto Sans TC"),
    ("微軟正黑體", "Noto Sans TC"),
    // Apple / macOS
    ("PingFang TC", "Noto Sans TC"),
    ("PingFang HK", "Noto Sans TC"),
    ("苹方-繁", "Noto Sans TC"),
    ("蘋方-繁", "Noto Sans TC"),
    ("Heiti TC", "Noto Sans TC"),
    ("Heiti TC Light", "Noto Sans TC"),
    ("Heiti TC Medium", "Noto Sans TC"),
    // Adobe Source Han / community
    ("Source Han Sans TC", "Noto Sans TC"),
    ("Source Han Sans TW", "Noto Sans TC"),
    ("Source Han Sans HK", "Noto Sans TC"),
    ("Source Han Sans Traditional Chinese", "Noto Sans TC"),
    ("思源黑體", "Noto Sans TC"),
    ("思源黑體 TC", "Noto Sans TC"),
    // ============================================================
    // Traditional Chinese — serif (Ming / Kai) → Noto Serif TC
    // ============================================================
    // Microsoft
    ("PMingLiU", "Noto Serif TC"),
    ("新細明體", "Noto Serif TC"),
    ("MingLiU", "Noto Serif TC"),
    ("細明體", "Noto Serif TC"),
    ("MingLiU_HKSCS", "Noto Serif TC"),
    ("PMingLiU-ExtB", "Noto Serif TC"),
    ("MingLiU-ExtB", "Noto Serif TC"),
    ("DFKai-SB", "Noto Serif TC"),
    ("標楷體", "Noto Serif TC"),
    // Apple / macOS
    ("Songti TC", "Noto Serif TC"),
    ("宋體-繁", "Noto Serif TC"),
    ("LiSong Pro", "Noto Serif TC"),
    ("儷宋 Pro", "Noto Serif TC"),
    ("BiauKai", "Noto Serif TC"),
    ("標楷體-繁", "Noto Serif TC"),
    // Adobe Source Han / community
    ("Source Han Serif TC", "Noto Serif TC"),
    ("Source Han Serif TW", "Noto Serif TC"),
    ("Source Han Serif HK", "Noto Serif TC"),
    ("Source Han Serif Traditional Chinese", "Noto Serif TC"),
    ("思源宋體", "Noto Serif TC"),
    ("思源宋體 TC", "Noto Serif TC"),
    // ============================================================
    // Pan-CJK alias normalization. PowerPoint emits the CJK names
    // when a theme references the pan-CJK Adobe / Google file; we
    // load the language-standalone files, so collapse the alias.
    // HK collapses to TC (Noto has no separate HK file).
    // ============================================================
    ("Noto Sans CJK JP", "Noto Sans JP"),
    ("Noto Serif CJK JP", "Noto Serif JP"),
    ("Noto Sans CJK KR", "Noto Sans KR"),
    ("Noto Serif CJK KR", "Noto Serif KR"),
    ("Noto Sans CJK SC", "Noto Sans SC"),
    ("Noto Serif CJK SC", "Noto Serif SC"),
    ("Noto Sans CJK TC", "Noto Sans TC"),
    ("Noto Serif CJK TC", "Noto Serif TC"),
    ("Noto Sans CJK HK", "Noto Sans TC"),
    ("Noto Serif CJK HK", "Noto Serif TC"),
];

/// Builds the default mapping table.
///
/// Single source of truth is [`DEFAULT_FONT_MAPPING`]; this function
/// just materializes it into a `BTreeMap` so callers can pass it
/// straight to [`get_mapped_font`] or merge user overrides.
#[must_use]
pub fn default_font_mapping() -> FontMapping {
    DEFAULT_FONT_MAPPING
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

/// Merges user-supplied mappings on top of the default table.
///
/// User entries with the same key override the default. With `None` /
/// empty, returns a fresh copy of [`default_font_mapping`].
#[must_use]
pub fn create_font_mapping<I>(user: Option<I>) -> FontMapping
where
    I: IntoIterator<Item = (String, String)>,
{
    let mut mapping = default_font_mapping();
    if let Some(entries) = user {
        for (k, v) in entries {
            mapping.insert(k, v);
        }
    }
    mapping
}

/// Resolves a PPTX font name through the mapping table.
///
/// Returns `None` when no entry matches; callers chain to system /
/// Google Fonts fallback. Lookup order:
///
/// 1. Exact match against the raw input.
/// 2. Full-width → half-width normalization, then exact match.
/// 3. ASCII-lowercase comparison after full-width normalization,
///    against every key in the table.
#[must_use]
pub fn get_mapped_font(font_family: &str, mapping: &FontMapping) -> Option<String> {
    if font_family.is_empty() {
        return None;
    }

    // 1. Exact match on raw input.
    if let Some(value) = mapping.get(font_family) {
        return Some(value.clone());
    }

    // 2. Full-width normalization, then exact match.
    let normalized = normalize_full_width(font_family);
    if normalized != font_family {
        if let Some(value) = mapping.get(&normalized) {
            return Some(value.clone());
        }
    }

    // 3. Case-insensitive scan after full-width normalization on both sides.
    let lower = ascii_lowercase(&normalized);
    for (key, value) in mapping {
        if ascii_lowercase(&normalize_full_width(key)) == lower {
            return Some(value.clone());
        }
    }

    None
}

/// Normalizes full-width ASCII (`U+FF01..U+FF5E`) and ideographic space
/// (`U+3000`) to their half-width counterparts.
///
/// Korean and Japanese theme XML occasionally emits full-width Latin
/// names (e.g. `ＭＳ Ｐゴシック`) which would otherwise fail an
/// exact-match lookup against the half-width entries in the default
/// table.
fn normalize_full_width(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        let cp = ch as u32;
        if (0xFF01..=0xFF5E).contains(&cp) {
            // Safe because the range maps to ASCII printable (0x21..0x7E).
            if let Some(mapped) = char::from_u32(cp - 0xFEE0) {
                out.push(mapped);
                continue;
            }
        }
        if cp == 0x3000 {
            out.push(' ');
            continue;
        }
        out.push(ch);
    }
    out
}

/// ASCII-only `to_lowercase`.
///
/// Locale-independent: only `A..Z` are folded. CJK characters pass
/// through unchanged (they have no case to fold).
fn ascii_lowercase(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch.is_ascii_uppercase() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_maps_to(name: &str, expected: &str) {
        let m = default_font_mapping();
        assert_eq!(
            m.get(name).map(String::as_str),
            Some(expected),
            "{name} should map to {expected}"
        );
    }

    // -- Latin (Microsoft Core → Google Crosextra) ----------------------------

    #[test]
    fn latin_metric_clones() {
        assert_maps_to("Calibri", "Carlito");
        assert_maps_to("Calibri Light", "Carlito");
        assert_maps_to("Arial", "Arimo");
        assert_maps_to("Helvetica", "Arimo");
        assert_maps_to("Times", "Tinos");
        assert_maps_to("Times New Roman", "Tinos");
        assert_maps_to("Courier", "Cousine");
        assert_maps_to("Courier New", "Cousine");
        assert_maps_to("Cambria", "Caladea");
    }

    #[test]
    fn latin_unmapped_fonts_return_none() {
        let m = default_font_mapping();
        // No Crosextra metric-clone exists — leave unmapped so the
        // resolver chains to system / Google fallback.
        for unmapped in ["Tahoma", "Verdana", "Georgia", "Consolas", "Trebuchet MS"] {
            assert!(!m.contains_key(unmapped), "{unmapped} should NOT be mapped");
        }
    }

    // -- Japanese sans-serif → Noto Sans JP -----------------------------------

    #[test]
    fn japanese_gothic_microsoft() {
        for name in [
            "MS Gothic",
            "MS ゴシック",
            "MS PGothic",
            "MS Pゴシック",
            "MS UI Gothic",
            "Meiryo",
            "Meiryo UI",
            "メイリオ",
            "Yu Gothic",
            "Yu Gothic UI",
            "游ゴシック",
        ] {
            assert_maps_to(name, "Noto Sans JP");
        }
    }

    #[test]
    fn japanese_gothic_apple() {
        for name in [
            "Hiragino Sans",
            "Hiragino Kaku Gothic Pro",
            "Hiragino Kaku Gothic ProN",
            "Hiragino Maru Gothic ProN",
            "ヒラギノ角ゴ ProN",
        ] {
            assert_maps_to(name, "Noto Sans JP");
        }
    }

    #[test]
    fn japanese_gothic_source_han_and_community() {
        for name in [
            "Source Han Sans",
            "Source Han Sans JP",
            "Source Han Sans Japanese",
            "源ノ角ゴシック",
            "IPAGothic",
            "M PLUS 1p",
            "Pretendard JP",
        ] {
            assert_maps_to(name, "Noto Sans JP");
        }
    }

    // -- Japanese serif → Noto Serif JP ---------------------------------------

    #[test]
    fn japanese_mincho_microsoft() {
        for name in [
            "MS Mincho",
            "MS 明朝",
            "MS PMincho",
            "MS P明朝",
            "Yu Mincho",
            "游明朝",
        ] {
            assert_maps_to(name, "Noto Serif JP");
        }
    }

    #[test]
    fn japanese_mincho_apple_and_source_han() {
        for name in [
            "Hiragino Mincho Pro",
            "Hiragino Mincho ProN",
            "ヒラギノ明朝 ProN",
            "Source Han Serif",
            "Source Han Serif JP",
            "Source Han Serif Japanese",
            "源ノ明朝",
            "IPAMincho",
        ] {
            assert_maps_to(name, "Noto Serif JP");
        }
    }

    // -- Korean sans-serif → Noto Sans KR -------------------------------------

    #[test]
    fn korean_gothic_microsoft_and_apple() {
        for name in [
            "Malgun Gothic",
            "맑은 고딕",
            "맑은고딕",
            "Apple SD Gothic Neo",
            "AppleGothic",
            "애플고딕",
        ] {
            assert_maps_to(name, "Noto Sans KR");
        }
    }

    #[test]
    fn korean_gothic_legacy_system() {
        for name in [
            "Dotum",
            "DotumChe",
            "돋움",
            "돋움체",
            "Gulim",
            "GulimChe",
            "굴림",
            "굴림체",
        ] {
            assert_maps_to(name, "Noto Sans KR");
        }
    }

    #[test]
    fn korean_gothic_nanum_and_community() {
        for name in [
            "NanumGothic",
            "나눔고딕",
            "NanumBarunGothic",
            "NanumSquare",
            "Pretendard",
            "Pretendard Variable",
            "Spoqa Han Sans",
            "Spoqa Han Sans Neo",
            "본고딕",
            "Source Han Sans K",
            "Source Han Sans Korean",
            "함초롬돋움",
            "KoPubDotum",
        ] {
            assert_maps_to(name, "Noto Sans KR");
        }
    }

    #[test]
    fn korean_gothic_hanyang() {
        for name in [
            "HY견고딕",
            "HYGothic",
            "HY중고딕",
            "HY헤드라인M",
            "HYHeadLineM",
            "HY그래픽M",
        ] {
            assert_maps_to(name, "Noto Sans KR");
        }
    }

    // -- Korean serif → Noto Serif KR -----------------------------------------

    #[test]
    fn korean_serif_system_and_apple() {
        for name in [
            "Batang",
            "BatangChe",
            "바탕",
            "Gungsuh",
            "궁서",
            "AppleMyungjo",
            "애플명조",
        ] {
            assert_maps_to(name, "Noto Serif KR");
        }
    }

    #[test]
    fn korean_serif_nanum_and_community() {
        for name in [
            "NanumMyeongjo",
            "나눔명조",
            "본명조",
            "Source Han Serif K",
            "Source Han Serif Korean",
            "함초롬바탕",
            "KoPubBatang",
        ] {
            assert_maps_to(name, "Noto Serif KR");
        }
    }

    #[test]
    fn korean_serif_hanyang() {
        for name in ["HY견명조", "HYMyeongJo", "HY신명조", "HY궁서"] {
            assert_maps_to(name, "Noto Serif KR");
        }
    }

    // -- Simplified Chinese sans-serif → Noto Sans SC -------------------------

    #[test]
    fn simplified_chinese_gothic_microsoft() {
        for name in [
            "Microsoft YaHei",
            "Microsoft YaHei UI",
            "微软雅黑",
            "SimHei",
            "黑体",
            "DengXian",
            "等线",
        ] {
            assert_maps_to(name, "Noto Sans SC");
        }
    }

    #[test]
    fn simplified_chinese_gothic_apple_founder_source_han() {
        for name in [
            "PingFang SC",
            "苹方-简",
            "Heiti SC",
            "STHeiti",
            "华文黑体",
            "FZHei-B01",
            "方正黑体",
            "Source Han Sans SC",
            "Source Han Sans CN",
            "思源黑体",
        ] {
            assert_maps_to(name, "Noto Sans SC");
        }
    }

    // -- Simplified Chinese serif → Noto Serif SC -----------------------------

    #[test]
    fn simplified_chinese_serif_microsoft() {
        for name in [
            "SimSun",
            "宋体",
            "NSimSun",
            "新宋体",
            "FangSong",
            "仿宋",
            "KaiTi",
            "楷体",
        ] {
            assert_maps_to(name, "Noto Serif SC");
        }
    }

    #[test]
    fn simplified_chinese_serif_apple_source_han() {
        for name in [
            "Songti SC",
            "STSong",
            "华文宋体",
            "Source Han Serif SC",
            "Source Han Serif CN",
            "思源宋体",
            "方正书宋",
        ] {
            assert_maps_to(name, "Noto Serif SC");
        }
    }

    // -- Traditional Chinese sans-serif → Noto Sans TC ------------------------

    #[test]
    fn traditional_chinese_gothic() {
        for name in [
            "Microsoft JhengHei",
            "Microsoft JhengHei UI",
            "微軟正黑體",
            "PingFang TC",
            "PingFang HK",
            "苹方-繁",
            "Heiti TC",
            "Source Han Sans TC",
            "Source Han Sans TW",
            "Source Han Sans HK",
            "思源黑體",
        ] {
            assert_maps_to(name, "Noto Sans TC");
        }
    }

    // -- Traditional Chinese serif → Noto Serif TC ----------------------------

    #[test]
    fn traditional_chinese_serif() {
        for name in [
            "PMingLiU",
            "新細明體",
            "MingLiU",
            "細明體",
            "DFKai-SB",
            "標楷體",
            "Songti TC",
            "LiSong Pro",
            "BiauKai",
            "Source Han Serif TC",
            "Source Han Serif TW",
            "Source Han Serif HK",
            "思源宋體",
        ] {
            assert_maps_to(name, "Noto Serif TC");
        }
    }

    // -- Pan-CJK alias normalization ------------------------------------------

    #[test]
    fn pan_cjk_aliases_collapse_to_standalone() {
        assert_maps_to("Noto Sans CJK JP", "Noto Sans JP");
        assert_maps_to("Noto Serif CJK JP", "Noto Serif JP");
        assert_maps_to("Noto Sans CJK KR", "Noto Sans KR");
        assert_maps_to("Noto Serif CJK KR", "Noto Serif KR");
        assert_maps_to("Noto Sans CJK SC", "Noto Sans SC");
        assert_maps_to("Noto Serif CJK SC", "Noto Serif SC");
        assert_maps_to("Noto Sans CJK TC", "Noto Sans TC");
        assert_maps_to("Noto Serif CJK TC", "Noto Serif TC");
        // HK collapses to TC.
        assert_maps_to("Noto Sans CJK HK", "Noto Sans TC");
        assert_maps_to("Noto Serif CJK HK", "Noto Serif TC");
    }

    // -- CJK Script Equality structural check ---------------------------------

    #[test]
    fn cjk_script_equality_all_eight_cells_present() {
        // Every (script × style) cell has at least one entry whose
        // value is the corresponding Noto target. This is the
        // structural assertion behind the "CJK Script Equality" rule:
        // no script gets dropped at the mapping layer.
        let m = default_font_mapping();
        let cells: &[(&str, &str)] = &[
            ("Yu Gothic", "Noto Sans JP"),
            ("Yu Mincho", "Noto Serif JP"),
            ("Malgun Gothic", "Noto Sans KR"),
            ("Batang", "Noto Serif KR"),
            ("Microsoft YaHei", "Noto Sans SC"),
            ("SimSun", "Noto Serif SC"),
            ("Microsoft JhengHei", "Noto Sans TC"),
            ("PMingLiU", "Noto Serif TC"),
        ];
        for (sample_key, expected) in cells {
            assert_eq!(
                m.get(*sample_key).map(String::as_str),
                Some(*expected),
                "missing CJK cell coverage for {sample_key} → {expected}"
            );
        }
    }

    // -- create_font_mapping --------------------------------------------------

    #[test]
    fn create_font_mapping_with_no_user_returns_default_copy() {
        let m = create_font_mapping::<Vec<(String, String)>>(None);
        assert_eq!(m.get("Calibri").map(String::as_str), Some("Carlito"));
        assert_eq!(m.get("Arial").map(String::as_str), Some("Arimo"));
    }

    #[test]
    fn create_font_mapping_user_overrides_default() {
        let user = vec![("Calibri".to_string(), "Custom Font".to_string())];
        let m = create_font_mapping(Some(user));
        assert_eq!(m.get("Calibri").map(String::as_str), Some("Custom Font"));
        assert_eq!(m.get("Arial").map(String::as_str), Some("Arimo"));
    }

    #[test]
    fn create_font_mapping_user_can_add_new_entries() {
        let user = vec![("My Custom Font".to_string(), "Noto Sans".to_string())];
        let m = create_font_mapping(Some(user));
        assert_eq!(
            m.get("My Custom Font").map(String::as_str),
            Some("Noto Sans")
        );
        assert_eq!(m.get("Calibri").map(String::as_str), Some("Carlito"));
    }

    // -- get_mapped_font lookup semantics -------------------------------------

    #[test]
    fn get_mapped_font_exact_match() {
        let m: FontMapping = [
            ("Calibri".to_string(), "Carlito".to_string()),
            ("MS Gothic".to_string(), "Noto Sans JP".to_string()),
        ]
        .into_iter()
        .collect();
        assert_eq!(get_mapped_font("Calibri", &m), Some("Carlito".to_string()));
    }

    #[test]
    fn get_mapped_font_case_insensitive() {
        let m: FontMapping = [("Calibri".to_string(), "Carlito".to_string())]
            .into_iter()
            .collect();
        assert_eq!(get_mapped_font("calibri", &m), Some("Carlito".to_string()));
        assert_eq!(get_mapped_font("CALIBRI", &m), Some("Carlito".to_string()));
    }

    #[test]
    fn get_mapped_font_unknown_returns_none() {
        let m: FontMapping = [("Calibri".to_string(), "Carlito".to_string())]
            .into_iter()
            .collect();
        assert_eq!(get_mapped_font("Unknown Font", &m), None);
    }

    #[test]
    fn get_mapped_font_empty_returns_none() {
        let m = default_font_mapping();
        assert_eq!(get_mapped_font("", &m), None);
    }

    #[test]
    fn get_mapped_font_full_width_normalization() {
        let m = default_font_mapping();
        // ＭＳ Ｐゴシック (full-width 'P') → MS Pゴシック.
        assert_eq!(
            get_mapped_font("ＭＳ Ｐゴシック", &m),
            Some("Noto Sans JP".to_string())
        );
        // ＭＳ Ｐ明朝 (full-width 'P') → MS P明朝 → Noto Serif JP
        // (the JP-specific standalone, not the TS-quirk pan-CJK).
        assert_eq!(
            get_mapped_font("ＭＳ Ｐ明朝", &m),
            Some("Noto Serif JP".to_string())
        );
        // Ideographic space U+3000 → ASCII space.
        assert_eq!(
            get_mapped_font("ＭＳ\u{3000}Ｐゴシック", &m),
            Some("Noto Sans JP".to_string())
        );
    }

    #[test]
    fn normalize_full_width_basic() {
        assert_eq!(normalize_full_width("ＡＢＣ"), "ABC");
        assert_eq!(normalize_full_width("Ｐ"), "P");
        assert_eq!(normalize_full_width("\u{3000}"), " ");
        assert_eq!(normalize_full_width("メイリオ"), "メイリオ");
        assert_eq!(normalize_full_width("ABC"), "ABC");
    }

    // -- Single source of truth ----------------------------------------------

    #[test]
    fn default_table_has_no_duplicate_keys() {
        // BTreeMap dedups silently — verify the const's list itself
        // has no duplicate keys, which would indicate an editing
        // mistake (later entry silently overrides earlier).
        let mut seen = std::collections::HashSet::new();
        for (k, _) in DEFAULT_FONT_MAPPING {
            assert!(seen.insert(*k), "duplicate key in const: {k}");
        }
    }

    #[test]
    fn default_table_runtime_matches_const_size() {
        // Sanity: BTreeMap has the same number of entries as the const
        // (since we just verified no dup keys, runtime size must match).
        assert_eq!(default_font_mapping().len(), DEFAULT_FONT_MAPPING.len());
    }
}
