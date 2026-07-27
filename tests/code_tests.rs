use htmd::{
    HtmlToMarkdown,
    options::{CodeBlockFence, Options},
};
use pretty_assertions::assert_eq;

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
