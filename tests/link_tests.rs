use htmd::{
    Element, HtmlToMarkdown,
    element_handler::{HandlerResult, Handlers},
    options::{LinkStyle, Options, TranslationMode},
};
mod common;
use common::convert;

fn convert_nested_reference(_handlers: &dyn Handlers, _element: Element) -> Option<HandlerResult> {
    HtmlToMarkdown::builder()
        .options(Options {
            link_style: LinkStyle::Referenced,
            ..Default::default()
        })
        .build()
        .convert(r#"<a href="/inner">Inner</a>"#)
        .ok()
        .map(Into::into)
}

#[test]
fn links() {
    let html = r#"
        <a href="https://example.com">Link 1</a>
        <a href="https://example.com" title="Hello">Link 2</a>
        "#;
    assert_eq!(
        "[Link 1](https://example.com) [Link 2](https://example.com \"Hello\")",
        convert(html).unwrap(),
    );
}

#[test]
fn links_with_spaces_in_destination_and_title() {
    assert_eq!(
        r#"[Link](<https://example.com/hello world> "Hello")"#,
        convert(r#"<a href="https://example.com/hello world" title="Hello">Link</a>"#).unwrap(),
    );
}

#[test]
fn links_with_spaces_around_text() {
    assert_eq!("[bla](/)", convert(r#"<a href="/"> bla </a>"#).unwrap());
    assert_eq!(
        "Some [random](/) text",
        convert(r#"Some <a href="/"> random </a> text"#).unwrap()
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

#[test]
fn referenced_links_are_scoped_to_reentrant_conversions() {
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            link_style: LinkStyle::Referenced,
            ..Default::default()
        })
        .add_handler(vec!["nested"], convert_nested_reference)
        .build();

    let markdown = converter
        .convert(r#"<a href="/one">One</a><nested></nested><a href="/two">Two</a>"#)
        .unwrap();

    assert_eq!(
        "[One][1][Inner][1]\n\n[1]: /inner[Two][2]\n\n[1]: /one\n[2]: /two",
        markdown
    );
}
