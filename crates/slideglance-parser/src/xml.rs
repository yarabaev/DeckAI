//! XML deserialization helpers for PPTX content.
//!
//! [`parse_xml`] mirrors the TypeScript reference's
//! `XMLParser({ removeNSPrefix: true })` semantics: tag and attribute
//! namespace prefixes (`a:`, `p:`, `r:`, …) are stripped before deserializing,
//! and `xmlns` declaration attributes are dropped entirely. Downstream
//! `#[serde(rename = "...")]` annotations therefore use the *local* element
//! and attribute names.

use std::fmt;
use std::str::Utf8Error;

use quick_xml::events::attributes::AttrError;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::name::QName;
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use quick_xml::DeError as QuickXmlDeError;
use quick_xml::Error as QuickXmlError;
use serde::de::DeserializeOwned;

/// Strips XML namespace prefixes from element names and attribute names, and
/// drops `xmlns` / `xmlns:*` declaration attributes, then deserializes the
/// result into `T` via `quick-xml`'s serde integration.
///
/// # Errors
///
/// - [`XmlError::Read`] if the input is not well-formed XML.
/// - [`XmlError::Deserialize`] if the stripped XML does not match `T`.
pub fn parse_xml<T: DeserializeOwned>(xml: &str) -> Result<T, XmlError> {
    let stripped = strip_namespaces(xml)?;
    quick_xml::de::from_str(&stripped).map_err(XmlError::Deserialize)
}

/// Returns a copy of `xml` with every XML namespace prefix removed and every
/// `xmlns`/`xmlns:*` declaration attribute dropped.
///
/// # Errors
///
/// [`XmlError::Read`] if the input is not well-formed XML.
pub fn strip_namespaces(xml: &str) -> Result<String, XmlError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut buf = Vec::with_capacity(xml.len());
    let mut writer = Writer::new(&mut buf);
    // Element name stack — used to scope the space-guard substitution
    // to text inside `<a:t>` (post-strip: local name `"t"`). We must
    // not touch inter-element indent whitespace; quick-xml-de relies
    // on it being trimmable to deserialize element-only structs.
    let mut stack: Vec<Vec<u8>> = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Eof) => break,
            Ok(Event::Start(start)) => {
                let local = local_part(start.name()).to_vec();
                let rewritten = rewrite_start(&start)?;
                writer
                    .write_event(Event::Start(rewritten))
                    .map_err(XmlError::Write)?;
                stack.push(local);
            }
            Ok(Event::Empty(start)) => {
                let rewritten = rewrite_start(&start)?;
                writer
                    .write_event(Event::Empty(rewritten))
                    .map_err(XmlError::Write)?;
            }
            Ok(Event::End(end)) => {
                let rewritten = rewrite_end(&end);
                writer
                    .write_event(Event::End(rewritten))
                    .map_err(XmlError::Write)?;
                stack.pop();
            }
            Ok(Event::Text(t)) => {
                let in_text_element = stack.last().is_some_and(|n| n.as_slice() == b"t");
                let raw: &[u8] = t.as_ref();
                if in_text_element && raw.contains(&0x20) {
                    // Substitute 0x20 with the Unicode PUA codepoint
                    // U+F0E1 so quick-xml-de's reader-level text trim
                    // (no public override) leaves leading / trailing
                    // spaces of `<a:t>` content alone — PowerPoint
                    // legitimately splits date strings into runs like
                    // `<a:t>년 </a:t><a:t>04</a:t>`, and dropping the
                    // trailing space silently glues them together.
                    // `extract_text` restores spaces on the way out.
                    let mut escaped = Vec::with_capacity(raw.len());
                    for &b in raw {
                        if b == 0x20 {
                            escaped.extend_from_slice(SPACE_GUARD_BYTES);
                        } else {
                            escaped.push(b);
                        }
                    }
                    let s = std::str::from_utf8(&escaped).map_err(XmlError::NotUtf8)?;
                    writer
                        .write_event(Event::Text(BytesText::from_escaped(s.to_string())))
                        .map_err(XmlError::Write)?;
                } else {
                    writer
                        .write_event(Event::Text(t))
                        .map_err(XmlError::Write)?;
                }
            }
            Ok(other) => writer.write_event(other).map_err(XmlError::Write)?,
            Err(e) => return Err(XmlError::Read(e)),
        }
    }

    String::from_utf8(buf).map_err(|e| XmlError::NotUtf8(e.utf8_error()))
}

/// UTF-8 encoding of U+F0E1 — the Private Use Area codepoint we use
/// as a placeholder for ASCII spaces during XML strip/deserialize so
/// quick-xml-de's mandatory text trim doesn't drop them.
pub const SPACE_GUARD_BYTES: &[u8] = b"\xEF\x83\xA1";

/// String form of [`SPACE_GUARD_BYTES`] (U+F0E1).
pub const SPACE_GUARD: &str = "\u{F0E1}";

/// Reverse the [`SPACE_GUARD`] substitution applied by
/// [`strip_namespaces`]. Call this on every text-content string
/// extracted from a deserialized model before downstream rendering.
#[must_use]
pub fn unguard_spaces(s: &str) -> String {
    if s.contains(SPACE_GUARD) {
        s.replace(SPACE_GUARD, " ")
    } else {
        s.to_string()
    }
}

fn rewrite_start(start: &BytesStart<'_>) -> Result<BytesStart<'static>, XmlError> {
    let local = local_part(start.name());
    let mut new_start = BytesStart::new(String::from_utf8_lossy(local).into_owned());

    for attr in start.attributes() {
        let attr = attr.map_err(XmlError::Attr)?;
        if is_xmlns_attr(attr.key) {
            continue;
        }
        let local_key = local_part(attr.key);
        // push_attribute copies bytes internally (extend_from_slice), so the
        // borrowed slices below do not outlive the call.
        new_start.push_attribute((local_key, attr.value.as_ref()));
    }

    Ok(new_start)
}

fn rewrite_end(end: &BytesEnd<'_>) -> BytesEnd<'static> {
    let local = local_part(end.name());
    BytesEnd::new(String::from_utf8_lossy(local).into_owned())
}

/// Returns the local-part bytes of `name`, i.e. the slice after the first
/// `:`, or the full slice if there is no prefix.
fn local_part(name: QName<'_>) -> &[u8] {
    let bytes = name.into_inner();
    match bytes.iter().position(|&b| b == b':') {
        Some(i) => &bytes[i + 1..],
        None => bytes,
    }
}

fn is_xmlns_attr(key: QName<'_>) -> bool {
    let bytes = key.into_inner();
    bytes == b"xmlns" || bytes.starts_with(b"xmlns:")
}

/// Failure modes when stripping namespaces or deserializing XML.
#[derive(Debug)]
pub enum XmlError {
    /// Failed to tokenize the XML input.
    Read(QuickXmlError),
    /// Failed to parse an attribute (malformed quotes, etc.).
    Attr(AttrError),
    /// I/O error while emitting the stripped XML.
    Write(std::io::Error),
    /// The stripped output was not valid UTF-8.
    NotUtf8(Utf8Error),
    /// Failed to deserialize the stripped XML into the target type.
    Deserialize(QuickXmlDeError),
}

impl fmt::Display for XmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(e) => write!(f, "xml read error: {e}"),
            Self::Attr(e) => write!(f, "xml attribute error: {e}"),
            Self::Write(e) => write!(f, "xml write io error: {e}"),
            Self::NotUtf8(e) => write!(f, "stripped xml not utf-8: {e}"),
            Self::Deserialize(e) => write!(f, "xml deserialize error: {e}"),
        }
    }
}

impl std::error::Error for XmlError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read(e) => Some(e),
            Self::Attr(e) => Some(e),
            Self::Write(e) => Some(e),
            Self::NotUtf8(e) => Some(e),
            Self::Deserialize(e) => Some(e),
        }
    }
}
