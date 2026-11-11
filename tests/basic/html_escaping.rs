use pretty_assertions::assert_eq;
use pulldown_cmark::Options as CommonMarkOptions;

use crate::common::{convert_pure, render_markdown};

/// Pure mode writes a `span`, an `a` without an `href` and an empty `em` as
/// their bare content, so the text node after one continues the text node
/// before it. A `<` opening a tag or an autolink which the first ends partway
/// through is escaped, since that node cannot see whether the rest follows.
/// Unescaped, each of these reads back as raw HTML or a link instead of the
/// text it was.
#[test]
fn a_sequence_finished_by_a_sibling_stays_text() {
    for (html, expected_markdown, expected_content) in [
        (
            "<p>&lt;<span>div</span> class&gt;x</p>",
            r"\<div class>x",
            "&lt;div class&gt;x",
        ),
        (
            "<p>&lt;<a>!--</a> x --&gt; y</p>",
            r"\<!-- x --> y",
            "&lt;!-- x --&gt; y",
        ),
        ("<p>a&lt;<em></em>b&gt;c</p>", r"a\<b>c", "a&lt;b&gt;c"),
        ("<p>&lt;/<span>p</span> &gt;</p>", r"\</p >", "&lt;/p &gt;"),
        (
            "<p>&lt;<span>http://x.y</span>&gt;</p>",
            r"\<http://x.y\>",
            "&lt;http://x.y&gt;",
        ),
        (
            "<p>&lt;1<span>@b.co&gt;</span></p>",
            r"\<1@b.co>",
            "&lt;1@b.co&gt;",
        ),
    ] {
        let markdown = convert_pure(html).unwrap();
        assert_eq!(
            format!("<p>{expected_content}</p>\n"),
            render_markdown(&markdown, CommonMarkOptions::empty()),
            "round trip of {html}"
        );
        assert_eq!(expected_markdown, markdown, "markdown for {html}");
    }
}
