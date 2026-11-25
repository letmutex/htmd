use htmd::{
    HtmlToMarkdown,
    options::{Options, TranslationMode},
};
use pretty_assertions::assert_eq;

#[test]
fn faithful_mode_ignores_bare_pre_as_code_block() {
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            bare_pre_as_code_block: true,
            translation_mode: TranslationMode::Faithful,
            ..Default::default()
        })
        .build();

    assert_eq!(
        "<pre>Test</pre>",
        converter.convert("<pre>Test</pre>").unwrap()
    );
    assert_eq!(
        r#"<pre class="language-rust">Test</pre>"#,
        converter
            .convert(r#"<pre class="language-rust">Test</pre>"#)
            .unwrap()
    );
}
