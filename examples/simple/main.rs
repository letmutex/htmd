use htmd::{Element, HtmlToMarkdown, element_handler::Handlers, options::Options};

fn main() {
    let converter = HtmlToMarkdown::new();
    assert_eq!("# Heading", converter.convert("<h1>Heading</h1>").unwrap());

    // Skip tags
    let converter = HtmlToMarkdown::builder()
        .skip_tags(vec!["script", "style"])
        .build();
    assert_eq!(
        "",
        converter.convert("<script>let x = 0;</script>").unwrap()
    );

    // Options
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            heading_style: htmd::options::HeadingStyle::Setex,
            ..Default::default()
        })
        .build();
    assert_eq!(
        "Heading\n=======",
        converter.convert("<h1>Heading</h1>").unwrap()
    );

    // Custom tag handlers
    let converter = HtmlToMarkdown::builder()
        .add_handler(vec!["svg"], |_: &dyn Handlers, _: Element| {
            Some("[Svg Image]".into())
        })
        .build();
    assert_eq!("[Svg Image]", converter.convert("<svg></svg>").unwrap());
}
