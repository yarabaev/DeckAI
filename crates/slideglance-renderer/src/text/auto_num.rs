//! Bullet auto-numbering schemes.
//!
//! Direct port of `formatAutoNum`/`toRoman`/`toAlpha` from
//! .

use slideglance_model::AutoNumScheme;

/// Format `index` (1-based) for the given auto-numbering scheme.
#[must_use]
pub fn format_auto_num(scheme: AutoNumScheme, index: u32) -> String {
    match scheme {
        AutoNumScheme::ArabicPeriod => format!("{index}."),
        AutoNumScheme::ArabicParenR => format!("{index})"),
        AutoNumScheme::ArabicPlain => format!("{index}"),
        AutoNumScheme::RomanUcPeriod => format!("{}.", to_roman(index)),
        AutoNumScheme::RomanLcPeriod => format!("{}.", to_roman(index).to_lowercase()),
        AutoNumScheme::AlphaUcPeriod => format!("{}.", to_alpha(index)),
        AutoNumScheme::AlphaLcPeriod => format!("{}.", to_alpha(index).to_lowercase()),
        AutoNumScheme::AlphaUcParenR => format!("{})", to_alpha(index)),
        AutoNumScheme::AlphaLcParenR => format!("{})", to_alpha(index).to_lowercase()),
        AutoNumScheme::CircleNumDbPlain | AutoNumScheme::CircleNumWdWhitePlain => {
            // Unicode circled digit one (U+2460) — covers 1..=20 (①..⑳).
            // Numbers > 20 fall back to bare digits since OOXML only
            // defines circled glyphs for 1..=20.
            to_circled_num(index, false)
        }
        AutoNumScheme::CircleNumWdBlackPlain => {
            // Black-circle variants: U+2776..U+277F for 1..=10
            // (❶..❿), U+24EB..U+24F4 for 11..=20.
            to_circled_num(index, true)
        }
    }
}

fn to_circled_num(index: u32, black: bool) -> String {
    if index == 0 {
        return String::new();
    }
    let ch = if black {
        match index {
            1..=10 => char::from_u32(0x2775 + index),
            11..=20 => char::from_u32(0x24EB + (index - 11)),
            _ => None,
        }
    } else {
        match index {
            1..=20 => char::from_u32(0x245F + index),
            _ => None,
        }
    };
    ch.map_or_else(|| index.to_string(), |c| c.to_string())
}

fn to_roman(num: u32) -> String {
    const ROMAN: &[(u32, &str)] = &[
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut result = String::new();
    let mut remaining = num;
    for &(value, symbol) in ROMAN {
        while remaining >= value {
            result.push_str(symbol);
            remaining -= value;
        }
    }
    result
}

fn to_alpha(num: u32) -> String {
    let mut result = String::new();
    let mut remaining = num;
    while remaining > 0 {
        remaining -= 1;
        // Push the letter for `remaining % 26` to the front.
        let ch = char::from(b'A' + u8::try_from(remaining % 26).unwrap_or(0));
        result.insert(0, ch);
        remaining /= 26;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arabic_variants() {
        assert_eq!(format_auto_num(AutoNumScheme::ArabicPeriod, 7), "7.");
        assert_eq!(format_auto_num(AutoNumScheme::ArabicParenR, 7), "7)");
        assert_eq!(format_auto_num(AutoNumScheme::ArabicPlain, 7), "7");
    }

    #[test]
    fn roman_uppercase_lowercase() {
        assert_eq!(format_auto_num(AutoNumScheme::RomanUcPeriod, 4), "IV.");
        assert_eq!(format_auto_num(AutoNumScheme::RomanLcPeriod, 4), "iv.");
        assert_eq!(
            format_auto_num(AutoNumScheme::RomanUcPeriod, 1944),
            "MCMXLIV."
        );
    }

    #[test]
    fn alpha_uppercase_lowercase_period_paren() {
        assert_eq!(format_auto_num(AutoNumScheme::AlphaUcPeriod, 1), "A.");
        assert_eq!(format_auto_num(AutoNumScheme::AlphaLcPeriod, 1), "a.");
        assert_eq!(format_auto_num(AutoNumScheme::AlphaUcParenR, 1), "A)");
        assert_eq!(format_auto_num(AutoNumScheme::AlphaLcParenR, 1), "a)");
    }

    #[test]
    fn alpha_wraps_to_double_letters() {
        // 26 -> Z, 27 -> AA, 52 -> AZ, 53 -> BA
        assert_eq!(format_auto_num(AutoNumScheme::AlphaUcPeriod, 26), "Z.");
        assert_eq!(format_auto_num(AutoNumScheme::AlphaUcPeriod, 27), "AA.");
        assert_eq!(format_auto_num(AutoNumScheme::AlphaUcPeriod, 52), "AZ.");
        assert_eq!(format_auto_num(AutoNumScheme::AlphaUcPeriod, 53), "BA.");
    }

    #[test]
    fn roman_zero_emits_empty_string() {
        // The spec emits `""` for 0 because the loop never runs. Our
        // port should match.
        assert_eq!(format_auto_num(AutoNumScheme::RomanUcPeriod, 0), ".");
    }
}
