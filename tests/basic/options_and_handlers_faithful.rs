use std::{sync::Arc, thread::JoinHandle};

use pretty_assertions::assert_eq;

use htmd::{
    HtmlToMarkdown,
    element_handler::{Context, HandlerResult, Handlers},
    node::Node,
    options::{LinkStyle, Options, TranslationMode},
};

use crate::common::convert_faithful;

#[test]
fn upper_case_tags() {
    let html = r#"<H1>Hello</H1> <P>World</P>"#;
    assert_eq!("# Hello\n\nWorld", convert_faithful(html).unwrap());
}

#[test]
fn multithreading() {
    let html = r#"<a href="https://example.com">Example</a>
    <a href="https://example.com">Example</a>
    <a href="https://example.com">Example</a>
    <a href="https://example.com">Example</a>
    <a href="https://example.com">Example</a>
    "#;
    let expected = "[Example][1] [Example][2] [Example][3] [Example][4] [Example][5]\n\n\
    [1]: https://example.com\n[2]: https://example.com\n[3]: https://example.com\n\
    [4]: https://example.com\n[5]: https://example.com";
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            // We use a global vec to store all referenced links of the doc in
            // the anchor element handler, this is unsafe for multithreading
            // usage if we do nothing
            link_style: LinkStyle::Referenced,
            translation_mode: TranslationMode::Faithful,
            ..Default::default()
        })
        .build();
    let converter = Arc::new(converter);
    let mut handlers: Vec<JoinHandle<()>> = vec![];
    for _ in 0..20 {
        let converter_clone = converter.clone();
        let handle = std::thread::spawn(move || {
            let md = converter_clone.convert(html).unwrap();
            assert_eq!(expected, md);
        });
        handlers.push(handle);
    }
    for handle in handlers {
        handle.join().unwrap();
    }
}

#[test]
fn custom_comment_handler_overrides_faithful_comments() {
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            translation_mode: TranslationMode::Faithful,
            ..Default::default()
        })
        .comment_handler(
            |_handlers: &dyn Handlers, node: &Node<'_>, context: Context| {
                let text = node.comment()?;
                if text == "skip" {
                    return None;
                }
                if context.is_block() {
                    Some(HandlerResult {
                        content: format!("\n\n<!--custom:{text}-->\n\n"),
                        markdown_translated: text != "fail",
                    })
                } else {
                    Some(HandlerResult {
                        content: format!("<!--inline:{text}-->"),
                        markdown_translated: false,
                    })
                }
            },
        )
        .build();

    assert_eq!(
        "<!--custom:top-->\n\na<!--inline:in-->b",
        converter
            .convert("<!--top--><!--skip--><p>a<!--in-->b</p>")
            .unwrap()
    );
    assert_eq!(
        "*   <!--custom:li-->",
        converter.convert("<ul><li><!--li--></li></ul>").unwrap()
    );
    assert_eq!(
        "<ol><!--fail--><li>a</li></ol>",
        converter.convert("<ol><!--fail--><li>a</li></ol>").unwrap()
    );
}
