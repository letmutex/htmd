use htmd::HtmlToMarkdown;
use htmd::options::{LinkStyle, Options};

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
