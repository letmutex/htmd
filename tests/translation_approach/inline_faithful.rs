use pretty_assertions::assert_eq;

use crate::common::{convert_faithful, convert_faithful_setext};

/// The "Translating HTML nodes" table: a CommonMark block may only process its
/// content in a block context.
#[test]
fn translating_html_nodes() {
    assert_eq!("# <p>a</p>", convert_faithful("<h1><p>a</p></h1>").unwrap());
}

/// The "Translating HTML nodes" rule that an inline begins an inline context,
/// applied to every inline which can hold an element: a block inside one goes
/// out as a raw HTML inline, as it does inside a heading. Images, autolinks and
/// hard line breaks hold no elements. GFM strikethrough has no handler, so
/// `<del>` is a raw HTML inline, like `<span>`.
#[test]
fn blocks_inside_inlines() {
    // An inline as HTML, then as the CommonMark expected for it with text at
    // each end of its content and with a block alone; `{}` stands for that
    // content. A raw HTML inline at the edge of an emphasis string would not
    // flank, so the "Emphasis and strong emphasis" approach writes that
    // emphasis as HTML.
    let inlines = [
        (r#"<a href="u">{}</a>"#, "[{}](u)", "[{}](u)"),
        ("<em>{}</em>", "*{}*", "<em>{}</em>"),
        ("<i>{}</i>", "*{}*", "<i>{}</i>"),
        ("<strong>{}</strong>", "**{}**", "<strong>{}</strong>"),
        ("<b>{}</b>", "**{}**", "<b>{}</b>"),
        ("<code>{}</code>", "<code>{}</code>", "<code>{}</code>"),
        ("<del>{}</del>", "<del>{}</del>", "<del>{}</del>"),
        (
            r#"<span class="s">{}</span>"#,
            r#"<span class="s">{}</span>"#,
            r#"<span class="s">{}</span>"#,
        ),
    ];
    // Each is spelled as it serializes, so the table carries the `<tbody>` the
    // HTML parser would otherwise insert.
    let blocks = [
        "<p>b</p>",
        "<h2>b</h2>",
        "<ul><li>b</li></ul>",
        "<ol><li>b</li></ol>",
        "<blockquote>b</blockquote>",
        "<pre>b</pre>",
        "<hr>",
        "<table><tbody><tr><td>b</td></tr></tbody></table>",
        "<div>b</div>",
    ];

    // Every case runs, so a failure lists all of the cases which went wrong.
    let mut failures = Vec::new();
    for (inline, between_text, alone) in inlines {
        for block in blocks {
            let block_between_text = format!("x{block}y");
            let contents = [(block_between_text.as_str(), between_text), (block, alone)];
            for (content, markdown) in contents {
                let html = format!("a{}c", inline.replace("{}", content));
                let expected = format!("a{}c", markdown.replace("{}", content));
                let actual = convert_faithful(&html).unwrap();
                if actual != expected {
                    failures.push((html, expected, actual));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} cases failed, as (HTML in, expected, actual): {failures:#?}",
        failures.len()
    );
}

/// Rows 1-4 of the "Code blocks and code spans" table. A code span's and a
/// fenced code block's content is literal text, so every row writes HTML rather
/// than a break. `crate::basic::serialization_faithful::faithful_mode_keeps_adjacent_code_spans_apart`
/// covers row 5.
#[test]
fn code_blocks_and_code_spans() {
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

/// A code span whose text holds a line ending. A code span's content is literal
/// text, and no encoding of a break survives there (see the "Code blocks and
/// code spans" section), which makes this the same situation as the `<br>` rows
/// of [`code_blocks_and_code_spans`]: the `<code>` has to go out as a raw HTML
/// inline, whose contents the walk collapses to a single space. Writing the
/// span itself would break the "Translating HTML nodes" rule that no handler
/// may write a bare newline in an inline context.
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

/// A code span holding nothing. The backticks meant to open an empty span close
/// it instead, so CommonMark has no spelling for one and the `<code>` goes out
/// as a raw HTML inline. The walk collapses its contents, which turns the
/// whitespace-only spans below into the single space of the first row.
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

/// Rows 1-6 of the "Emphasis and strong emphasis" table. An emphasis delimiter
/// placed against a raw `<br>` would not flank, so rows 1, 2 and 4 write the
/// element as HTML; rows 5 and 6 do so because they leave no emphasis string to
/// delimit. `crate::basic::serialization_faithful::faithful_mode_keeps_adjacent_emphasis_apart` covers
/// row 7.
#[test]
fn emphasis_and_strong_emphasis() {
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

/// `<br>`s alone in an inline which goes out as a raw HTML inline. GFM
/// strikethrough has no handler, so `<del>` is one, like `<span>`.
#[test]
fn brs_alone_in_a_raw_html_inline() {
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

/// `<br>`s in link text, which the "Links" section translates as CommonMark. A
/// link label has no flanking rule, so every case keeps its CommonMark link.
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
