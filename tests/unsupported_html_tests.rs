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

/// The "Inline elements" table. An emphasis delimiter placed against a raw
/// `<br>` would not flank, so rows 1, 2 and 4 write the element as HTML.
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
/// setext heading leaves its content, so row 1 falls back to ATX.
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

/// The second "Headings" table. An ATX heading's `#` has already opened the
/// line, so a raw `<br>` is safe anywhere in it.
#[test]
fn atx_headings() {
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
