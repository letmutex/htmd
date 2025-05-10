use htmd::convert;

#[test]
fn links() {
    let html = r#"
        <a href="https://example.com">Link 1</a>
        <a href="https://example.com" title="Hello">Link 2</a>
        "#;
    assert_eq!(
        "[Link 1](https://example.com)[Link 2](https://example.com \"Hello\")",
        convert(html).unwrap(),
    );
}

#[test]
fn links_with_spaces_around_text() {
    assert_eq!(
        "[bla](/)",
        convert(r#"<a href="/"> bla </a>"#).unwrap()
    );
    assert_eq!(
        "Some [random](/) text",
        convert(r#"Some <a href="/"> random </a> text"#).unwrap()
    )
}
