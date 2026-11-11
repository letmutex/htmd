use pretty_assertions::assert_eq;

use crate::common::convert_faithful;

/// Asserts that faithful mode writes `html` back as it stands — the
/// fallback for a table holding what a Markdown one cannot — and that pure
/// mode, which has no fallback, writes `pure_markdown` instead.
fn assert_html_fallback(html: &str, pure_markdown: &str) {
    assert_eq!(html, convert_faithful(html).unwrap());
    assert_eq!(
        pure_markdown,
        htmd::HtmlToMarkdown::new().convert(html).unwrap()
    );
}

/// CommonMark has no caption: writing the content as a paragraph above the
/// table reads as one but is not one, so only pure mode does it.
#[test]
fn test_table_caption_is_commonmark_only_in_pure_mode() {
    let html = concat!(
        "<table><caption>Sample Table</caption>",
        "<thead><tr><th>h</th></tr></thead>",
        "<tbody><tr><td>John</td></tr></tbody></table>"
    );

    assert_eq!(
        "Sample Table\n\n| h    |\n| ---- |\n| John |",
        htmd::HtmlToMarkdown::new().convert(html).unwrap()
    );
    // A `<caption>` is only valid inside a `<table>`, so the whole table is
    // serialized around it rather than the caption written on its own.
    assert_eq!(html, convert_faithful(html).unwrap());
}

/// A GFM table has no headerless form: the delimiter row which makes the
/// block a table has to follow a header row.
#[test]
fn a_table_with_no_header_row_is_written_as_html() {
    let html = concat!(
        "<table><tbody><tr><td>a</td><td>b</td></tr>",
        "<tr><td>c</td><td>d</td></tr></tbody></table>"
    );
    assert_eq!(html, convert_faithful(html).unwrap());

    // A block element in a cell is a raw HTML inline and no reason to
    // serialize; the missing header row is the reason here.
    let block_cell = "<table><tbody><tr><td><p>a</p></td></tr></tbody></table>";
    assert_eq!(block_cell, convert_faithful(block_cell).unwrap());
}

/// Pure mode has no HTML to fall back on, so it writes an empty header row.
/// A table is only built where the markup holds a `th` or a `thead`, hence
/// the empty `<thead>` needed to reach this.
#[test]
fn pure_mode_writes_an_empty_header_row_for_a_headerless_table() {
    let html = "<table><thead></thead><tbody><tr><td>a</td></tr></tbody></table>";

    assert_eq!(
        "|   |\n| - |\n| a |",
        htmd::HtmlToMarkdown::new().convert(html).unwrap()
    );
    assert_eq!(html, convert_faithful(html).unwrap());
}

/// A GFM table has one header row, so only the first `<tr>` of a `<thead>`
/// reaches the Markdown; the rest would be dropped. Pure mode drops the
/// second header row.
#[test]
fn extra_header_rows_are_written_as_html() {
    assert_html_fallback(
        concat!(
            "<table><thead><tr><th>A</th></tr><tr><th>B</th></tr></thead>",
            "<tbody><tr><td>c</td></tr></tbody></table>"
        ),
        "| A |\n| - |\n| c |",
    );
}

/// A GFM row is all header cells or all body cells, so a `<th>` among a body
/// row's `<td>`s would be dropped. Pure mode drops it, which leaves the
/// row's `<td>` in the first column.
#[test]
fn a_header_cell_in_a_body_row_is_written_as_html() {
    assert_html_fallback(
        concat!(
            "<table><tbody><tr><th>A</th><th>B</th></tr>",
            "<tr><th>r</th><td>c</td></tr></tbody></table>"
        ),
        "| A | B |\n| - | - |\n| c |   |",
    );
}

/// A `<colgroup>`, a `<script>` between rows, and a row holding no cell all
/// reach the Markdown table as nothing at all.
#[test]
fn table_content_a_markdown_row_cannot_hold_is_written_as_html() {
    for html in [
        concat!(
            "<table><colgroup><col></colgroup><thead><tr><th>A</th></tr></thead>",
            "<tbody><tr><td>c</td></tr></tbody></table>"
        ),
        concat!(
            "<table><thead><tr><th>A</th></tr></thead>",
            "<tbody><script>x</script><tr><td>c</td></tr></tbody></table>"
        ),
        concat!(
            "<table><thead><tr><th>A</th></tr></thead>",
            "<tbody><tr><td>c</td></tr><tr></tr></tbody></table>"
        ),
    ] {
        assert_html_fallback(html, "| A |\n| - |\n| c |");
    }
}

/// The cells of a GFM header row are header cells, so a `<thead>` of `<td>`
/// reaches the Markdown promoted to `<th>`. Pure mode takes the promotion.
#[test]
fn a_thead_of_data_cells_is_written_as_html() {
    assert_html_fallback(
        concat!(
            "<table><thead><tr><td>A</td><td>B</td></tr></thead>",
            "<tbody><tr><td>c</td><td>d</td></tr></tbody></table>"
        ),
        "| A | B |\n| - | - |\n| c | d |",
    );
}

/// A GFM table has one body, so a second `<tbody>`'s rows join the first and
/// the split between them is lost. Pure mode takes the join.
#[test]
fn a_second_tbody_is_written_as_html() {
    assert_html_fallback(
        concat!(
            "<table><thead><tr><th>h</th></tr></thead>",
            "<tbody><tr><td>a</td></tr></tbody>",
            "<tbody><tr><td>b</td></tr></tbody></table>"
        ),
        "| h |\n| - |\n| a |\n| b |",
    );
}

/// A GFM table has one header row, so a second `<thead>` overwrites the
/// header the first one gave and that header is lost. Pure mode takes the
/// overwrite.
#[test]
fn a_second_thead_is_written_as_html() {
    assert_html_fallback(
        concat!(
            "<table><thead><tr><th>A</th></tr></thead>",
            "<thead><tr><th>B</th></tr></thead>",
            "<tbody><tr><td>c</td></tr></tbody></table>"
        ),
        "| B |\n| - |\n| c |",
    );
}

/// A GFM table's header row comes first, so a `<thead>` following body rows
/// can only become one by moving ahead of the rows it followed. A `<tbody>`
/// opening with a `<th>` row supplies the header just as a `<thead>` does,
/// which leaves the `<thead>` after it the second of two. Pure mode takes
/// the reordering.
#[test]
fn a_thead_after_a_body_row_is_written_as_html() {
    for html in [
        concat!(
            "<table><tbody><tr><td>c</td></tr></tbody>",
            "<thead><tr><th>B</th></tr></thead></table>"
        ),
        concat!(
            "<table><tbody><tr><th>A</th></tr><tr><td>c</td></tr></tbody>",
            "<thead><tr><th>B</th></tr></thead></table>"
        ),
    ] {
        assert_html_fallback(html, "| B |\n| - |\n| c |");
    }
}

/// A `<tbody>` holding no row reaches the Markdown as nothing at all, so the
/// `<thead>` after it would move ahead of a body section the table no longer
/// has. Pure mode takes the reordering, which leaves a table of a header
/// alone.
#[test]
fn a_thead_after_an_empty_tbody_is_written_as_html() {
    assert_html_fallback(
        concat!(
            "<table><tbody></tbody>",
            "<thead><tr><th>B</th></tr></thead></table>"
        ),
        "| B |\n| - |",
    );
}

/// A GFM table's header row comes first, so a row of `<th>` following a body
/// row can only become one by moving ahead of the rows it followed. Pure
/// mode drops the row, as it drops a `<th>` among a body row's `<td>`s; the
/// header it leaves empty is the one
/// `pure_mode_writes_an_empty_header_row_for_a_headerless_table` covers.
#[test]
fn a_header_row_after_a_body_row_is_written_as_html() {
    assert_html_fallback(
        "<table><tbody><tr><td>1</td></tr><tr><th>H</th></tr></tbody></table>",
        "|   |\n| - |\n| 1 |",
    );
}

/// Every row of a GFM table holds the columns its header declares, so a row
/// of a different width gains cells the HTML never held, or gives the table
/// a header column the HTML never declared.
#[test]
fn a_row_wider_or_narrower_than_the_header_is_written_as_html() {
    assert_html_fallback(
        concat!(
            "<table><thead><tr><th>A</th></tr></thead>",
            "<tbody><tr><td>c</td><td>d</td></tr></tbody></table>"
        ),
        "| A |   |\n| - | - |\n| c | d |",
    );
    assert_html_fallback(
        concat!(
            "<table><thead><tr><th>A</th><th>B</th></tr></thead>",
            "<tbody><tr><td>c</td></tr></tbody></table>"
        ),
        "| A | B |\n| - | - |\n| c |   |",
    );
}

/// A Markdown table is built from its cells' content, which leaves a comment
/// between them nowhere to go. Stray text is not the same case: the parser
/// foster-parents it out of the table, ahead of the table's own output.
#[test]
fn a_comment_in_a_table_is_written_as_html() {
    for html in [
        concat!(
            "<table><!-- c --><thead><tr><th>A</th></tr></thead>",
            "<tbody><tr><td>c</td></tr></tbody></table>"
        ),
        concat!(
            "<table><thead><tr><th>A</th></tr></thead>",
            "<tbody><!-- c --><tr><td>c</td></tr></tbody></table>"
        ),
        concat!(
            "<table><thead><tr><th>A</th></tr></thead>",
            "<tbody><tr><!-- c --><td>c</td></tr></tbody></table>"
        ),
    ] {
        assert_html_fallback(html, "| A |\n| - |\n| c |");
    }

    let with_stray_text = concat!(
        "<table><thead><tr><th>A</th></tr></thead>",
        "<tbody>stray<tr><td>c</td></tr></tbody></table>"
    );
    assert_eq!(
        "stray\n\n| A |\n| - |\n| c |",
        convert_faithful(with_stray_text).unwrap()
    );
}

/// A `|` in a cell is escaped as `\|`, but a backslash escape written
/// inside a tag stays the two characters it is. A `|` in a raw HTML
/// inline's open tag is encoded as the character reference the HTML parser
/// reading the result decodes back.
#[test]
fn a_pipe_in_an_open_tag_is_encoded() {
    let in_a_body_cell = concat!(
        "<table><thead><tr><th>A</th></tr></thead>",
        "<tbody><tr><td><span title=\"a|b\">c</span></td></tr></tbody></table>"
    );
    assert_eq!(
        concat!(
            "| A                               |\n",
            "| ------------------------------- |\n",
            "| <span title=\"a&#124;b\">c</span> |"
        ),
        convert_faithful(in_a_body_cell).unwrap()
    );

    let in_a_header_cell = concat!(
        "<table><thead><tr><th><span title=\"a|b\">c</span></th></tr></thead>",
        "<tbody><tr><td>c</td></tr></tbody></table>"
    );
    assert_eq!(
        concat!(
            "| <span title=\"a&#124;b\">c</span> |\n",
            "| ------------------------------- |\n",
            "| c                               |"
        ),
        convert_faithful(in_a_header_cell).unwrap()
    );

    // Pure mode drops the `<span>`, and the attribute with it.
    assert_eq!(
        "| A |\n| - |\n| c |",
        htmd::HtmlToMarkdown::new().convert(in_a_body_cell).unwrap()
    );
}

/// Neither escape for a `|` reaches inside a comment, so a comment holding
/// one takes the whole table to HTML. A nested table's comment is written
/// into the outer row too, so it takes the outer table with it.
#[test]
fn a_comment_holding_a_pipe_is_written_as_html() {
    // Pure mode drops every comment, so none of these reaches a row.
    assert_html_fallback(
        concat!(
            "<table><tbody><tr><th>A</th></tr>",
            "<tr><td><!-- a|b --></td></tr></tbody></table>"
        ),
        "| A |\n| - |\n|   |",
    );
    for html in [
        concat!(
            "<table><tbody><tr><th><!-- a|b --></th></tr>",
            "<tr><td>c</td></tr></tbody></table>"
        ),
        concat!(
            "<table><tbody><tr><th>A</th></tr><tr><td>",
            "<table><tbody><tr><td><!-- a|b --></td></tr></tbody></table>",
            "</td></tr></tbody></table>"
        ),
    ] {
        assert_eq!(html, convert_faithful(html).unwrap());
    }

    // A comment holding no `|` still translates, as
    // `a_comment_in_a_table_is_written_as_html` shows for one outside a
    // cell.
    let without_a_pipe = concat!(
        "<table><thead><tr><th>A</th></tr></thead>",
        "<tbody><tr><td>a<!-- c -->b</td></tr></tbody></table>"
    );
    assert_eq!(
        "| A            |\n| ------------ |\n| a<!-- c -->b |",
        convert_faithful(without_a_pipe).unwrap()
    );
}

/// A GFM table is a header row and the rows under it, so a table holding no
/// row at all is not one. Pure mode, with no fallback, drops it.
#[test]
fn an_empty_table_is_written_as_html() {
    assert_html_fallback("<table></table>", "");
    assert_html_fallback("<table><tbody></tbody></table>", "");
    assert_html_fallback("<table><thead></thead></table>", "");
}
