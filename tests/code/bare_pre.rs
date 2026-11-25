use htmd::{
    HtmlToMarkdown,
    element_handler::{Context, Element, Handlers},
    options::{CodeBlockFence, CodeBlockStyle, Options},
};
use pretty_assertions::assert_eq;

#[test]
fn bare_pre_is_not_a_code_block_by_default() {
    assert_eq!("foo\nbar", htmd::convert("<pre>foo\nbar</pre>").unwrap());
    assert_eq!("\\```foo", htmd::convert("<pre>```foo</pre>").unwrap());
}

#[test]
fn bare_pre_as_code_block_fenced_and_language() {
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            bare_pre_as_code_block: true,
            ..Default::default()
        })
        .build();

    assert_eq!(
        "```\nfoo\nbar\n```",
        converter.convert("<pre>foo\nbar</pre>").unwrap()
    );
    assert_eq!(
        "```\nfoo\n```",
        converter.convert("<pre>foo\n</pre>").unwrap()
    );
    assert_eq!(
        "```rust\nfn main() {}\n```",
        converter
            .convert(r#"<pre class="language-rust">fn main() {}</pre>"#)
            .unwrap()
    );
    assert_eq!(
        "```\nsudo apt update\nsudo apt install curl\n```",
        converter
            .convert(
                "<pre><span>sudo apt update\n</span><span>sudo apt install curl\n</span></pre>"
            )
            .unwrap()
    );
    assert_eq!("```\n\n```", converter.convert("<pre></pre>").unwrap());
}

#[test]
fn bare_pre_as_code_block_handles_leading_fence_chars_without_escaping() {
    let backtick_converter = HtmlToMarkdown::builder()
        .options(Options {
            bare_pre_as_code_block: true,
            ..Default::default()
        })
        .build();

    assert_eq!(
        "````\n```rust\nfn main() {}\n```\n````",
        backtick_converter
            .convert("<pre>```rust\nfn main() {}\n```</pre>")
            .unwrap()
    );
    assert_eq!(
        "```\n`foo`\n```",
        backtick_converter.convert("<pre>`foo`</pre>").unwrap()
    );

    let tilde_converter = HtmlToMarkdown::builder()
        .options(Options {
            bare_pre_as_code_block: true,
            code_block_fence: CodeBlockFence::Tildes,
            ..Default::default()
        })
        .build();

    assert_eq!(
        "~~~~\n~~~\nfoo\n~~~\n~~~~",
        tilde_converter.convert("<pre>~~~\nfoo\n~~~</pre>").unwrap()
    );
}

#[test]
fn bare_pre_as_code_block_indented() {
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            bare_pre_as_code_block: true,
            code_block_style: CodeBlockStyle::Indented,
            ..Default::default()
        })
        .build();

    assert_eq!(
        "    foo\n    bar",
        converter
            .convert("<pre class=\"language-rust\">foo\nbar</pre>")
            .unwrap()
    );
    assert_eq!("", converter.convert("<pre></pre>").unwrap());
}

#[test]
fn bare_pre_as_code_block_does_not_double_wrap_code_or_nested_pre() {
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            bare_pre_as_code_block: true,
            ..Default::default()
        })
        .build();

    assert_eq!(
        "```\nfoo\n```",
        converter.convert("<pre><code>foo</code></pre>").unwrap()
    );
    assert_eq!(
        "  ```\nfoo\n```",
        converter
            .convert("<pre>\n  <code>foo</code>\n</pre>")
            .unwrap()
    );
    assert_eq!(
        "```\nfoo\n```",
        converter.convert("<pre><pre>foo</pre></pre>").unwrap()
    );
    assert_eq!(
        "```\nfoo\n```",
        converter
            .convert("<pre><pre><code>foo</code></pre></pre>")
            .unwrap()
    );
    assert_eq!(
        "```\na\n\nb\n\nc\n```",
        converter.convert("<pre>a<pre>b</pre>c</pre>").unwrap()
    );

    let indented = HtmlToMarkdown::builder()
        .options(Options {
            bare_pre_as_code_block: true,
            code_block_style: CodeBlockStyle::Indented,
            ..Default::default()
        })
        .build();
    assert_eq!(
        "    foo",
        indented
            .convert("<pre><pre><code>foo</code></pre></pre>")
            .unwrap()
    );
}

#[test]
fn default_mode_preserves_nested_pre_sibling_separation() {
    assert_eq!(
        "a\n\nb\n\nc",
        htmd::convert("<pre>a<pre>b</pre>c</pre>").unwrap()
    );
    assert_eq!(
        "```\na\n\nb\n\nc\n```",
        htmd::convert("<pre><code>a<pre>b</pre>c</code></pre>").unwrap()
    );
}

#[test]
fn unwrapped_pre_paths_keep_fence_escaping_when_bare_pre_option_is_enabled() {
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            bare_pre_as_code_block: true,
            ..Default::default()
        })
        .build();

    assert_eq!(
        "\\```text\n```\nfoo\n```",
        converter
            .convert("<pre>```text\n<code>foo</code></pre>")
            .unwrap()
    );

    let tree = converter
        .html_to_tree("<pre><pre>```foo</pre></pre>")
        .unwrap();
    let outer_pre = tree
        .root()
        .find(|node| node.tag_name() == Some("pre"))
        .unwrap();
    let inner_pre = outer_pre.first_child().unwrap();
    assert_eq!("\\```foo", converter.convert(&inner_pre).unwrap());

    let custom_pre = HtmlToMarkdown::builder()
        .options(Options {
            bare_pre_as_code_block: true,
            ..Default::default()
        })
        .add_handler(vec!["pre"], |handlers: &dyn Handlers, element: &Element| {
            Some(
                handlers
                    .walk_children_content(
                        element.node,
                        Context {
                            literal: true,
                            ..element.context
                        },
                    )
                    .into(),
            )
        })
        .build();
    assert_eq!("\\```foo", custom_pre.convert("<pre>```foo</pre>").unwrap());
}

#[test]
fn bare_pre_as_code_block_classes_containers_and_trailing_newlines() {
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            bare_pre_as_code_block: true,
            ..Default::default()
        })
        .build();

    assert_eq!(
        "```rust\nfn main() {}\n```",
        converter
            .convert(r#"<pre class="highlight language-rust">fn main() {}</pre>"#)
            .unwrap()
    );
    assert_eq!(
        "```\nTest\n```",
        converter
            .convert(r#"<pre class="language-">Test</pre>"#)
            .unwrap()
    );
    assert_eq!(
        "```\nfoo\n\n```",
        converter.convert("<pre>foo\n\n</pre>").unwrap()
    );
    assert_eq!(
        "before\n\n```\ncode\n```\n\nafter",
        converter
            .convert("<p>before</p><pre>code</pre><p>after</p>")
            .unwrap()
    );
    assert_eq!(
        "> ```\n> code\n> ```",
        converter
            .convert("<blockquote><pre>code</pre></blockquote>")
            .unwrap()
    );
    assert_eq!(
        "*   ```\n    code\n    ```",
        converter
            .convert("<ul><li><pre>code</pre></li></ul>")
            .unwrap()
    );
}

#[test]
fn custom_pre_and_code_handlers_fall_back_independently() {
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            bare_pre_as_code_block: true,
            ..Default::default()
        })
        .add_handler(vec!["pre"], |handlers: &dyn Handlers, element: &Element| {
            handlers.fallback(element)
        })
        .add_handler(
            vec!["code"],
            |handlers: &dyn Handlers, element: &Element| handlers.fallback(element),
        )
        .build();

    assert_eq!(
        "```\nbare\n```\n\n`inline`",
        converter
            .convert("<pre>bare</pre><p><code>inline</code></p>")
            .unwrap()
    );
}

#[test]
fn bare_pre_as_code_block_skips_inline_contexts() {
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            bare_pre_as_code_block: true,
            ..Default::default()
        })
        .build();

    assert_eq!(
        "# foo",
        converter.convert("<h1><pre>foo</pre></h1>").unwrap()
    );
    assert_eq!(
        "[&#10;&#10;foo&#10;&#10;](https://example.com)",
        converter
            .convert(r#"<a href="https://example.com"><pre>foo</pre></a>"#)
            .unwrap()
    );
    assert_eq!(
        "| h   |\n| --- |\n| foo |",
        converter
            .convert("<table><thead><tr><th>h</th></tr></thead><tbody><tr><td><pre>foo</pre></td></tr></tbody></table>")
            .unwrap()
    );
}

#[test]
fn bare_pre_as_code_block_handles_nested_wrappers_and_code_pre_without_leaks() {
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            bare_pre_as_code_block: true,
            ..Default::default()
        })
        .build();

    assert_eq!(
        "```\nfoo\n```",
        converter
            .convert("<pre><div><pre><code>foo</code></pre></div></pre>")
            .unwrap()
    );
    assert_eq!(
        "```\n\n\n> foo\n\n```",
        converter
            .convert("<pre><blockquote><pre>foo</pre></blockquote></pre>")
            .unwrap()
    );
    assert_eq!(
        "```\n\n\n*   foo\n\n```",
        converter
            .convert("<pre><ul><li><pre>foo</pre></li></ul></pre>")
            .unwrap()
    );
    assert_eq!(
        htmd::convert("<pre><code><pre>foo</pre></code></pre>").unwrap(),
        converter
            .convert("<pre><code><pre>foo</pre></code></pre>")
            .unwrap()
    );
}
