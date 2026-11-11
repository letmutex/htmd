use indoc::indoc;
use pretty_assertions::assert_eq;

use crate::common::convert_faithful;

#[test]
fn code_blocks() {
    let html = r#"
        <pre><code>println!("Hello");</code></pre>
        "#;
    assert_eq!(
        "```\nprintln!(\"Hello\");\n```",
        convert_faithful(html).unwrap()
    );
}

#[test]
fn code_blocks_with_lang_class() {
    let html = r#"
        <pre><code class="language-rust">println!("Hello");</code></pre>
        "#;
    assert_eq!(
        "```rust\nprintln!(\"Hello\");\n```",
        convert_faithful(html).unwrap()
    );
}

#[test]
fn faithful_mode_preserves_an_empty_language_class() {
    let html = r#"<pre><code class="language-">Test</code></pre>"#;

    assert_eq!(html, convert_faithful(html).unwrap());
}

#[test]
fn code_blocks_decode_html_entities() {
    let html = r#"<pre><code>let x = 5 &amp;&amp; y &lt; 10;</code></pre>"#;

    assert_eq!(
        "```\nlet x = 5 && y < 10;\n```",
        convert_faithful(html).unwrap()
    );
}

#[test]
fn faithful_mode_pre() {
    assert_eq!(
        convert_faithful(indoc!(r#"<pre>Test</pre>"#)).unwrap(),
        indoc!(r#"<pre>Test</pre>"#)
    );
}

#[test]
fn faithful_mode_code_block1() {
    assert_eq!(
        convert_faithful(indoc!(r#"<pre><code accesskey="f">Test</code></pre>"#)).unwrap(),
        indoc!(r#"<pre><code accesskey="f">Test</code></pre>"#)
    );
}

#[test]
fn faithful_mode_code_block2() {
    assert_eq!(
        convert_faithful(indoc!(
            r#"<pre><code class="language-ruby"><i>Test</i></code></pre>"#
        ))
        .unwrap(),
        indoc!(r#"<pre><code class="language-ruby"><i>Test</i></code></pre>"#)
    );
}
