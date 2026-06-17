//! Ported from.

use slideglance_parser::parse_notes_text;

#[test]
fn returns_none_for_missing_notes_root() {
    assert!(parse_notes_text("<other/>").unwrap().is_none());
}

#[test]
fn returns_none_for_empty_sp_tree() {
    let xml = r#"
        <p:notes xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
            <p:cSld><p:spTree/></p:cSld>
        </p:notes>
    "#;
    assert!(parse_notes_text(xml).unwrap().is_none());
}

#[test]
fn extracts_single_paragraph_text() {
    let xml = r#"
        <p:notes xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                 xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <p:cSld><p:spTree>
                <p:sp>
                    <p:nvSpPr><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr>
                    <p:txBody>
                        <a:p><a:r><a:t>Hello world</a:t></a:r></a:p>
                    </p:txBody>
                </p:sp>
            </p:spTree></p:cSld>
        </p:notes>
    "#;
    assert_eq!(
        parse_notes_text(xml).unwrap().as_deref(),
        Some("Hello world")
    );
}

#[test]
fn joins_paragraphs_with_newline() {
    let xml = r#"
        <p:notes xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                 xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <p:cSld><p:spTree>
                <p:sp><p:txBody>
                    <a:p><a:r><a:t>First line</a:t></a:r></a:p>
                    <a:p><a:r><a:t>Second line</a:t></a:r></a:p>
                </p:txBody></p:sp>
            </p:spTree></p:cSld>
        </p:notes>
    "#;
    assert_eq!(
        parse_notes_text(xml).unwrap().as_deref(),
        Some("First line\nSecond line"),
    );
}

#[test]
fn concatenates_runs_within_paragraph_without_separator() {
    let xml = r#"
        <p:notes xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                 xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <p:cSld><p:spTree>
                <p:sp><p:txBody>
                    <a:p>
                        <a:r><a:t>Alpha </a:t></a:r>
                        <a:r><a:t>Beta </a:t></a:r>
                        <a:r><a:t>Gamma</a:t></a:r>
                    </a:p>
                </p:txBody></p:sp>
            </p:spTree></p:cSld>
        </p:notes>
    "#;
    assert_eq!(
        parse_notes_text(xml).unwrap().as_deref(),
        Some("Alpha Beta Gamma"),
    );
}

#[test]
fn includes_field_text() {
    let xml = r#"
        <p:notes xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                 xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <p:cSld><p:spTree>
                <p:sp><p:txBody>
                    <a:p>
                        <a:r><a:t>Date: </a:t></a:r>
                        <a:fld id="{1234}" type="datetime"><a:t>2026-04-28</a:t></a:fld>
                    </a:p>
                </p:txBody></p:sp>
            </p:spTree></p:cSld>
        </p:notes>
    "#;
    assert_eq!(
        parse_notes_text(xml).unwrap().as_deref(),
        Some("Date: 2026-04-28"),
    );
}

#[test]
fn skips_slide_number_placeholder() {
    let xml = r#"
        <p:notes xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                 xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <p:cSld><p:spTree>
                <p:sp>
                    <p:nvSpPr><p:nvPr><p:ph type="sldNum"/></p:nvPr></p:nvSpPr>
                    <p:txBody><a:p><a:r><a:t>Slide 5</a:t></a:r></a:p></p:txBody>
                </p:sp>
                <p:sp>
                    <p:nvSpPr><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr>
                    <p:txBody><a:p><a:r><a:t>Real notes</a:t></a:r></a:p></p:txBody>
                </p:sp>
            </p:spTree></p:cSld>
        </p:notes>
    "#;
    assert_eq!(
        parse_notes_text(xml).unwrap().as_deref(),
        Some("Real notes")
    );
}

#[test]
fn returns_none_when_only_whitespace_paragraphs() {
    let xml = r#"
        <p:notes xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                 xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <p:cSld><p:spTree>
                <p:sp><p:txBody>
                    <a:p><a:r><a:t>   </a:t></a:r></a:p>
                </p:txBody></p:sp>
            </p:spTree></p:cSld>
        </p:notes>
    "#;
    // Whitespace-only paragraphs trim to empty after the join.
    assert!(parse_notes_text(xml).unwrap().is_none());
}
