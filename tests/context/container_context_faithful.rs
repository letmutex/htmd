use pretty_assertions::assert_eq;

use crate::common::convert_faithful;

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
        convert_faithful(r#"<p><br><img src="i" alt=""></p>"#).unwrap()
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
    // Row 1 of "Lists" in `translation_approach.md` wants two HTML blocks here,
    // but `br` is a type 7 tag, so the run stays inline.
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
/// `basic::serialization_faithful::faithful_mode_serializes_a_table_with_a_caption`. See the
/// tables section of `translation_approach.md`.
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
