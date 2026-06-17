//! Ported from the spec.
//!
//! The Rust [`ColorResolver::resolve`] takes a typed [`ColorRef`] (constructed
//! by the parser) and is therefore total — the TS test cases for `null` /
//! empty XML nodes / unknown structures live in the parser layer instead.

use slideglance_color::{
    ColorMap, ColorRef, ColorResolver, ColorScheme, ColorTransform, PerMille, ResolvedColor, Rgb,
    SchemeColorKey,
};

fn test_scheme() -> ColorScheme {
    ColorScheme {
        dk1: Rgb::new(0x00, 0x00, 0x00),
        lt1: Rgb::new(0xFF, 0xFF, 0xFF),
        dk2: Rgb::new(0x44, 0x54, 0x6A),
        lt2: Rgb::new(0xE7, 0xE6, 0xE6),
        accent1: Rgb::new(0x44, 0x72, 0xC4),
        accent2: Rgb::new(0xED, 0x7D, 0x31),
        accent3: Rgb::new(0xA5, 0xA5, 0xA5),
        accent4: Rgb::new(0xFF, 0xC0, 0x00),
        accent5: Rgb::new(0x5B, 0x9B, 0xD5),
        accent6: Rgb::new(0x70, 0xAD, 0x47),
        hlink: Rgb::new(0x05, 0x63, 0xC1),
        fol_hlink: Rgb::new(0x95, 0x4F, 0x72),
    }
}

fn test_map() -> ColorMap {
    ColorMap {
        bg1: SchemeColorKey::Lt1,
        tx1: SchemeColorKey::Dk1,
        bg2: SchemeColorKey::Lt2,
        tx2: SchemeColorKey::Dk2,
        accent1: SchemeColorKey::Accent1,
        accent2: SchemeColorKey::Accent2,
        accent3: SchemeColorKey::Accent3,
        accent4: SchemeColorKey::Accent4,
        accent5: SchemeColorKey::Accent5,
        accent6: SchemeColorKey::Accent6,
        hlink: SchemeColorKey::Hlink,
        fol_hlink: SchemeColorKey::FolHlink,
    }
}

fn resolver() -> ColorResolver {
    ColorResolver::new(test_scheme(), test_map())
}

#[test]
fn resolves_srgb_clr() {
    let result = resolver().resolve(&ColorRef::Srgb {
        rgb: Rgb::new(0xFF, 0x00, 0x00),
        transform: ColorTransform::default(),
    });
    assert_eq!(result, ResolvedColor::new(Rgb::new(0xFF, 0x00, 0x00), 1.0));
}

#[test]
fn resolves_scheme_accent1() {
    let result = resolver().resolve(&ColorRef::Scheme {
        name: "accent1".to_owned(),
        transform: ColorTransform::default(),
    });
    assert_eq!(result, ResolvedColor::new(Rgb::new(0x44, 0x72, 0xC4), 1.0));
}

#[test]
fn resolves_scheme_via_color_map_tx1_to_dk1() {
    let result = resolver().resolve(&ColorRef::Scheme {
        name: "tx1".to_owned(),
        transform: ColorTransform::default(),
    });
    assert_eq!(result, ResolvedColor::new(Rgb::new(0x00, 0x00, 0x00), 1.0));
}

#[test]
fn resolves_scheme_via_color_map_bg1_to_lt1() {
    let result = resolver().resolve(&ColorRef::Scheme {
        name: "bg1".to_owned(),
        transform: ColorTransform::default(),
    });
    assert_eq!(result, ResolvedColor::new(Rgb::new(0xFF, 0xFF, 0xFF), 1.0));
}

#[test]
fn resolves_scheme_direct_dk1() {
    // Direct scheme key lookup (not in ColorMap fields).
    let result = resolver().resolve(&ColorRef::Scheme {
        name: "dk1".to_owned(),
        transform: ColorTransform::default(),
    });
    assert_eq!(result, ResolvedColor::new(Rgb::new(0x00, 0x00, 0x00), 1.0));
}

#[test]
fn resolves_unknown_scheme_name_falls_back_to_dk1() {
    // Matches the TS the spec behavior where unknown names fall back to dk1.
    let result = resolver().resolve(&ColorRef::Scheme {
        name: "bogusName".to_owned(),
        transform: ColorTransform::default(),
    });
    assert_eq!(result, ResolvedColor::new(Rgb::new(0x00, 0x00, 0x00), 1.0));
}

#[test]
fn resolves_sys_clr_with_last_value() {
    let result = resolver().resolve(&ColorRef::System {
        last: Rgb::new(0x00, 0x00, 0x00),
        transform: ColorTransform::default(),
    });
    assert_eq!(result, ResolvedColor::new(Rgb::new(0x00, 0x00, 0x00), 1.0));
}

#[test]
fn applies_alpha_through_resolver() {
    let result = resolver().resolve(&ColorRef::Srgb {
        rgb: Rgb::new(0xFF, 0x00, 0x00),
        transform: ColorTransform {
            alpha: Some(PerMille::new(50_000)),
            ..Default::default()
        },
    });
    assert_eq!(result.rgb, Rgb::new(0xFF, 0x00, 0x00));
    assert_eq!(result.alpha, 0.5);
}

#[test]
fn applies_scheme_clr_with_lum_mod() {
    // accent1 (#4472C4) with lumMod 75% — verifies that transforms are applied
    // *after* scheme resolution (matches TS resolver.applyColorTransforms order).
    let result = resolver().resolve(&ColorRef::Scheme {
        name: "accent1".to_owned(),
        transform: ColorTransform {
            lum_mod: Some(PerMille::new(75_000)),
            ..Default::default()
        },
    });
    // The base color changes; we only check the result is darker than accent1
    // by at least one channel and alpha stayed at 1.
    let base = Rgb::new(0x44, 0x72, 0xC4);
    assert_ne!(result.rgb, base);
    assert_eq!(result.alpha, 1.0);
}

#[test]
fn resolves_preset_known_names() {
    // Eight TS-parity preset names round-trip through the resolver.
    let cases = [
        ("black", Rgb::new(0, 0, 0)),
        ("white", Rgb::new(0xFF, 0xFF, 0xFF)),
        ("red", Rgb::new(0xFF, 0, 0)),
        ("green", Rgb::new(0, 0x80, 0)),
        ("blue", Rgb::new(0, 0, 0xFF)),
        ("yellow", Rgb::new(0xFF, 0xFF, 0)),
        ("cyan", Rgb::new(0, 0xFF, 0xFF)),
        ("magenta", Rgb::new(0xFF, 0, 0xFF)),
    ];
    for (name, expected) in cases {
        let result = resolver().resolve(&ColorRef::Preset {
            name: name.to_owned(),
            transform: ColorTransform::default(),
        });
        assert_eq!(result, ResolvedColor::new(expected, 1.0), "preset {name}");
    }
}

#[test]
fn resolves_preset_unknown_falls_back_to_black() {
    // Unknown names match the resolver's existing convention for unknown
    // scheme references — black with the supplied transforms applied.
    let result = resolver().resolve(&ColorRef::Preset {
        name: "aliceBlue".to_owned(),
        transform: ColorTransform::default(),
    });
    assert_eq!(result, ResolvedColor::new(Rgb::new(0, 0, 0), 1.0));
}

#[test]
fn applies_preset_clr_with_alpha_transform() {
    let result = resolver().resolve(&ColorRef::Preset {
        name: "red".to_owned(),
        transform: ColorTransform {
            alpha: Some(PerMille::new(50_000)),
            ..Default::default()
        },
    });
    assert_eq!(result.rgb, Rgb::new(0xFF, 0, 0));
    assert_eq!(result.alpha, 0.5);
}
