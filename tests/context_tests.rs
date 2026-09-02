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
    // translated.
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
