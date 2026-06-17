//! Light smoke tests verifying that the tagged-union enums serialize and
//! deserialize round-trip via JSON. The model crate has no parsing logic
//! (Phase 3-3 owns that); these tests exist to catch accidental
//! `#[serde(tag = "...")]` mismatches between the type definitions and
//! whatever the parser will emit.

#![cfg(feature = "serde")]

use slideglance_color::{ResolvedColor, Rgb};
use slideglance_model::{Fill, SolidFill};

#[test]
fn fill_solid_roundtrip() {
    let original = Fill::Solid(SolidFill {
        color: ResolvedColor::opaque(Rgb::new(0xFF, 0x00, 0x00)),
    });
    let json = serde_json::to_string(&original).unwrap();
    assert!(
        json.contains(r#""type":"solid""#),
        "expected lowercase tag, got {json}"
    );
    let parsed: Fill = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, original);
}

#[test]
fn fill_none_roundtrip() {
    use slideglance_model::NoFill;
    let original = Fill::None(NoFill {});
    let json = serde_json::to_string(&original).unwrap();
    assert!(
        json.contains(r#""type":"none""#),
        "expected lowercase tag, got {json}"
    );
    let parsed: Fill = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, original);
}
