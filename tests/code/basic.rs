use htmd::{
    Element, HtmlToMarkdown,
    element_handler::Handlers,
    options::{CodeBlockFence, Options},
};
#[cfg(feature = "rcdom")]
use markup5ever_rcdom::{Node as RcDomNode, NodeData};
use pretty_assertions::assert_eq;

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
    let span = tree
        .root()
        .find(|node| node.tag_name() == Some("span"))
        .unwrap();

    assert_eq!("  *literal*  \nsecond", converter.convert(&span).unwrap());
}

#[test]
fn delegated_unhandled_subtree_preserves_ancestor_preformatted_context() {
    let converter = HtmlToMarkdown::builder()
        .add_handler(
            vec!["delegate"],
            |handlers: &dyn Handlers, element: &Element| {
                let child = element.node.first_child()?;
                handlers.handle(&child, element.context)
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

/// Pure mode has no HTML fallback for a code span holding a line ending, which
/// `translation_approach::inline_faithful::code_span_holding_a_line_ending` covers for
/// faithful mode. CommonMark reads such a line ending as a space, so writing
/// the space keeps the rendered span and leaves the heading whole; the bare
/// newline would end the heading and strand the rest of it in a paragraph.
#[test]
fn pure_mode_writes_a_code_span_line_ending_as_the_space_commonmark_reads() {
    assert_eq!(
        "# a `x y` b",
        htmd::convert("<h1>a <code>x\ny</code> b</h1>").unwrap()
    );
    assert_eq!(
        "a `x y` b",
        htmd::convert("<p>a <code>x\ny</code> b</p>").unwrap()
    );
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

#[cfg(feature = "rcdom")]
#[test]
fn processing_instruction_nodes_are_ignored() {
    let node = RcDomNode::new(NodeData::ProcessingInstruction {
        target: "xml".into(),
        contents: "version=\"1.0\"".into(),
    });

    assert_eq!("", htmd::convert(&node).unwrap());
}

// See https://github.com/letmutex/htmd/issues/138
#[test]
fn code_block_with_spans_ending_in_newline() {
    let html = "<pre><code>\
        <span class=\"line\"><span class=\"cl\">sudo apt update\n</span></span>\
        <span class=\"line\"><span class=\"cl\">sudo apt install curl\n</span></span>\
        </code></pre>";
    assert_eq!(
        "```\nsudo apt update\nsudo apt install curl\n```",
        htmd::convert(html).unwrap()
    );
}

#[test]
fn code_block_with_blank_lines_and_indentation_in_spans() {
    let html = r#"<pre><code class="language-python"><span class="line"><span class="cl"><span class="k">def</span> <span class="nf">foo</span><span class="p">():</span>
</span></span><span class="line"><span class="cl">
</span></span><span class="line"><span class="cl">
</span></span><span class="line"><span class="cl">    <span class="k">return</span> <span class="mi">42</span>
</span></span></code></pre>"#;
    assert_eq!(
        "```python\ndef foo():\n\n\n    return 42\n```",
        htmd::convert(html).unwrap()
    );
}

#[test]
fn span_fallback_in_pre_preserves_newlines() {
    let converter = HtmlToMarkdown::builder()
        .add_handler(
            vec!["span"],
            |handlers: &dyn Handlers, element: &Element| handlers.fallback(element),
        )
        .build();

    let html = "<pre><code><span>sudo apt update\n</span><span>sudo apt install curl\n</span></code></pre>";
    assert_eq!(
        "```\nsudo apt update\nsudo apt install curl\n```",
        converter.convert(html).unwrap()
    );
}
