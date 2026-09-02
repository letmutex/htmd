//! Tests for the block/inline context an element is translated in, following
//! the "Given an HTML node that can't be encoded as CommonMark" rule of
//! `unsupported_html.md`.

use pretty_assertions::assert_eq;

mod common;
use common::convert_faithful;

#[test]
fn html_at_the_document_root_is_a_block() {
    assert_eq!(
        "<div>a</div>\n\n<div>b</div>",
        convert_faithful("<div>a</div><div>b</div>").unwrap()
    );
    assert_eq!(
        "<div><br><br></div>",
        convert_faithful("<div><br><br></div>").unwrap()
    );
    assert_eq!(
        "# <div><p>a</p></div>",
        convert_faithful("<h1><div><p>a</p></div></h1>").unwrap()
    );
}

/// `br` is not a block-level tag name, so a lone `<br>` on a line would open a
/// [type 7 HTML block][], which cannot interrupt a paragraph and swallows every
/// following line down to the next blank one. Such a tag is therefore a raw
/// HTML inline everywhere, block context included.
///
/// [type 7 HTML block]: https://spec.commonmark.org/0.31.2/#html-blocks
#[test]
fn a_type_7_tag_is_a_raw_inline_even_in_a_block_context() {
    assert_eq!("<br>", convert_faithful("<br>").unwrap());
    assert_eq!("<br><br>", convert_faithful("<br><br>").unwrap());
    assert_eq!(
        r#"This is <em foo="">really</em> important."#,
        convert_faithful("This is <em foo>really</em> important.").unwrap()
    );
    assert_eq!(
        "*   a<br>b",
        convert_faithful("<ul><li>a<br>b</li></ul>").unwrap()
    );
    assert_eq!(
        "> a<br>b",
        convert_faithful("<blockquote>a<br>b</blockquote>").unwrap()
    );
    // A paragraph still frames itself as a block, so a `<br>` beside one does
    // not join it.
    assert_eq!("<br>\n\na", convert_faithful("<br><p>a</p>").unwrap());
    assert_eq!("a\n\n<br>", convert_faithful("<p>a</p><br>").unwrap());
}

#[test]
fn a_commonmark_block_in_an_inline_context_is_a_raw_inline() {
    assert_eq!("# <p>a</p>", convert_faithful("<h1><p>a</p></h1>").unwrap());
    assert_eq!(
        "# x<p>a</p>y",
        convert_faithful("<h1>x<p>a</p>y</h1>").unwrap()
    );
    assert_eq!(
        "# <blockquote>a</blockquote>",
        convert_faithful("<h1><blockquote>a</blockquote></h1>").unwrap()
    );
    assert_eq!(
        "# <ul><li>a</li></ul>",
        convert_faithful("<h1><ul><li>a</li></ul></h1>").unwrap()
    );
    assert_eq!("# <hr>", convert_faithful("<h1><hr></h1>").unwrap());
    // Only the tags of a raw HTML inline are HTML; what it holds is still
    // translated, except in a code block, whose content is literal text.
    assert_eq!(
        "# <pre><code>a</code></pre>",
        convert_faithful("<h1><pre><code>a</code></pre></h1>").unwrap()
    );
    assert_eq!(
        "# <table><thead><tr><th>h</th></tr></thead><tbody><tr><td>a</td></tr></tbody></table>",
        convert_faithful(
            "<h1><table><thead><tr><th>h</th></tr></thead>\
             <tbody><tr><td>a</td></tr></tbody></table></h1>"
        )
        .unwrap()
    );
}

/// Not a faithful translation: CommonMark provides no way to write raw text
/// without opening a paragraph.
#[test]
fn raw_text_at_the_document_root_is_left_alone() {
    assert_eq!("foo", convert_faithful("foo").unwrap());
}

#[test]
fn html_in_a_paragraph_is_a_raw_inline() {
    assert_eq!("<br><br>", convert_faithful("<p><br><br></p>").unwrap());
    assert_eq!(
        "<br>*b*",
        convert_faithful("<p><br><em>b</em></p>").unwrap()
    );
    assert_eq!(
        "*a*<br>",
        convert_faithful("<p><em>a</em><br></p>").unwrap()
    );
    assert_eq!(
        "<br>![](i)",
        convert_faithful(r#"<p><br><img src="i"></p>"#).unwrap()
    );
}

#[test]
fn html_in_a_heading_is_a_raw_inline() {
    assert_eq!("# <br><br>", convert_faithful("<h1><br><br></h1>").unwrap());
    assert_eq!(
        "# <br>*b*",
        convert_faithful("<h1><br><em>b</em></h1>").unwrap()
    );
    assert_eq!(
        "###### *a*<br>",
        convert_faithful("<h6><em>a</em><br></h6>").unwrap()
    );
}

#[test]
fn html_in_a_blockquote_is_a_block() {
    assert_eq!(
        "> <div>a</div>\n> \n> <div>b</div>",
        convert_faithful("<blockquote><div>a</div><div>b</div></blockquote>").unwrap()
    );
    assert_eq!(
        "> <br><br>",
        convert_faithful("<blockquote><br><br></blockquote>").unwrap()
    );
    assert_eq!(
        "> <br>*b*",
        convert_faithful("<blockquote><p><br><em>b</em></p></blockquote>").unwrap()
    );
}

#[test]
fn html_in_a_list_item_is_a_block() {
    assert_eq!(
        "*   <div>a</div>\n\n    <div>b</div>",
        convert_faithful("<ul><li><div>a</div><div>b</div></li></ul>").unwrap()
    );
    assert_eq!(
        "*   <br>\n\n    a",
        convert_faithful("<ul><li><br><p>a</p></li></ul>").unwrap()
    );
    // Row 1 of "Lists" in `unsupported_html.md` wants two HTML blocks here, but
    // `br` is a type 7 tag, so the run stays inline.
    assert_eq!(
        "*   <br><br>",
        convert_faithful("<ul><li><br><br></li></ul>").unwrap()
    );
}

/// A cell's contents are parsed as inline content, so no HTML block can open
/// inside one.
#[test]
fn html_in_a_table_cell_is_a_raw_inline() {
    assert_eq!(
        "| h       |\n| ------- |\n| <br>*b* |",
        convert_faithful(
            "<table><thead><tr><th>h</th></tr></thead>\
             <tbody><tr><td><br><em>b</em></td></tr></tbody></table>"
        )
        .unwrap()
    );
}

#[test]
fn html_in_an_inline_element_is_a_raw_inline() {
    assert_eq!(
        "a<span><br></span>b",
        convert_faithful("<p>a<span><br></span>b</p>").unwrap()
    );
    assert_eq!(
        "a<del><br></del>b",
        convert_faithful("<p>a<del><br></del>b</p>").unwrap()
    );
    assert_eq!(
        "a[<br>](u)b",
        convert_faithful(r#"<p>a<a href="u"><br></a>b</p>"#).unwrap()
    );
}

/// The content of a code span or code block is literal text, so no encoding of
/// a break survives there.
#[test]
fn html_in_code_is_a_raw_inline() {
    assert_eq!(
        "a<code>x<br>y</code>b",
        convert_faithful("<p>a<code>x<br>y</code>b</p>").unwrap()
    );
    assert_eq!(
        "<pre><code>a<br>b</code></pre>",
        convert_faithful("<pre><code>a<br>b</code></pre>").unwrap()
    );
    // An untranslatable attribute on the `<pre>` is no reason to start
    // translating content which must stay literal.
    assert_eq!(
        "# <pre><b>a</b></pre>",
        convert_faithful("<h1><pre><b>a</b></pre></h1>").unwrap()
    );
    assert_eq!(
        r#"# <pre class="x"><b>a</b></pre>"#,
        convert_faithful(r#"<h1><pre class="x"><b>a</b></pre></h1>"#).unwrap()
    );
}

/// A raw text element holds literal characters rather than markup, so it is
/// serialized whole: Markdown escaping or whitespace collapsing there would
/// rewrite the script, style, or textarea itself.
#[test]
fn a_raw_text_element_is_serialized_verbatim() {
    assert_eq!(
        "# <script>a*b_c[d]</script>",
        convert_faithful("<h1><script>a*b_c[d]</script></h1>").unwrap()
    );
    assert_eq!(
        "# <script>if(a<b){}</script>",
        convert_faithful("<h1><script>if(a<b){}</script></h1>").unwrap()
    );
    assert_eq!(
        "x<script>a*b</script>y",
        convert_faithful("<p>x<script>a*b</script>y</p>").unwrap()
    );
    assert_eq!(
        "<script>a*b\nc</script>",
        convert_faithful("<script>a*b\nc</script>").unwrap()
    );
    // The escaped line ending survives the trip back: a CommonMark parser
    // decodes the reference while producing the raw HTML inline.
    assert_eq!(
        "# <script>a&#10;b</script>",
        convert_faithful("<h1><script>a\nb</script></h1>").unwrap()
    );
}

/// `textarea` and `title` hold no markup either, but an HTML parser decodes a
/// character reference inside one. That decoding is the reason raw text
/// elements must be serialized verbatim, so these two take the ordinary raw
/// HTML inline path instead.
#[test]
fn an_rcdata_element_is_translated() {
    assert_eq!(
        r"# <textarea>a\*b\*c</textarea>",
        convert_faithful("<h1><textarea>a*b*c</textarea></h1>").unwrap()
    );
    assert_eq!(
        r"# <title>a\*b\*c</title>",
        convert_faithful("<h1><title>a*b*c</title></h1>").unwrap()
    );
    // The walk compresses a line ending to a space rather than escaping it, so
    // that is the half this path loses.
    assert_eq!(
        "# <textarea>a b</textarea>",
        convert_faithful("<h1><textarea>a\nb</textarea></h1>").unwrap()
    );
    assert_eq!(
        "<textarea>a*b\nc</textarea>",
        convert_faithful("<textarea>a*b\nc</textarea>").unwrap()
    );
}

/// A line ending in a raw HTML inline ends the leaf block holding it: a blank
/// line ends a paragraph, and a single line ending ends an ATX heading or a
/// table row. Each is therefore written as a character reference.
#[test]
fn a_raw_inline_escapes_its_line_endings() {
    assert_eq!(
        r#"a<em foo="1&#10;&#10;2">y</em>b"#,
        convert_faithful("<p>a<em foo=\"1\n\n2\">y</em>b</p>").unwrap()
    );
    assert_eq!(
        "# <pre>x&#10;y</pre>",
        convert_faithful("<h1><pre>x\ny</pre></h1>").unwrap()
    );
    // The parser folds a literal CRLF into a line feed, so the carriage return
    // has to arrive as a character reference to survive.
    assert_eq!(
        "# <pre>x&#13;&#10;y</pre>",
        convert_faithful("<h1><pre>x&#13;&#10;y</pre></h1>").unwrap()
    );
    assert_eq!(
        concat!(
            "| h                        |\n",
            "| ------------------------ |\n",
            "| <em foo=\"1&#10;2\">y</em> |"
        ),
        convert_faithful(
            "<table><thead><tr><th>h</th></tr></thead>\
             <tbody><tr><td><em foo=\"1\n2\">y</em></td></tr></tbody></table>"
        )
        .unwrap()
    );
    // Only a blank line ends a type 6 HTML block, so one keeps the rest of its
    // line structure.
    assert_eq!(
        "<div>a\n&#10;b</div>",
        convert_faithful("<div>a\n\nb</div>").unwrap()
    );
}

/// A type 1 HTML block ends at its closing tag rather than at a blank line, so
/// escaping a blank line inside one would needlessly rewrite the script, style,
/// or preformatted text itself.
#[test]
fn a_type_1_block_keeps_its_blank_lines() {
    assert_eq!(
        "<script>a\n\nb</script>",
        convert_faithful("<script>a\n\nb</script>").unwrap()
    );
    assert_eq!(
        "<style>a\n\nb</style>",
        convert_faithful("<style>a\n\nb</style>").unwrap()
    );
    assert_eq!(
        "<pre>a\n\nb</pre>",
        convert_faithful("<pre>a\n\nb</pre>").unwrap()
    );
    assert_eq!(
        "<textarea>a\n\nb</textarea>",
        convert_faithful("<textarea>a\n\nb</textarea>").unwrap()
    );
    assert_eq!(
        "# <script>a&#10;&#10;b</script>",
        convert_faithful("<h1><script>a\n\nb</script></h1>").unwrap()
    );
}

#[test]
fn a_serialized_element_follows_its_context() {
    assert_eq!(
        r#"x <em foo="">y</em> z"#,
        convert_faithful("<p>x <em foo>y</em> z</p>").unwrap()
    );
    assert_eq!(
        r#"# <em foo="">y</em>"#,
        convert_faithful("<h1><em foo>y</em></h1>").unwrap()
    );
    assert_eq!(
        r#"*   <em foo="">y</em>"#,
        convert_faithful("<ul><li><em foo>y</em></li></ul>").unwrap()
    );
}
