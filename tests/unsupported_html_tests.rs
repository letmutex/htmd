//! One test per table of `unsupported_html.md`, holding that table's rows in
//! the order they are written there.
//!
//! Where a row writes `<br><br>...<br>` — one or more `<br>` elements — it is
//! checked with one and with three, except where the table gives one `<br>` and
//! a run of them a row each.

use pretty_assertions::assert_eq;

mod common;
use common::{convert_faithful, convert_faithful_setext};

/// The "Translating HTML nodes" table: a CommonMark block may only process its
/// content in a block context.
#[test]
fn translating_html_nodes() {
    assert_eq!("# <p>a</p>", convert_faithful("<h1><p>a</p></h1>").unwrap());
}

/// The "Special case for paragraphs" table. See also the first row of
/// [`paragraphs`] and the second row of [`blockquotes`].
#[test]
fn special_case_for_paragraphs() {
    assert_eq!(
        r#"<p><iframe src="u">a</iframe></p>"#,
        convert_faithful(r#"<p><iframe src="u">a</iframe></p>"#).unwrap()
    );
}

/// The "Code" table. A code span's and a fenced code block's content is
/// literal text, so every row writes HTML rather than a break.
#[test]
fn code() {
    assert_eq!(
        "a<code>x<br>y</code>b",
        convert_faithful("<p>a<code>x<br>y</code>b</p>").unwrap()
    );
    assert_eq!(
        "a<code>x<br><br><br>y</code>b",
        convert_faithful("<p>a<code>x<br><br><br>y</code>b</p>").unwrap()
    );

    assert_eq!(
        "a<code><br></code>b",
        convert_faithful("<p>a<code><br></code>b</p>").unwrap()
    );
    assert_eq!(
        "a<code><br><br><br></code>b",
        convert_faithful("<p>a<code><br><br><br></code>b</p>").unwrap()
    );

    assert_eq!(
        "<pre><code>a<br>b</code></pre>",
        convert_faithful("<pre><code>a<br>b</code></pre>").unwrap()
    );
    assert_eq!(
        "<pre><code>a<br><br><br>b</code></pre>",
        convert_faithful("<pre><code>a<br><br><br>b</code></pre>").unwrap()
    );

    assert_eq!(
        "<pre><code><br></code></pre>",
        convert_faithful("<pre><code><br></code></pre>").unwrap()
    );
    assert_eq!(
        "<pre><code><br><br><br></code></pre>",
        convert_faithful("<pre><code><br><br><br></code></pre>").unwrap()
    );
}

/// A code span whose text holds a line ending. The "Code" section's special
/// case — a code span's content is literal text, and no encoding of a break
/// survives there — makes this the same situation as the `<br>` rows of
/// [`code`]: the `<code>` has to go out as a raw HTML inline, whose contents
/// the walk collapses to a single space. Writing the span itself would break
/// the "Translating HTML nodes" rule that no handler may write a bare newline
/// in an inline context.
#[test]
fn code_span_holding_a_line_ending() {
    // An ATX heading is a single line, so any line ending truncates it.
    assert_eq!(
        "# a<code>x y</code>b",
        convert_faithful("<h1>a<code>x\ny</code>b</h1>").unwrap()
    );
    // A setext heading's underline attaches to the line above it.
    assert_eq!(
        "a<code>x y</code>b\n==================",
        convert_faithful_setext("<h1>a<code>x\ny</code>b</h1>").unwrap()
    );

    // A paragraph absorbs a lone line ending, which CommonMark reads as a
    // space; a blank one ends the paragraph instead.
    assert_eq!(
        "a<code>x y</code>b",
        convert_faithful("<p>a<code>x\n\ny</code>b</p>").unwrap()
    );
    assert_eq!(
        "> a<code>x y</code>b",
        convert_faithful("<blockquote><p>a<code>x\n\ny</code>b</p></blockquote>").unwrap()
    );
    // The same span in a block context, where the blank line ends the
    // blockquote rather than the paragraph inside it.
    assert_eq!(
        "> <code>x y</code>",
        convert_faithful("<blockquote><code>x\n\ny</code></blockquote>").unwrap()
    );
}

/// A code span holding nothing. The "Code" section's special case again: the
/// backticks meant to open an empty span close it instead, so CommonMark has no
/// spelling for one and the `<code>` goes out as a raw HTML inline. The walk
/// collapses its contents, which turns the whitespace-only spans below into the
/// single space of the first row.
#[test]
fn empty_code_span() {
    assert_eq!(
        "a<code></code>b",
        convert_faithful("<p>a<code></code>b</p>").unwrap()
    );
    assert_eq!(
        "a<code> </code>b",
        convert_faithful("<p>a<code> </code>b</p>").unwrap()
    );
    // Whitespace-only content reaches the emptiness test only after the trim,
    // so a line ending here is caught as an empty span rather than as the
    // line ending of [`code_span_holding_a_line_ending`].
    assert_eq!(
        "a<code> </code>b",
        convert_faithful("<p>a<code>\n</code>b</p>").unwrap()
    );

    // Pure mode has no fallback and drops the span, rather than write the bare
    // backticks CommonMark reads as literal text.
    assert_eq!(
        "ab",
        htmd::HtmlToMarkdown::new()
            .convert("<p>a<code></code>b</p>")
            .unwrap()
    );
}

/// The "Inline elements" table. An emphasis delimiter placed against a raw
/// `<br>` would not flank, so rows 1, 2 and 4 write the element as HTML; rows 7
/// and 8 do so because they leave no emphasis string to delimit.
#[test]
fn inline_elements() {
    assert_eq!(
        "<em><br>a</em>",
        convert_faithful("<p><em><br>a</em></p>").unwrap()
    );
    assert_eq!(
        "<em><br><br><br>a</em>",
        convert_faithful("<p><em><br><br><br>a</em></p>").unwrap()
    );

    assert_eq!(
        "<em>a<br></em>",
        convert_faithful("<p><em>a<br></em></p>").unwrap()
    );
    assert_eq!(
        "<em>a<br><br><br></em>",
        convert_faithful("<p><em>a<br><br><br></em></p>").unwrap()
    );

    assert_eq!(
        "*a<br>b*",
        convert_faithful("<p><em>a<br>b</em></p>").unwrap()
    );
    assert_eq!(
        "*a<br><br><br>b*",
        convert_faithful("<p><em>a<br><br><br>b</em></p>").unwrap()
    );

    assert_eq!(
        "a<em><br></em>b",
        convert_faithful("<p>a<em><br></em>b</p>").unwrap()
    );
    assert_eq!(
        "a<em><br><br><br></em>b",
        convert_faithful("<p>a<em><br><br><br></em>b</p>").unwrap()
    );

    assert_eq!(
        "a<del><br></del>b",
        convert_faithful("<p>a<del><br></del>b</p>").unwrap()
    );
    assert_eq!(
        "a<del><br><br><br></del>b",
        convert_faithful("<p>a<del><br><br><br></del>b</p>").unwrap()
    );

    assert_eq!(
        "a<span><br></span>b",
        convert_faithful("<p>a<span><br></span>b</p>").unwrap()
    );
    assert_eq!(
        "a<span><br><br><br></span>b",
        convert_faithful("<p>a<span><br><br><br></span>b</p>").unwrap()
    );

    assert_eq!(
        "a<em> </em>b",
        convert_faithful("<p>a<em> </em>b</p>").unwrap()
    );
    assert_eq!(
        "a<strong> </strong>b",
        convert_faithful("<p>a<strong> </strong>b</p>").unwrap()
    );
    // A no-break space is Unicode whitespace too, so it leaves no emphasis
    // string either.
    assert_eq!(
        "a<em>\u{a0}</em>b",
        convert_faithful("<p>a<em>&#160;</em>b</p>").unwrap()
    );

    assert_eq!(
        "a<em></em>b",
        convert_faithful("<p>a<em></em>b</p>").unwrap()
    );
}

/// The "Links" table. A link label has no flanking rule, so every row keeps
/// its CommonMark link.
#[test]
fn links() {
    assert_eq!(
        "a[<br>](u)b",
        convert_faithful(r#"<p>a<a href="u"><br></a>b</p>"#).unwrap()
    );
    assert_eq!(
        "a[<br><br><br>](u)b",
        convert_faithful(r#"<p>a<a href="u"><br><br><br></a>b</p>"#).unwrap()
    );

    assert_eq!(
        "a[<br>c](u)b",
        convert_faithful(r#"<p>a<a href="u"><br>c</a>b</p>"#).unwrap()
    );
    assert_eq!(
        "a[<br><br><br>c](u)b",
        convert_faithful(r#"<p>a<a href="u"><br><br><br>c</a>b</p>"#).unwrap()
    );

    assert_eq!(
        "a[c<br>](u)b",
        convert_faithful(r#"<p>a<a href="u">c<br></a>b</p>"#).unwrap()
    );
    assert_eq!(
        "a[c<br><br><br>](u)b",
        convert_faithful(r#"<p>a<a href="u">c<br><br><br></a>b</p>"#).unwrap()
    );
}

/// The "At the document root" table.
#[test]
fn at_the_document_root() {
    assert_eq!("<br>", convert_faithful("<br>").unwrap());
    assert_eq!("<br><br><br>", convert_faithful("<br><br><br>").unwrap());

    assert_eq!("<br>\n\na", convert_faithful("<br><p>a</p>").unwrap());
    assert_eq!("a\n\n<br>", convert_faithful("<p>a</p><br>").unwrap());

    // `div` is a block-level tag name, so the element is a type 6 HTML block
    // and round-trips verbatim however many `<br>`s it holds.
    assert_eq!(
        "<div><br></div>",
        convert_faithful("<div><br></div>").unwrap()
    );
    assert_eq!(
        "<div><br><br><br></div>",
        convert_faithful("<div><br><br><br></div>").unwrap()
    );
}

/// The first "Headings" table. A lone `<br>` would open an HTML block where a
/// setext heading leaves its content, so the `<br>`-only heading row falls back
/// to ATX.
#[test]
fn setext_headings() {
    assert_eq!("# <br>", convert_faithful_setext("<h1><br></h1>").unwrap());
    assert_eq!(
        "<br><br><br>\n============",
        convert_faithful_setext("<h1><br><br><br></h1>").unwrap()
    );

    assert_eq!(
        "<br>*b*\n=======",
        convert_faithful_setext("<h1><br><em>b</em></h1>").unwrap()
    );
    assert_eq!(
        "*a*<br>\n=======",
        convert_faithful_setext("<h1><em>a</em><br></h1>").unwrap()
    );
}

/// The first "Headings" table's empty row. A level 1 or 2 heading holding
/// nothing has no setext spelling: the underline is empty too, so the whole
/// heading would disappear.
#[test]
fn an_empty_setext_heading_falls_back_to_atx() {
    assert_eq!("#", convert_faithful_setext("<h1></h1>").unwrap());
    assert_eq!("##", convert_faithful_setext("<h2></h2>").unwrap());
    assert_eq!("#", convert_faithful_setext("<h1>   </h1>").unwrap());

    // The heading still has to survive whatever follows it.
    assert_eq!(
        "#\n\na",
        convert_faithful_setext("<h1></h1><p>a</p>").unwrap()
    );
}

/// The second "Headings" table. An ATX heading's `#` has already opened the
/// line, so a raw `<br>` is safe anywhere in it.
#[test]
fn atx_headings() {
    // An empty heading drops the space as well, which would otherwise be
    // trailing whitespace.
    assert_eq!("#", convert_faithful("<h1></h1>").unwrap());
    assert_eq!("#\n\na", convert_faithful("<h1></h1><p>a</p>").unwrap());
    assert_eq!("###\n\na", convert_faithful("<h3></h3><p>a</p>").unwrap());

    assert_eq!("# <br>", convert_faithful("<h1><br></h1>").unwrap());
    assert_eq!(
        "# <br><br><br>",
        convert_faithful("<h1><br><br><br></h1>").unwrap()
    );

    assert_eq!(
        "# <br>*b*",
        convert_faithful("<h1><br><em>b</em></h1>").unwrap()
    );
    assert_eq!(
        "# *a*<br>",
        convert_faithful("<h1><em>a</em><br></h1>").unwrap()
    );
}

/// The "Paragraphs" table. Row 1 is the "Special case for paragraphs" rule: a
/// bare `<br>` re-opens as an HTML block, which would dissolve the `<p>` around
/// it. A second tag keeps the line from being type 7, so row 2 needs nothing.
#[test]
fn paragraphs() {
    assert_eq!("<p><br></p>", convert_faithful("<p><br></p>").unwrap());
    assert_eq!("<br><br>", convert_faithful("<p><br><br></p>").unwrap());
    assert_eq!(
        "<br><br><br>",
        convert_faithful("<p><br><br><br></p>").unwrap()
    );

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
    assert_eq!(
        "![](i)<br>",
        convert_faithful(r#"<p><img src="i"><br></p>"#).unwrap()
    );
}

/// The "Blockquotes" table. Row 2 is the paragraph row one container down: the
/// `>` is stripped before the line is scanned, so the bare tag would re-open as
/// an HTML block exactly as it does at the root.
#[test]
fn blockquotes() {
    assert_eq!(
        "> <br>",
        convert_faithful("<blockquote><br></blockquote>").unwrap()
    );
    assert_eq!(
        "> <br><br><br>",
        convert_faithful("<blockquote><br><br><br></blockquote>").unwrap()
    );

    assert_eq!(
        "> <p><br></p>",
        convert_faithful("<blockquote><p><br></p></blockquote>").unwrap()
    );

    assert_eq!(
        "> <br>*b*",
        convert_faithful("<blockquote><p><br><em>b</em></p></blockquote>").unwrap()
    );
    assert_eq!(
        "> *a*<br>",
        convert_faithful("<blockquote><p><em>a</em><br></p></blockquote>").unwrap()
    );
}

/// The "Table cells" table. A cell's contents are parsed as inline content, so
/// no HTML block can open inside one.
#[test]
fn table_cells() {
    // The one-column, one-row table the rows describe, showing only the cell
    // which holds the `<br>`.
    fn convert_body_cell(cell: &str) -> String {
        let html = format!(
            "<table><thead><tr><th>h</th></tr></thead>\
             <tbody><tr><td>{cell}</td></tr></tbody></table>"
        );
        let markdown = convert_faithful(&html).unwrap();
        markdown.lines().nth(2).unwrap().to_string()
    }

    assert_eq!("| <br>*b* |", convert_body_cell("<br><em>b</em>"));
    assert_eq!("| *a*<br> |", convert_body_cell("<em>a</em><br>"));
    assert_eq!("| <br> |", convert_body_cell("<br>"));
    assert_eq!("| <br><br><br> |", convert_body_cell("<br><br><br>"));
}

/// The "Table cells" section's rule that the entire table is serialized when it
/// holds a child which could only be written as HTML. A cell carrying an
/// attribute is the case that rule names; a row, a section, a `<tfoot>` and a
/// `<colgroup>` are the same kind of child. `table::extract_table_content`
/// walks the table's own sections and rows rather than routing them through
/// `tr_handler` and `table_section_handler`, so it repeats their checks.
#[test]
fn table_children_which_only_html_can_express() {
    for html in [
        // The cell carrying an attribute, the case the rule names.
        "<table><thead><tr><th scope=\"col\">h</th></tr></thead>\
         <tbody><tr><td>a</td></tr></tbody></table>",
        // A body row, a header row, a `<thead>` and a `<tbody>` carrying one.
        "<table><thead><tr><th>h</th></tr></thead>\
         <tbody><tr id=\"r\"><td>a</td></tr></tbody></table>",
        "<table><thead><tr id=\"r\"><th>h</th></tr></thead>\
         <tbody><tr><td>a</td></tr></tbody></table>",
        "<table><thead id=\"t\"><tr><th>h</th></tr></thead>\
         <tbody><tr><td>a</td></tr></tbody></table>",
        "<table><thead><tr><th>h</th></tr></thead>\
         <tbody id=\"b\"><tr><td>a</td></tr></tbody></table>",
        // A GFM table has one body, so a `<tfoot>`'s rows could only join it,
        // losing the footer.
        "<table><thead><tr><th>h</th></tr></thead>\
         <tbody><tr><td>a</td></tr></tbody>\
         <tfoot><tr><td>f</td></tr></tfoot></table>",
        // A `<colgroup>` has no Markdown spelling at all.
        "<table><colgroup><col span=\"2\"></colgroup>\
         <thead><tr><th>h</th></tr></thead>\
         <tbody><tr><td>a</td></tr></tbody></table>",
    ] {
        assert_eq!(html, convert_faithful(html).unwrap());
    }
}
