use pretty_assertions::assert_eq;

use crate::common::{convert_faithful, convert_faithful_setext};

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

/// Steps 3 and 4 of the "Headings" approach, for setext. A lone `<br>` would
/// open an HTML block where a setext heading leaves its content, so a heading
/// holding only one falls back to ATX.
#[test]
fn setext_headings() {
    assert_eq!("# <br>", convert_faithful_setext("<h1><br></h1>").unwrap());
    assert_eq!(
        "<br><br><br>\n============",
        convert_faithful_setext("<h1><br><br><br></h1>").unwrap()
    );

    assert_eq!("a #\n===", convert_faithful_setext("<h1>a #</h1>").unwrap());
    assert_eq!("\\#\n==", convert_faithful_setext("<h1>#</h1>").unwrap());
}

/// Step 3 of the "Headings" approach, for empty content. A level 1 or 2 heading
/// holding nothing has no setext encoding: the underline is empty too, so the
/// whole heading would disappear.
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

/// Step 4 of the "Headings" approach, for ATX. An ATX heading's `#` has already
/// opened the line, so a raw `<br>` is safe anywhere in it. A `#` run ending
/// the content would close the heading, so it is escaped: by step 4 in `a #`,
/// and by the generic text escape in `#`, whose run opens a text node. `crate::basic::round_trips_faithful::atx_heading_of_a_hash_run_from_a_handler` covers
/// step 4 for content which is nothing but the run.
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

    assert_eq!(r"# a \#", convert_faithful("<h1>a #</h1>").unwrap());
    assert_eq!(r"# \#", convert_faithful("<h1>#</h1>").unwrap());
}

/// The "Paragraphs" table. Rows 1 and 2 are the section's rule that a paragraph
/// whose content would open an HTML block is serialized whole, since the block
/// would dissolve the `<p>` around it: a leading `<iframe>` opens one, as does
/// a bare `<br>`. A second tag keeps the line from being type 7, so row 3 needs
/// nothing. See also the second row of [`blockquotes`].
#[test]
fn paragraphs() {
    assert_eq!(
        r#"<p><iframe src="u">a</iframe></p>"#,
        convert_faithful(r#"<p><iframe src="u">a</iframe></p>"#).unwrap()
    );
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
        convert_faithful(r#"<p><br><img src="i" alt=""></p>"#).unwrap()
    );
    assert_eq!(
        "![](i)<br>",
        convert_faithful(r#"<p><img src="i" alt=""><br></p>"#).unwrap()
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

/// The "Tables" table. A cell's contents are parsed as inline content, so no
/// HTML block can open inside one.
#[test]
fn tables() {
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

/// The "Tables" section's rule that the entire table is serialized when it
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
