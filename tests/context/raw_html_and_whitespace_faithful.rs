use pretty_assertions::assert_eq;

use crate::common::convert_faithful;

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
    // `translation_approach.md` takes.
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
    // See `round_trip_in_an_inline_context` in `tests/basic/round_trips_faithful.rs`.
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
/// "Translating HTML nodes" section of `translation_approach.md`. Encoding the
/// line ending instead would need a parser which decodes a character reference
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

/// A `|` in an open tag is encoded for the same reason as a line ending there.
/// The `\|` which `table::normalize_cell_content` writes for a `|` elsewhere in
/// a cell is a CommonMark escape, and one written inside a tag stays the two
/// characters it is, so the tag keeps a `|` that would split the row.
#[test]
fn a_raw_inline_encodes_the_pipes_of_its_attributes() {
    assert_eq!(
        concat!(
            "| h                            |\n",
            "| ---------------------------- |\n",
            r#"| <em foo="1&#124;2">y\|z</em> |"#
        ),
        convert_faithful(
            "<table><thead><tr><th>h</th></tr></thead>\
             <tbody><tr><td><em foo=\"1|2\">y|z</em></td></tr></tbody></table>"
        )
        .unwrap()
    );
    // Outside a cell the encoding is redundant but harmless: it decodes back to
    // the `|` it replaced. The contents are CommonMark text either way, where
    // `|` needs no escape at all.
    assert_eq!(
        r#"a<em foo="1&#124;2">y|z</em>b"#,
        convert_faithful(r#"<p>a<em foo="1|2">y|z</em>b</p>"#).unwrap()
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
    // A blank line is encoded.
    assert_eq!(
        r#"[t](u "a&#10;&#10;b")"#,
        convert_faithful("<p><a href=\"u\" title=\"a\n\nb\">t</a></p>").unwrap()
    );
    // A carriage return reaches the title only as a character reference, the
    // parser having folded any literal CRLF into a line feed. Each line ending
    // character is encoded where it stands, so the pair comes back intact.
    assert_eq!(
        r#"# [t](u "a&#13;&#10;b")"#,
        convert_faithful("<h1><a href=\"u\" title=\"a&#13;&#10;b\">t</a></h1>").unwrap()
    );
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
