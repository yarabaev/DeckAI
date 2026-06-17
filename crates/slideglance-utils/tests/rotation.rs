//! Integration tests for rotation conversion. Mirrors the spec's
//! `rotationToDegrees` test cases.

use slideglance_utils::rotation_to_degrees;

#[test]
fn rotation_5400000_is_90_degrees() {
    assert!((rotation_to_degrees(5_400_000) - 90.0).abs() < f64::EPSILON);
}

#[test]
fn rotation_zero_is_zero_degrees() {
    assert_eq!(rotation_to_degrees(0), 0.0);
}

#[test]
fn rotation_negative_works() {
    assert!((rotation_to_degrees(-5_400_000) - -90.0).abs() < f64::EPSILON);
}

#[test]
fn rotation_full_circle() {
    assert!((rotation_to_degrees(21_600_000) - 360.0).abs() < f64::EPSILON);
}
