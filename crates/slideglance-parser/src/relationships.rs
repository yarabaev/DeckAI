//! `.rels` parser: extracts `<Relationship>` entries and provides path
//! resolution helpers for OOXML part references.
//!
//! Mirrors.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::xml::{parse_xml, XmlError};

/// One `<Relationship>` entry from a `.rels` part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Relationship {
    /// `@Id` — referenced from XML bodies (e.g. `<a:blip r:embed="rId7"/>`).
    pub id: String,
    /// `@Type` — full relationship-type URL.
    pub ty: String,
    /// `@Target` — relative or absolute (leading `/`) path to the target part.
    pub target: String,
    /// `@TargetMode` (typically `"External"` for hyperlinks; absent for
    /// in-package targets).
    pub target_mode: Option<String>,
}

/// Parses a `.rels` XML body into a map keyed by relationship `Id`.
///
/// Entries missing any of `@Id` / `@Type` / `@Target` are skipped (matching
/// the TS the spec behavior where they emit a debug warning and drop
/// the entry).
///
/// # Errors
///
/// Returns [`XmlError`] if the XML is malformed.
pub fn parse_relationships(xml: &str) -> Result<BTreeMap<String, Relationship>, XmlError> {
    let root: RawRoot = parse_xml(xml)?;
    let mut out = BTreeMap::new();

    for raw in root.relationship {
        let (Some(id), Some(ty), Some(target)) = (raw.id, raw.ty, raw.target) else {
            // Matches the TS "entry missing required attribute, skipping"
            // debug log: silently drop the malformed entry.
            continue;
        };
        out.insert(
            id.clone(),
            Relationship {
                id,
                ty,
                target,
                target_mode: raw.target_mode,
            },
        );
    }

    Ok(out)
}

/// Returns the conventional `.rels` path for a part path.
///
/// `ppt/slides/slide1.xml` → `ppt/slides/_rels/slide1.xml.rels`,
/// `ppt/presentation.xml` → `ppt/_rels/presentation.xml.rels`.
#[must_use]
pub fn build_rels_path(xml_path: &str) -> String {
    match xml_path.rfind('/') {
        Some(last_slash) => {
            let dir = &xml_path[..last_slash];
            let filename = &xml_path[last_slash + 1..];
            format!("{dir}/_rels/{filename}.rels")
        }
        None => format!("_rels/{xml_path}.rels"),
    }
}

/// Resolves a relationship `@Target` against a base part path.
///
/// - Absolute targets (leading `/`) are returned with the leading slash
///   stripped.
/// - Relative targets are joined to the base part's directory and any `..`
///   segments are collapsed.
#[must_use]
pub fn resolve_relationship_target(base_path: &str, rel_target: &str) -> String {
    if let Some(stripped) = rel_target.strip_prefix('/') {
        return stripped.to_owned();
    }
    let base_dir = match base_path.rfind('/') {
        Some(i) => &base_path[..i],
        None => "",
    };
    let joined = if base_dir.is_empty() {
        rel_target.to_owned()
    } else {
        format!("{base_dir}/{rel_target}")
    };
    let mut resolved: Vec<&str> = Vec::new();
    for part in joined.split('/') {
        match part {
            ".." => {
                resolved.pop();
            }
            "." | "" => {}
            other => resolved.push(other),
        }
    }
    resolved.join("/")
}

#[derive(Debug, Deserialize)]
struct RawRoot {
    #[serde(default, rename = "Relationship")]
    relationship: Vec<RawRel>,
}

#[derive(Debug, Deserialize)]
struct RawRel {
    #[serde(rename = "@Id")]
    id: Option<String>,
    #[serde(rename = "@Type")]
    ty: Option<String>,
    #[serde(rename = "@Target")]
    target: Option<String>,
    #[serde(rename = "@TargetMode")]
    target_mode: Option<String>,
}
