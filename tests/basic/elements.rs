use pretty_assertions::assert_eq;

use crate::common::convert_pure;

#[test]
fn image_title_stays_outside_an_angle_bracket_destination() {
    let markdown = htmd::convert(
        r#"<img src="https://example.com/image name.png" alt="diagram" title="A title">"#,
    )
    .unwrap();

    assert_eq!(
        r#"![diagram](<https://example.com/image name.png> "A title")"#,
        markdown
    );
}

/// Step 1 of the headings approach in `translation_approach.md`: an inline
/// context holds no block, so pure mode writes only a heading's content there.
/// Faithful mode writes a raw HTML inline instead; see `tests/context/`.
#[test]
fn pure_heading_in_an_inline_context() {
    assert_eq!("*b*", convert_pure("<em><h2>b</h2></em>").unwrap());
    assert_eq!(
        "[b](u)",
        convert_pure(r#"<a href="u"><h2>b</h2></a>"#).unwrap()
    );
    assert_eq!(
        "| b |\n| - |",
        convert_pure("<table><thead><tr><th><h2>b</h2></th></tr></thead></table>").unwrap()
    );
    // The content is trimmed as it is in a block context. Its `#` run is
    // escaped by the enclosing heading, the only one spelled as ATX.
    assert_eq!(
        r"# a \#",
        convert_pure("<h1><div><h2> a # </h2></div></h1>").unwrap()
    );
}

#[test]
fn nested_divs() {
    let html = r#"
    <div>
        <div>
            <div>Hi</div>
        </div>
        <div></div>
        <div>there</div>
    </div>
    "#;
    assert_eq!("Hi\n\nthere", htmd::convert(html).unwrap());
}

#[test]
fn misnested_formatting_does_not_duplicate_or_lose_text() {
    let markdown = htmd::convert("<p><b>one<i>two</b>three</i>four").unwrap();

    assert_eq!("**one*two****three*four", markdown);
}
