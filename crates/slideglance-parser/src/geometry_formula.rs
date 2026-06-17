//! OOXML geometry guide-value formula evaluator.
//!
//! Mirrors. Evaluates the
//! formulas in `<a:avLst>` / `<a:gdLst>` so that path coordinate references
//! (guide names) can be resolved to numeric values.

use std::collections::BTreeMap;
use std::f64::consts::PI;

/// One guide definition (`<a:gd name="..." fmla="..."/>`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuideDefinition {
    /// Guide name as it appears in path commands (e.g. `"adj1"`).
    pub name: String,
    /// Formula string (`"val 12700"`, `"*/ w 1 2"`, etc.).
    pub fmla: String,
}

/// Built-in OOXML guide variables for a `w`×`h` shape.
fn builtin_variables(w: f64, h: f64) -> BTreeMap<String, f64> {
    let mut vars = BTreeMap::new();
    vars.insert("w".into(), w);
    vars.insert("h".into(), h);
    vars.insert("l".into(), 0.0);
    vars.insert("t".into(), 0.0);
    vars.insert("r".into(), w);
    vars.insert("b".into(), h);
    vars.insert("wd2".into(), w / 2.0);
    vars.insert("hd2".into(), h / 2.0);
    vars.insert("wd4".into(), w / 4.0);
    vars.insert("hd4".into(), h / 4.0);
    vars.insert("wd5".into(), w / 5.0);
    vars.insert("hd5".into(), h / 5.0);
    vars.insert("wd6".into(), w / 6.0);
    vars.insert("hd6".into(), h / 6.0);
    vars.insert("wd8".into(), w / 8.0);
    vars.insert("hd8".into(), h / 8.0);
    vars.insert("wd10".into(), w / 10.0);
    vars.insert("hd10".into(), h / 10.0);
    vars.insert("wd12".into(), w / 12.0);
    vars.insert("hd12".into(), h / 12.0);
    vars.insert("wd32".into(), w / 32.0);
    vars.insert("hd32".into(), h / 32.0);
    let ss = w.min(h);
    let ls = w.max(h);
    vars.insert("ss".into(), ss);
    vars.insert("ls".into(), ls);
    vars.insert("ssd2".into(), ss / 2.0);
    vars.insert("ssd4".into(), ss / 4.0);
    vars.insert("ssd6".into(), ss / 6.0);
    vars.insert("ssd8".into(), ss / 8.0);
    vars.insert("ssd16".into(), ss / 16.0);
    vars.insert("ssd32".into(), ss / 32.0);
    // OOXML angle constants in 60000ths-of-a-degree.
    vars.insert("cd2".into(), 10_800_000.0);
    vars.insert("cd4".into(), 5_400_000.0);
    vars.insert("cd8".into(), 2_700_000.0);
    vars.insert("3cd4".into(), 16_200_000.0);
    vars.insert("3cd8".into(), 8_100_000.0);
    vars.insert("5cd8".into(), 13_500_000.0);
    vars.insert("7cd8".into(), 18_900_000.0);
    vars
}

fn resolve(token: Option<&&str>, vars: &BTreeMap<String, f64>) -> f64 {
    let Some(t) = token else { return 0.0 };
    if let Ok(num) = t.parse::<f64>() {
        return num;
    }
    vars.get(*t).copied().unwrap_or(0.0)
}

/// Evaluates an OOXML guide formula string, returning the numeric result.
///
/// Supported operators (per ECMA-376 §20.1.9.11):
/// `val` / `+-` / `*\/` / `+\/` / `sin` / `cos` / `tan` / `at2` / `sqrt` /
/// `min` / `max` / `abs` / `pin` / `mod` (3D length) / `cat2` / `sat2` /
/// `?:`. Unknown operators return `0` to match the spec.
#[must_use]
pub fn evaluate_formula(fmla: &str, vars: &BTreeMap<String, f64>) -> f64 {
    let tokens: Vec<&str> = fmla.split_whitespace().collect();
    let Some(&op) = tokens.first() else {
        return 0.0;
    };

    let arg = |i: usize| resolve(tokens.get(i), vars);

    match op {
        "val" => arg(1),
        "+-" => arg(1) + arg(2) - arg(3),
        "*/" => {
            let denom = arg(3);
            let denom = if denom == 0.0 { 1.0 } else { denom };
            (arg(1) * arg(2) / denom).round()
        }
        "+/" => {
            let denom = arg(3);
            let denom = if denom == 0.0 { 1.0 } else { denom };
            ((arg(1) + arg(2)) / denom).round()
        }
        "sin" => (arg(1) * (arg(2) / 60_000.0 * PI / 180.0).sin()).round(),
        "cos" => (arg(1) * (arg(2) / 60_000.0 * PI / 180.0).cos()).round(),
        "tan" => (arg(1) * (arg(2) / 60_000.0 * PI / 180.0).tan()).round(),
        "at2" => (arg(2).atan2(arg(1)) * (180.0 / PI) * 60_000.0).round(),
        "sqrt" => arg(1).sqrt().round(),
        "min" => arg(1).min(arg(2)),
        "max" => arg(1).max(arg(2)),
        "abs" => arg(1).abs(),
        "pin" => {
            let lo = arg(1);
            let val = arg(2);
            let hi = arg(3);
            lo.max(hi.min(val))
        }
        "mod" => {
            let a = arg(1);
            let b = arg(2);
            let c = arg(3);
            (a * a + b * b + c * c).sqrt().round()
        }
        "cat2" => {
            let a = arg(1);
            let b = arg(2);
            let c = arg(3);
            (a * c.atan2(b).cos()).round()
        }
        "sat2" => {
            let a = arg(1);
            let b = arg(2);
            let c = arg(3);
            (a * c.atan2(b).sin()).round()
        }
        "?:" => {
            if arg(1) > 0.0 {
                arg(2)
            } else {
                arg(3)
            }
        }
        _ => 0.0,
    }
}

/// Evaluates `avLst` then `gdLst` against a `w`×`h` viewport, returning every
/// resolved guide variable plus the OOXML built-ins.
#[must_use]
pub fn evaluate_guides(
    av_lst: &[GuideDefinition],
    gd_lst: &[GuideDefinition],
    w: f64,
    h: f64,
) -> BTreeMap<String, f64> {
    let mut vars = builtin_variables(w, h);
    for gd in av_lst {
        let value = evaluate_formula(&gd.fmla, &vars);
        vars.insert(gd.name.clone(), value);
    }
    for gd in gd_lst {
        let value = evaluate_formula(&gd.fmla, &vars);
        vars.insert(gd.name.clone(), value);
    }
    vars
}

/// Resolves a path coordinate reference: numeric literal stays as-is,
/// otherwise the guide variable is looked up. Returns `0.0` for unknown
/// guides (matches TS).
#[must_use]
pub fn resolve_value(value: &str, vars: &BTreeMap<String, f64>) -> f64 {
    if let Ok(n) = value.parse::<f64>() {
        return n;
    }
    vars.get(value).copied().unwrap_or(0.0)
}
