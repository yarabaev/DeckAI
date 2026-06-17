//! Ported from
//! and (representative cases).

use slideglance_parser::{
    evaluate_formula, evaluate_guides, parse_custom_geometry, GuideDefinition,
};

// --- evaluate_formula ---

#[test]
fn val_returns_constant() {
    let vars = std::collections::BTreeMap::new();
    assert_eq!(evaluate_formula("val 100", &vars), 100.0);
}

#[test]
fn val_resolves_variable_name() {
    let mut vars = std::collections::BTreeMap::new();
    vars.insert("w".to_owned(), 1000.0);
    assert_eq!(evaluate_formula("val w", &vars), 1000.0);
}

#[test]
fn plus_minus_op() {
    let vars = std::collections::BTreeMap::new();
    assert_eq!(evaluate_formula("+- 10 20 5", &vars), 25.0);
}

#[test]
fn multiply_divide_op_rounds_to_integer() {
    let vars = std::collections::BTreeMap::new();
    // (10 * 3) / 2 = 15
    assert_eq!(evaluate_formula("*/ 10 3 2", &vars), 15.0);
}

#[test]
fn divide_by_zero_falls_back_to_one() {
    let vars = std::collections::BTreeMap::new();
    assert_eq!(evaluate_formula("*/ 10 5 0", &vars), 50.0);
}

#[test]
fn pin_clamps_value_into_range() {
    let vars = std::collections::BTreeMap::new();
    assert_eq!(evaluate_formula("pin 10 5 20", &vars), 10.0);
    assert_eq!(evaluate_formula("pin 10 25 20", &vars), 20.0);
    assert_eq!(evaluate_formula("pin 10 15 20", &vars), 15.0);
}

#[test]
fn min_max_abs() {
    let vars = std::collections::BTreeMap::new();
    assert_eq!(evaluate_formula("min 3 5", &vars), 3.0);
    assert_eq!(evaluate_formula("max 3 5", &vars), 5.0);
    assert_eq!(evaluate_formula("abs -7", &vars), 7.0);
}

#[test]
fn sin_at_zero_returns_zero() {
    let vars = std::collections::BTreeMap::new();
    assert_eq!(evaluate_formula("sin 100 0", &vars), 0.0);
}

#[test]
fn ternary_picks_branch_by_sign() {
    let vars = std::collections::BTreeMap::new();
    assert_eq!(evaluate_formula("?: 1 100 200", &vars), 100.0);
    assert_eq!(evaluate_formula("?: -1 100 200", &vars), 200.0);
    assert_eq!(evaluate_formula("?: 0 100 200", &vars), 200.0);
}

#[test]
fn unknown_op_returns_zero() {
    let vars = std::collections::BTreeMap::new();
    assert_eq!(evaluate_formula("xyz 1 2 3", &vars), 0.0);
}

// --- evaluate_guides ---

#[test]
fn evaluate_guides_resolves_av_then_gd() {
    let av = vec![GuideDefinition {
        name: "adj1".into(),
        fmla: "val 50000".into(),
    }];
    let gd = vec![GuideDefinition {
        name: "x1".into(),
        // depends on adj1, which was set above
        fmla: "*/ adj1 w 100000".into(),
    }];
    let vars = evaluate_guides(&av, &gd, 1000.0, 500.0);
    assert_eq!(vars["adj1"], 50_000.0);
    assert_eq!(vars["x1"], 500.0);
    // Built-ins still present
    assert_eq!(vars["w"], 1000.0);
    assert_eq!(vars["wd2"], 500.0);
    assert_eq!(vars["ss"], 500.0);
}

// --- parse_custom_geometry ---

#[test]
fn returns_none_for_empty_path_lst() {
    assert!(parse_custom_geometry("<a:custGeom/>").unwrap().is_none());
}

#[test]
fn parses_rectangle_path() {
    let xml = r#"<a:custGeom xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:pathLst>
            <a:path w="100" h="50">
                <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
                <a:lnTo><a:pt x="100" y="0"/></a:lnTo>
                <a:lnTo><a:pt x="100" y="50"/></a:lnTo>
                <a:lnTo><a:pt x="0" y="50"/></a:lnTo>
                <a:close/>
            </a:path>
        </a:pathLst>
    </a:custGeom>"#;
    let result = parse_custom_geometry(xml).unwrap().unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].width, 100.0);
    assert_eq!(result[0].height, 50.0);
    assert_eq!(result[0].commands, "M 0 0 L 100 0 L 100 50 L 0 50 Z");
}

#[test]
fn parses_cubic_bezier_path() {
    let xml = r#"<a:custGeom xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:pathLst>
            <a:path w="100" h="100">
                <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
                <a:cubicBezTo>
                    <a:pt x="10" y="10"/>
                    <a:pt x="50" y="50"/>
                    <a:pt x="100" y="100"/>
                </a:cubicBezTo>
            </a:path>
        </a:pathLst>
    </a:custGeom>"#;
    let result = parse_custom_geometry(xml).unwrap().unwrap();
    assert_eq!(result[0].commands, "M 0 0 C 10 10, 50 50, 100 100");
}

#[test]
fn quadratic_bezier_uses_q_command() {
    let xml = r#"<a:custGeom xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:pathLst>
            <a:path w="100" h="100">
                <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
                <a:quadBezTo>
                    <a:pt x="50" y="0"/>
                    <a:pt x="100" y="100"/>
                </a:quadBezTo>
            </a:path>
        </a:pathLst>
    </a:custGeom>"#;
    let result = parse_custom_geometry(xml).unwrap().unwrap();
    assert_eq!(result[0].commands, "M 0 0 Q 50 0, 100 100");
}

#[test]
fn resolves_guide_reference_in_path() {
    let xml = r#"<a:custGeom xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:gdLst>
            <a:gd name="midX" fmla="*/ w 1 2"/>
        </a:gdLst>
        <a:pathLst>
            <a:path w="200" h="200">
                <a:moveTo><a:pt x="midX" y="0"/></a:moveTo>
                <a:lnTo><a:pt x="midX" y="200"/></a:lnTo>
            </a:path>
        </a:pathLst>
    </a:custGeom>"#;
    let result = parse_custom_geometry(xml).unwrap().unwrap();
    assert_eq!(result[0].commands, "M 100 0 L 100 200");
}

#[test]
fn skips_path_with_zero_dimensions() {
    let xml = r#"<a:custGeom xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:pathLst>
            <a:path w="0" h="0">
                <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
            </a:path>
        </a:pathLst>
    </a:custGeom>"#;
    let result = parse_custom_geometry(xml).unwrap();
    assert!(result.is_none());
}
