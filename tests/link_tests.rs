use htmd::{
    HtmlToMarkdown,
    options::{LinkStyle, Options, TranslationMode},
};
mod common;
use common::convert_faithful;

#[test]
fn links() {
    let html = r#"
        <a href="https://example.com">Link 1</a>
        <a href="https://example.com" title="Hello">Link 2</a>
        "#;
    assert_eq!(
        "[Link 1](https://example.com) [Link 2](https://example.com \"Hello\")",
        convert_faithful(html).unwrap(),
    );
}

#[test]
fn links_with_spaces_in_destination_and_title() {
    assert_eq!(
        r#"[Link](<https://example.com/hello world> "Hello")"#,
        convert_faithful(r#"<a href="https://example.com/hello world" title="Hello">Link</a>"#)
            .unwrap(),
    );
}

#[test]
fn links_with_spaces_around_text() {
    assert_eq!(
        "[bla](/)",
        convert_faithful(r#"<a href="/"> bla </a>"#).unwrap()
    );
    assert_eq!(
        "Some [random](/) text",
        convert_faithful(r#"Some <a href="/"> random </a> text"#).unwrap()
    )
}

#[test]
fn links_inlined_prefer_autolinks() {
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            translation_mode: TranslationMode::Faithful,
            link_style: LinkStyle::InlinedPreferAutolinks,
            ..Default::default()
        })
        .build();

    let html = r#"<a href="https://example.com">https://example.com</a>"#;
    assert_eq!("<https://example.com>", converter.convert(html).unwrap());

    let html = r#"<a href="https://example.com">Link</a>"#;
    assert_eq!(
        "[Link](https://example.com)",
        converter.convert(html).unwrap()
    );

    let html = r#"<a href="https://example.com" title="https://example.com">Link</a>"#;
    assert_eq!(
        r#"[Link](https://example.com "https://example.com")"#,
        converter.convert(html).unwrap()
    );
}


/// A line ending in a link destination ends the leaf block holding it: written
/// literally, `a[t](u⏎⏎v)b` is two paragraphs and no link at all. CommonMark
/// decodes a character reference in a destination, so it is encoded instead.
#[test]
fn a_link_destination_escapes_its_line_endings() {
    assert_eq!(
        "a[t](u&#10;v)b",
        convert_faithful("<p>a<a href=\"u\nv\">t</a>b</p>").unwrap()
    );
    assert_eq!(
        "a[t](u&#10;&#10;v)b",
        convert_faithful("<p>a<a href=\"u\n\nv\">t</a>b</p>").unwrap()
    );
    assert_eq!(
        "# a[t](u&#10;v)b",
        convert_faithful("<h1>a<a href=\"u\nv\">t</a>b</h1>").unwrap()
    );
    // A carriage return reaches the destination only as a character reference,
    // the parser having folded any literal CRLF into a line feed.
    assert_eq!(
        "a[t](u&#13;v)b",
        convert_faithful("<p>a<a href=\"u&#13;v\">t</a>b</p>").unwrap()
    );
    // An image's destination takes the same encoding.
    assert_eq!(
        "a![](i&#10;j)b",
        convert_faithful("<p>a<img src=\"i\nj\">b</p>").unwrap()
    );
    assert_eq!(
        "[t](u)",
        convert_faithful(r#"<p><a href="u">t</a></p>"#).unwrap()
    );
    assert_eq!(
        r"[t](u\(v\))",
        convert_faithful(r#"<p><a href="u(v)">t</a></p>"#).unwrap()
    );
}
