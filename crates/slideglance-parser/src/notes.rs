//! Speaker-notes (presenter notes) parser.
//!
//! Mirrors. OOXML stores notes in
//! `ppt/notesSlides/notesSlide{N}.xml`; the XML mirrors slide structure
//! (`<p:notes><p:cSld><p:spTree>...`) but we only need the human-readable
//! text from body placeholders, not full layout or styling. The `sldNum`
//! placeholder is intentionally skipped so the slide number isn't included
//! in the notes string.
//!
//! Implementation note: notes parsing is the only place in the crate that
//! cares about preserving leading/trailing whitespace inside element bodies
//! (e.g. `<a:t>Alpha </a:t>`). quick-xml's serde deserializer trims that
//! whitespace away, so we drive a direct event reader here instead of
//! routing through `parse_xml`.

use quick_xml::events::Event;
use quick_xml::name::QName;
use quick_xml::reader::Reader;

use crate::xml::XmlError;

/// Extracts concatenated speaker-notes text from a `notesSlide` XML body.
///
/// Returns `Ok(None)` when no notes content is present. Paragraphs are
/// joined with `\n`; runs (`<a:r>`) and fields (`<a:fld>`) within a
/// paragraph are concatenated without separators.
///
/// # Errors
///
/// Returns [`XmlError`] when the input is not well-formed XML.
pub fn parse_notes_text(xml: &str) -> Result<Option<String>, XmlError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut paragraphs: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut sp_depth: u32 = 0;
    let mut skip_sp = false;
    let mut in_paragraph = false;
    let mut in_t = false;

    loop {
        match reader.read_event() {
            Ok(Event::Eof) => break,
            Ok(Event::Start(start)) => {
                let local = local_name(start.name());
                match local {
                    b"sp" => {
                        sp_depth += 1;
                        skip_sp = false;
                    }
                    b"ph" if sp_depth > 0 && has_sld_num_type(&start)? => {
                        skip_sp = true;
                    }
                    b"p" if sp_depth > 0 && !skip_sp => {
                        in_paragraph = true;
                        current.clear();
                    }
                    b"t" if in_paragraph => {
                        in_t = true;
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(start)) => {
                if local_name(start.name()) == b"ph" && sp_depth > 0 && has_sld_num_type(&start)? {
                    skip_sp = true;
                }
            }
            Ok(Event::End(end)) => {
                let local = local_name(end.name());
                match local {
                    b"sp" => {
                        sp_depth = sp_depth.saturating_sub(1);
                        if sp_depth == 0 {
                            skip_sp = false;
                        }
                    }
                    b"p" if in_paragraph => {
                        if !current.is_empty() {
                            paragraphs.push(std::mem::take(&mut current));
                        }
                        in_paragraph = false;
                    }
                    b"t" => {
                        in_t = false;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(t)) => {
                if in_t {
                    let unescaped = t.unescape().map_err(XmlError::Read)?;
                    current.push_str(&unescaped);
                }
            }
            Ok(_) => {}
            Err(e) => return Err(XmlError::Read(e)),
        }
    }

    if paragraphs.is_empty() {
        return Ok(None);
    }
    let joined = paragraphs.join("\n");
    let trimmed = joined.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_owned()))
    }
}

fn has_sld_num_type(start: &quick_xml::events::BytesStart<'_>) -> Result<bool, XmlError> {
    for attr in start.attributes() {
        let attr = attr.map_err(XmlError::Attr)?;
        if local_name(attr.key) == b"type" && attr.value.as_ref() == b"sldNum" {
            return Ok(true);
        }
    }
    Ok(false)
}

fn local_name(name: QName<'_>) -> &[u8] {
    let bytes = name.into_inner();
    match bytes.iter().position(|&b| b == b':') {
        Some(i) => &bytes[i + 1..],
        None => bytes,
    }
}
