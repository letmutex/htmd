use htmd::{
    Element, HtmlToMarkdown,
    element_handler::{ElementHandler, HandlerResult, Handlers},
    options::{LinkStyle, Options, TranslationMode},
};
mod common;
use common::convert_faithful;

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

fn discard_nested_reference(_handlers: &dyn Handlers, _element: Element) -> Option<HandlerResult> {
    let _ = HtmlToMarkdown::builder()
        .options(Options {
            link_style: LinkStyle::Referenced,
            ..Default::default()
        })
        .build()
        .convert(r#"<a href="/discarded">Discarded</a>"#);
    None
}

struct ConvertReferenceOnAppend;

impl ElementHandler for ConvertReferenceOnAppend {
    fn append(&self) -> Option<String> {
        HtmlToMarkdown::builder()
            .options(Options {
                link_style: LinkStyle::Referenced,
                ..Default::default()
            })
            .build()
            .convert(r#"<a href="/inner">Inner</a>"#)
            .ok()
    }

    fn handle(&self, handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
        Some(handlers.walk_children(element.node))
    }
}

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
        "[One][1][Inner](/inner)[Two][2]\n\n[1]: /one\n[2]: /two",
        markdown
    );
}

#[test]
fn referenced_links_from_reentrant_append_are_inlined() {
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            link_style: LinkStyle::Referenced,
            ..Default::default()
        })
        .add_handler(vec!["append-conversion"], ConvertReferenceOnAppend)
        .build();

    let markdown = converter.convert(r#"<a href="/outer">Outer</a>"#).unwrap();

    assert_eq!("[Outer][1]\n\n[1]: /outer\n\n[Inner](/inner)", markdown);
}

#[test]
fn discarded_reentrant_conversion_does_not_add_references() {
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            link_style: LinkStyle::Referenced,
            ..Default::default()
        })
        .add_handler(vec!["discard"], discard_nested_reference)
        .build();

    let markdown = converter
        .convert(r#"<discard></discard><a href="/outer">Outer</a>"#)
        .unwrap();

    assert_eq!("[Outer][1]\n\n[1]: /outer", markdown);
}

#[test]
fn faithful_table_fallback_discards_caption_link_references() {
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            link_style: LinkStyle::Referenced,
            translation_mode: TranslationMode::Faithful,
            ..Default::default()
        })
        .build();
    let html = concat!(
        r#"<table><caption><a href="/caption">Caption link</a>"#,
        r#"<span class="label">Caption</span></caption>"#,
        r#"<tr><th>Header</th></tr><tr><td>Cell</td></tr></table>"#,
        r#"<a href="/after">After</a>"#
    );

    let markdown = converter.convert(html).unwrap();

    assert_eq!(
        concat!(
            r#"<table><caption><a href="/caption">Caption link</a>"#,
            r#"<span class="label">Caption</span></caption>"#,
            r#"<tbody><tr><th>Header</th></tr><tr><td>Cell</td></tr></tbody></table>"#,
            "\n\n[After][1]\n\n[1]: /after"
        ),
        markdown
    );
}
