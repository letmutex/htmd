use pretty_assertions::assert_eq;

use htmd::{
    Element, HtmlToMarkdown,
    element_handler::{ElementHandler, HandlerResult, Handlers},
    options::Options,
};

#[test]
fn with_head() {
    let html = r#"
    <html>
        <head>
            <title>Demo</title>
            <script>console.log('Hello');</script>
            <style>body {}</style>
        </head>
        <body>
            Content
        </body>
    </html>
    "#;
    assert_eq!(
        "Demo\n\nconsole.log('Hello');\n\nbody {}\n\nContent",
        htmd::convert(html).unwrap()
    );
}

#[test]
fn with_custom_rules() {
    // Remove element
    let html = r#"<img src="https://example.com"/>"#;
    let md = HtmlToMarkdown::builder()
        .add_handler(vec!["img"], |_: &dyn Handlers, _element: &Element| None)
        .build()
        .convert(html)
        .unwrap();
    assert_eq!("", &md);
}

#[test]
fn with_custom_rules_and_fallback() {
    let html = r#"<img src="https://example.com"/>"#;
    let converter = HtmlToMarkdown::builder()
        .add_handler(vec!["img"], |handlers: &dyn Handlers, element: &Element| {
            if element.attr("id") == Some("do_not_skip_me") {
                handlers.fallback(element)
            } else {
                None
            }
        })
        .options(Options {
            ..Default::default()
        })
        .build();
    assert_eq!("", &converter.convert(html).unwrap());

    let html = r#"<img src="https://example.com" id="do_not_skip_me"/>"#;
    assert_eq!(
        "![](https://example.com)",
        &converter.convert(html).unwrap()
    );
}

#[test]
fn scripting_option() {
    let html = r#"<noscript><p>Hello</p></noscript>"#;
    let md = HtmlToMarkdown::builder()
        .scripting_enabled(true)
        .build()
        .convert(html)
        .unwrap();
    assert_eq!(r#"\<p>Hello\</p>"#, md);

    let md = HtmlToMarkdown::builder()
        .scripting_enabled(false)
        .build()
        .convert(html)
        .unwrap();
    assert_eq!("Hello", md);
}

#[test]
fn append_content_when_the_body_is_empty() {
    struct Footnote;

    impl ElementHandler for Footnote {
        fn handle(&self, _handlers: &dyn Handlers, _element: &Element) -> Option<HandlerResult> {
            None
        }

        fn append(&self) -> Option<String> {
            Some("\n\n[^1]: A footnote.".to_string())
        }
    }

    let converter = HtmlToMarkdown::builder()
        .add_handler(vec!["img"], Footnote)
        .build();

    // Appended content opens with a blank line separating it from the body.
    // With the body empty, that blank line must not lead the document.
    assert_eq!(
        "[^1]: A footnote.",
        converter.convert("<img src=\"a.png\">").unwrap()
    );
    assert_eq!(
        "Hello\n\n[^1]: A footnote.",
        converter
            .convert("<p>Hello</p><img src=\"a.png\">")
            .unwrap()
    );
}
