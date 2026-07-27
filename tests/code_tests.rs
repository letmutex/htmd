use std::rc::Rc;

use htmd::{
    Element, HtmlToMarkdown, Node,
    element_handler::Handlers,
    options::{CodeBlockFence, Options},
};
use indoc::indoc;
use markup5ever_rcdom::NodeData;
use pretty_assertions::assert_eq;

mod common;
use common::convert;

fn find_element(node: &Rc<Node>, tag: &str) -> Option<Rc<Node>> {
    if let NodeData::Element { name, .. } = &node.data
        && name.local.as_ref() == tag
    {
        return Some(node.clone());
    }

    node.children
        .borrow()
        .iter()
        .find_map(|child| find_element(child, tag))
}

#[test]
fn code_blocks() {
    let html = r#"
        <pre><code>println!("Hello");</code></pre>
        "#;
    assert_eq!("```\nprintln!(\"Hello\");\n```", convert(html).unwrap());
}

#[test]
fn code_blocks_with_lang_class() {
    let html = r#"
        <pre><code class="language-rust">println!("Hello");</code></pre>
        "#;
    assert_eq!("```rust\nprintln!(\"Hello\");\n```", convert(html).unwrap());
}

#[test]
fn code_blocks_decode_html_entities() {
    let html = r#"<pre><code>let x = 5 &amp;&amp; y &lt; 10;</code></pre>"#;

    assert_eq!("```\nlet x = 5 && y < 10;\n```", convert(html).unwrap());
}

// See https://github.com/letmutex/htmd/issues/14 for background on this test --
// the `class` attribute is deliberately misplaced to support Markdown renderers
// which don't follow the CommonMark spec.
#[test]
fn code_blocks_with_lang_class_on_pre_tag() {
    let html = r#"
        <pre class="language-rust"><code>println!("Hello");</code></pre>
        "#;
    assert_eq!(
        "```rust\nprintln!(\"Hello\");\n```",
        htmd::convert(html).unwrap()
    );
}

#[test]
fn span_subtree_conversion_preserves_ancestor_preformatted_context() {
    let converter = HtmlToMarkdown::new();
    let tree = converter
        .html_to_tree("<pre><span>  *literal*  \nsecond</span></pre>")
        .unwrap();
    let span = find_element(&tree, "span").unwrap();

    assert_eq!("  *literal*  \nsecond", converter.tree_to_markdown(&span));
}

#[test]
fn delegated_unhandled_subtree_preserves_ancestor_preformatted_context() {
    let converter = HtmlToMarkdown::builder()
        .add_handler(
            vec!["delegate"],
            |handlers: &dyn Handlers, element: Element| {
                let child = element.node.children.borrow().first()?.clone();
                handlers.handle(&child)
            },
        )
        .build();

    assert_eq!(
        "  *literal*  \nsecond",
        converter
            .convert("<pre><delegate><mark>  *literal*  \nsecond</mark></delegate></pre>")
            .unwrap()
    );
}

#[test]
fn faithful_mode_pre() {
    assert_eq!(
        convert(indoc!(r#"<pre>Test</pre>"#)).unwrap(),
        indoc!(r#"<pre>Test</pre>"#)
    );
}

#[test]
fn faithful_mode_code_block1() {
    assert_eq!(
        convert(indoc!(r#"<pre><code accesskey="f">Test</code></pre>"#)).unwrap(),
        indoc!(r#"<pre><code accesskey="f">Test</code></pre>"#)
    );
}

#[test]
fn faithful_mode_code_block2() {
    assert_eq!(
        convert(indoc!(
            r#"<pre><code class="language-ruby"><i>Test</i></code></pre>"#
        ))
        .unwrap(),
        indoc!(r#"<pre><code class="language-ruby"><i>Test</i></code></pre>"#)
    );
}

#[test]
fn inline_code_made_only_of_backticks_uses_a_non_colliding_delimiter() {
    let markdown = htmd::convert("<code>``</code>").unwrap();

    assert_eq!("``` `` ```", markdown);
}

#[test]
fn inline_code_ending_in_a_backtick_keeps_the_backtick_inside_the_span() {
    let markdown = htmd::convert("<code>code`</code>").unwrap();

    assert_eq!("`` code` ``", markdown);
}

#[test]
fn preformatted_inline_code_preserves_boundary_spaces() {
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            preformatted_code: true,
            ..Default::default()
        })
        .build();

    let markdown = converter.convert("<code> foo </code>").unwrap();

    assert_eq!("`  foo  `", markdown);
}

#[test]
fn fenced_code_uses_a_fence_longer_than_any_run_in_its_content() {
    let markdown =
        htmd::convert("<pre><code>`````\nlet parsed = true;\n`````</code></pre>").unwrap();

    assert_eq!("``````\n`````\nlet parsed = true;\n`````\n``````", markdown);
}

#[test]
fn tilde_fenced_code_uses_a_fence_longer_than_any_run_in_its_content() {
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            code_block_fence: CodeBlockFence::Tildes,
            ..Default::default()
        })
        .build();

    let markdown = converter
        .convert("<pre><code>~~~~~\nlet parsed = true;\n~~~~~</code></pre>")
        .unwrap();

    assert_eq!("~~~~~~\n~~~~~\nlet parsed = true;\n~~~~~\n~~~~~~", markdown);
}
