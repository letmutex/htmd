use crate::common::convert_pure;

/// A code span or code block does not evaluate character references, so line
/// endings in link text in a literal context are left unencoded (converted to
/// spaces in code spans, preserved in code blocks).
#[test]
fn link_with_line_break_in_literal_context() {
    assert_eq!(
        "`[first second](https://example.com)`",
        convert_pure("<code><a href=\"https://example.com\">first\nsecond</a></code>").unwrap()
    );
    assert_eq!(
        "`[first second](https://example.com)`",
        convert_pure("<code><a href=\"https://example.com\">first\r\nsecond</a></code>").unwrap()
    );
    assert_eq!(
        "```\n[first\nsecond](https://example.com)\n```",
        convert_pure("<pre><code><a href=\"https://example.com\">first\nsecond</a></code></pre>")
            .unwrap()
    );
}
