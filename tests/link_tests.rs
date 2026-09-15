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

#[test]
fn nested_convert_with_referenced_links() {
    use htmd::Element;
    use htmd::element_handler::Handlers;
    use indoc::indoc;

    let converter = HtmlToMarkdown::builder()
        .options(Options {
            link_style: LinkStyle::Referenced,
            ..Default::default()
        })
        .add_handler(
            vec!["widget"],
            |_handlers: &dyn Handlers, _element: &Element| {
                let inner_converter = HtmlToMarkdown::builder()
                    .options(Options {
                        link_style: LinkStyle::Referenced,
                        ..Default::default()
                    })
                    .build();
                let inner_md = inner_converter
                    .convert(r#"<a href="https://inner.com">Inner</a>"#)
                    .unwrap();
                Some(inner_md.into())
            },
        )
        .build();

    let html = indoc!(
        r#"
        <p><a href="https://outer1.com">Outer 1</a></p>
        <widget></widget>
        <p><a href="https://outer2.com">Outer 2</a></p>
        "#
    );
    let md = converter.convert(html).unwrap();

    assert_eq!(
        indoc!(
            r#"
            [Outer 1][1]

            [Inner][1]

            [1]: https://inner.com

            [Outer 2][2]

            [1]: https://outer1.com
            [2]: https://outer2.com"#
        ),
        md
    );
}

#[test]
fn discard_links_in_table_falling_back_to_raw_html() {
    use indoc::indoc;

    let converter = HtmlToMarkdown::builder()
        .options(Options {
            link_style: LinkStyle::Referenced,
            translation_mode: TranslationMode::Faithful,
            ..Default::default()
        })
        .build();

    let html = indoc!(
        r#"
        <table>
            <tr>
                <td><a href="https://discarded.com">Discarded</a></td>
                <td><custom-unsupported>test</custom-unsupported></td>
            </tr>
        </table>
        <p><a href="https://kept.com">Kept</a></p>
        "#
    );
    let md = converter.convert(html).unwrap();

    assert_eq!(
        indoc!(
            r#"
            <table>
                <tbody><tr>
                    <td><a href="https://discarded.com">Discarded</a></td>
                    <td><custom-unsupported>test</custom-unsupported></td>
                </tr>
            </tbody></table>

            [Kept][1]

            [1]: https://kept.com"#
        ),
        md
    );
}
