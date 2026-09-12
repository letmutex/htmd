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
    // A block element in a cell is a raw HTML inline like any other.
    assert_eq!(
        "| h        |\n| -------- |\n| <p>a</p> |",
        convert_faithful(
            "<table><thead><tr><th>h</th></tr></thead>\
             <tbody><tr><td><p>a</p></td></tr></tbody></table>"
        )
        .unwrap()
    );
    assert_eq!(
        "| h                   |\n| ------------------- |\n| <ul><li>a</li></ul> |",
        convert_faithful(
            "<table><thead><tr><th>h</th></tr></thead>\
             <tbody><tr><td><ul><li>a</li></ul></td></tr></tbody></table>"
        )
        .unwrap()
    );
}

/// What a cell cannot absorb as a raw HTML inline is an attribute on the cell
/// itself: a GFM row has nowhere to write one. The `<caption>` half of the same
/// rule is covered by
/// `basic_tests::faithful_mode_serializes_a_table_with_a_caption`. See the
/// table cells section of `unsupported_html.md`.
#[test]
fn a_cell_attribute_writes_the_whole_table_as_html() {
    let with_colspan = "<table><thead><tr><th>h</th></tr></thead>\
                        <tbody><tr><td colspan=\"2\">a</td></tr></tbody></table>";
    assert_eq!(with_colspan, convert_faithful(with_colspan).unwrap());
    let heading_colspan = "<table><thead><tr><th colspan=\"2\">h</th></tr></thead>\
                           <tbody><tr><td>a</td></tr></tbody></table>";
    assert_eq!(heading_colspan, convert_faithful(heading_colspan).unwrap());
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
    // Only the tags of a raw HTML inline are HTML: a `<pre>` written as one
    // holds CommonMark, translated like any other inline content.
    assert_eq!(
        "# <pre>**a**</pre>",
        convert_faithful("<h1><pre><b>a</b></pre></h1>").unwrap()
    );
    assert_eq!(
        r#"# <pre class="x">**a**</pre>"#,
        convert_faithful(r#"<h1><pre class="x"><b>a</b></pre></h1>"#).unwrap()
    );
    // A `<code>` is the exception: a code block is a CommonMark block, so in
    // an inline context it is a raw HTML inline of its own.
    assert_eq!(
        r"# <pre><code>a\*b c</code></pre>",
        convert_faithful("<h1><pre><code>a*b\nc</code></pre></h1>").unwrap()
    );
    // An inline `<code>` outside a `<pre>` is a code span, whose content is
    // literal and so takes no escape.
    assert_eq!(
        "# <div>a`c*d`b</div>",
        convert_faithful("<h1><div>a<code>c*d</code>b</div></h1>").unwrap()
    );
}

/// A raw text element holds literal characters rather than markup, so its
/// structure is serialized as it stands. The Markdown specials of that text are
/// still escaped, a CommonMark parser reading what sits between the tags of a
/// raw HTML inline as text.
#[test]
fn a_raw_text_element_escapes_the_markdown_it_holds() {
    assert_eq!(
        r"# <script>a\*b\_c\[d\]</script>",
        convert_faithful("<h1><script>a*b_c[d]</script></h1>").unwrap()
    );
    // The `<` escape buys nothing here: CommonMark writes `&lt;`, and an HTML
    // parser decodes no character reference inside a raw text element. That is
    // the tradeoff the "Translating HTML nodes" section of
    // `unsupported_html.md` takes.
    assert_eq!(
        r"# <script>if(a\<b){}</script>",
        convert_faithful("<h1><script>if(a<b){}</script></h1>").unwrap()
    );
    assert_eq!(
        r"x<script>a\*b</script>y",
        convert_faithful("<p>x<script>a*b</script>y</p>").unwrap()
    );
    // An HTML block holds no CommonMark to escape, so it goes out as it
    // stands. Without the `<body>`, html5ever files a leading `<script>` under
    // the `<head>`, whose content is dropped.
    assert_eq!(
        "<script>a*b\nc</script>",
        convert_faithful("<body><script>a*b\nc</script></body>").unwrap()
    );
    // See `round_trip_in_an_inline_context` in `basic_tests.rs`.
    assert_eq!(
        "# <script>a b</script>",
        convert_faithful("<h1><script>a\nb</script></h1>").unwrap()
    );
}

/// `textarea` and `title` hold no markup either, but they are RCDATA rather
/// than raw text, so nothing forces their structure to be serialized and they
/// take the ordinary raw HTML inline path instead.
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
    assert_eq!(
        "# <textarea>a b</textarea>",
        convert_faithful("<h1><textarea>a\nb</textarea></h1>").unwrap()
    );
    assert_eq!(
        "<textarea>a*b\nc</textarea>",
        convert_faithful("<textarea>a*b\nc</textarea>").unwrap()
    );
}

/// "All raw HTML inlines will have whitespace collapsed in their contents
/// (including newlines, which would otherwise be problematic)" — the
/// "Translating HTML nodes" section of `unsupported_html.md`. Encoding the line
/// ending instead would need a parser which decodes a character reference
/// there, and neither a comment nor a raw text element has one.
#[test]
fn a_raw_inline_collapses_the_whitespace_of_its_content() {
    // Type 6.
    assert_eq!(
        "# x<div>a b</div>y",
        convert_faithful("<h1>x<div>a\nb</div>y</h1>").unwrap()
    );
    // Type 7.
    assert_eq!(
        "a<del> b </del>c",
        convert_faithful("<p>a<del>\n  b\n</del>c</p>").unwrap()
    );
    // Type 1, whose whitespace carries the meaning of the element — the loss
    // this collapse takes on.
    assert_eq!(
        "# <pre>a b</pre>",
        convert_faithful("<h1><pre>a\n  b</pre></h1>").unwrap()
    );
    // Ordinary flow content collapses the same way.
    assert_eq!("a b", convert_faithful("<p>a\nb</p>").unwrap());
    assert_eq!("# a b", convert_faithful("<h1>a\n  b</h1>").unwrap());
}

/// That collapse is scoped to the contents; an attribute value keeps its
/// whitespace. A bare line ending there would end the leaf block holding the
/// element, so that alone is encoded — the open tag passes through CommonMark
/// verbatim, and the HTML parser reading the result decodes it back.
#[test]
fn a_raw_inline_escapes_the_line_endings_of_its_attributes() {
    assert_eq!(
        r#"a<em foo="1&#10;&#10;2">y</em>b"#,
        convert_faithful("<p>a<em foo=\"1\n\n2\">y</em>b</p>").unwrap()
    );
    // Whitespace which is not a line ending is left alone.
    assert_eq!(
        r#"# a<div title="x  y">z w</div>b"#,
        convert_faithful("<h1>a<div title=\"x  y\">z  w</div>b</h1>").unwrap()
    );
    // A raw HTML inline nested in another one keeps its attribute value too.
    assert_eq!(
        r#"# a<div title="p  q"><em foo="r  s">t</em></div>b"#,
        convert_faithful(r#"<h1>a<div title="p  q"><em foo="r  s">t</em></div>b</h1>"#).unwrap()
    );
    assert_eq!(
        r#"# a<div title="p&#10;q"><em foo="r&#10;s">t</em></div>b"#,
        convert_faithful("<h1>a<div title=\"p\nq\"><em foo=\"r\ns\">t</em></div>b</h1>").unwrap()
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
    // An HTML *block* keeps its line structure: only a blank line ends a type 6
    // one, so only that is encoded.
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
    // Without the `<body>`, html5ever files a leading `<script>` or `<style>`
    // under the `<head>`, whose content is dropped.
    assert_eq!(
        "<script>a\n\nb</script>",
        convert_faithful("<body><script>a\n\nb</script></body>").unwrap()
    );
    assert_eq!(
        "<style>a\n\nb</style>",
        convert_faithful("<body><style>a\n\nb</style></body>").unwrap()
    );
    assert_eq!(
        "<pre>a\n\nb</pre>",
        convert_faithful("<pre>a\n\nb</pre>").unwrap()
    );
    assert_eq!(
        "<textarea>a\n\nb</textarea>",
        convert_faithful("<textarea>a\n\nb</textarea>").unwrap()
    );
    // In an inline context the same element is a raw HTML inline, whose blank
    // lines are collapsed with the rest of its whitespace.
    assert_eq!(
        "# <script>a b</script>",
        convert_faithful("<h1><script>a\n\nb</script></h1>").unwrap()
    );
}

/// A link title and an image's alt text are inline content, and a line ending
/// there ends the leaf block holding it just as one in a raw HTML inline does:
/// written literally, `# [t](u "a⏎b")` is the heading `[t](u "a` followed by
/// the paragraph `b")`. CommonMark recognizes character references in both, so
/// each line ending is encoded instead.
#[test]
fn a_title_escapes_its_line_endings() {
    assert_eq!(
        r#"# [t](u "a&#10;b")"#,
        convert_faithful("<h1><a href=\"u\" title=\"a\nb\">t</a></h1>").unwrap()
    );
    assert_eq!(
        r#"# ![a&#10;b](i "c&#10;d")"#,
        convert_faithful("<h1><img src=\"i\" alt=\"a\nb\" title=\"c\nd\"></h1>").unwrap()
    );
    // A blank line is dropped rather than encoded: the lines around it are
    // joined by the one reference the surviving line break needs.
    assert_eq!(
        r#"[t](u "a&#10;b")"#,
        convert_faithful("<p><a href=\"u\" title=\"a\n\nb\">t</a></p>").unwrap()
    );
    // A carriage return reaches the title only as a character reference, the
    // parser having folded any literal CRLF into a line feed. `lines()` splits
    // a CRLF, so that pair normalizes to the one reference a line break needs...
    assert_eq!(
        r#"# [t](u "a&#10;b")"#,
        convert_faithful("<h1><a href=\"u\" title=\"a&#13;&#10;b\">t</a></h1>").unwrap()
    );
    // ...while a lone carriage return is no line break to `lines()` and so is
    // encoded where it stands.
    assert_eq!(
        r#"# [t](u "a&#13;b")"#,
        convert_faithful("<h1><a href=\"u\" title=\"a&#13;b\">t</a></h1>").unwrap()
    );
    assert_eq!(
        concat!(
            "| h                |\n",
            "| ---------------- |\n",
            "| [t](u \"a&#10;b\") |"
        ),
        convert_faithful(
            "<table><thead><tr><th>h</th></tr></thead>\
             <tbody><tr><td><a href=\"u\" title=\"a\nb\">t</a></td></tr></tbody></table>"
        )
        .unwrap()
    );
}

/// A comment is a type 2 HTML block, which likewise ends at its own `-->`. In
/// an inline context it is a raw HTML inline instead, whose whitespace is
/// collapsed: no parser decodes a character reference inside a comment, so
/// encoding the line ending would be no use.
#[test]
fn a_comment_follows_its_context() {
    assert_eq!("<!--a\n\nb-->", convert_faithful("<!--a\n\nb-->").unwrap());
    assert_eq!(
        "<div>a</div>\n\n<!--c-->\n\n<div>b</div>",
        convert_faithful("<div>a</div><!--c--><div>b</div>").unwrap()
    );
    assert_eq!(
        "> <!--c-->",
        convert_faithful("<blockquote><!--c--></blockquote>").unwrap()
    );
    assert_eq!(
        "# x<!--a b-->y",
        convert_faithful("<h1>x<!--a\nb-->y</h1>").unwrap()
    );
    assert_eq!(
        "x<!--a b-->y",
        convert_faithful("<p>x<!--a\n\nb-->y</p>").unwrap()
    );
    assert_eq!(
        "# x<!--a b-->y",
        convert_faithful("<h1>x<!--a\r\nb-->y</h1>").unwrap()
    );
    // A comment inside a raw HTML inline is a comment still.
    assert_eq!(
        "# <div>x<!--a b-->y</div>",
        convert_faithful("<h1><div>x<!--a\nb-->y</div></h1>").unwrap()
    );
}

/// The html5ever tokenizer hands a CDATA section and a processing instruction
/// back as comments, so both take the type 2 path above rather than the type 5
/// and type 3 ones they were written as.
#[test]
fn a_cdata_section_and_a_processing_instruction_are_comments() {
    assert_eq!(
        "<!--[CDATA[x\n\ny]]-->",
        convert_faithful("<![CDATA[x\n\ny]]>").unwrap()
    );
    assert_eq!(
        "a<!--[CDATA[x y]]-->b",
        convert_faithful("<p>a<![CDATA[x\ny]]>b</p>").unwrap()
    );
    assert_eq!(
        "a<!--?php echo \"x y\"; ?-->b",
        convert_faithful("<p>a<?php echo \"x\ny\"; ?>b</p>").unwrap()
    );
}

/// What a raw HTML inline holds is an inline context as well, so the rule of
/// `a_commonmark_block_in_an_inline_context_is_a_raw_inline` holds one element
/// deeper.
#[test]
fn a_commonmark_block_inside_a_raw_inline_is_a_raw_inline() {
    assert_eq!(
        "# <div><blockquote>a</blockquote></div>",
        convert_faithful("<h1><div><blockquote>a</blockquote></div></h1>").unwrap()
    );
    assert_eq!(
        "# <div><ul><li>a</li></ul></div>",
        convert_faithful("<h1><div><ul><li>a</li></ul></div></h1>").unwrap()
    );
    assert_eq!(
        "# <div><h2>a</h2></div>",
        convert_faithful("<h1><div><h2>a</h2></div></h1>").unwrap()
    );
    assert_eq!(
        "# <div><hr></div>",
        convert_faithful("<h1><div><hr></div></h1>").unwrap()
    );
    // What CommonMark can write inline is still translated there.
    assert_eq!(
        "# <div>a*b*c</div>",
        convert_faithful("<h1><div>a<em>b</em>c</div></h1>").unwrap()
    );
    assert_eq!(
        r"# <div>a[l\*m](u)b</div>",
        convert_faithful(r#"<h1><div>a<a href="u">l*m</a>b</div></h1>"#).unwrap()
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

/// The contents of a math span are literal text, but a line ending there would
/// end the span and begin another block. LaTeX ignores whitespace, so each line
/// ending becomes a space.
#[test]
fn a_math_span_replaces_its_line_endings_with_spaces() {
    assert_eq!(
        "x$a b$y",
        convert_faithful("<p>x<span class=\"math math-inline\">a\nb</span>y</p>").unwrap()
    );
    assert_eq!(
        "$$a b$$",
        convert_faithful("<p><span class=\"math math-display\">a\nb</span></p>").unwrap()
    );
    // A carriage return reaches the span only as a character reference, the
    // parser having folded any literal CRLF into a line feed. A CRLF pair is
    // one line ending, so it becomes one space...
    assert_eq!(
        "$a b$",
        convert_faithful("<p><span class=\"math math-inline\">a&#13;&#10;b</span></p>").unwrap()
    );
    // ...while a lone carriage return is a line ending of its own.
    assert_eq!(
        "$a b$",
        convert_faithful("<p><span class=\"math math-inline\">a&#13;b</span></p>").unwrap()
    );
}
