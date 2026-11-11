use pretty_assertions::assert_eq;
use pulldown_cmark::Options as CommonMarkOptions;

use htmd::{
    Element, HtmlToMarkdown,
    element_handler::{HandlerResult, Handlers},
    options::{Options, TranslationMode},
};

use super::assert_round_trips;
use crate::common::{convert_faithful, convert_faithful_setext, render_markdown, round_trip};

/// `script` and `style` open a
/// [type 1 HTML block](https://spec.commonmark.org/0.31.2/#html-blocks), which
/// runs to the line holding its closing tag: the element passes through whole,
/// blank lines and Markdown specials alike. `div` is type 6, which ends at a
/// blank line instead, so a blank line inside one has to be escaped. A tag
/// outside both lists is type 7, which is written as a raw HTML inline whatever
/// the context.
///
/// The tags a special case of the "Translating HTML nodes" section of
/// `translation_approach.md` sets aside — `iframe`, `xmp`, `noscript`,
/// `noembed`, `noframes` and `plaintext` — are deliberately left uncovered here
/// and in `round_trip_in_an_inline_context`.
/// The tokenizer hands their content back as literal characters, so neither the
/// escape below nor the walk an inline context uses can encode a line ending
/// inside one; this implementation ignores those cases.
#[test]
fn round_trip_in_a_block_context() {
    // Type 1: verbatim, both ways. Without the `<body>`, html5ever files a
    // leading `<script>` or `<style>` under the `<head>`, whose content is
    // dropped.
    assert_eq!(
        "<script>a*b\n\nc</script>",
        round_trip("<body><script>a*b\n\nc</script></body>")
    );
    assert_eq!(
        "<style>a*b\n\nc</style>",
        round_trip("<body><style>a*b\n\nc</style></body>")
    );

    // Type 6: the escaped blank line decodes when an HTML parser reads this
    // output, so the trip is faithful.
    assert_eq!("<div>a*b\n&#10;c</div>", round_trip("<div>a*b\n\nc</div>"));

    // Type 7 is a raw HTML inline, so CommonMark opens a paragraph around it —
    // the tradeoff the "Translating HTML nodes" section of
    // `translation_approach.md` takes deliberately. The `*` survives, escaped
    // as `a\*b`; the blank line does not, a raw HTML inline having its
    // whitespace collapsed.
    assert_eq!(
        "<p><del>a*b c</del></p>\n",
        round_trip("<del>a*b\n\nc</del>")
    );
    // The loss the same section names: `<br><br>` at the root stays `<br><br>`,
    // which reads back inside the paragraph CommonMark opens around it.
    assert_eq!("<p><br><br></p>\n", round_trip("<br><br>"));
    // A lone `<br>` meets the type 7 start condition, so it comes back as the
    // HTML block it opened.
    assert_eq!("<br>", round_trip("<br>"));
}

/// The "Paragraphs" section of `translation_approach.md`: content
/// which opens an HTML block dissolves the paragraph or setext heading holding
/// it, so those two write themselves out whole instead. Each trip below would
/// otherwise come back without its `<p>` or `<h1>`.
#[test]
fn round_trip_of_a_leaf_block_an_html_block_would_dissolve() {
    // A lone type 7 tag.
    assert_eq!("<p><br></p>", round_trip("<p><br></p>"));
    // A type 1-6 tag opening the content. A comment is type 2, `iframe` type 6.
    assert_eq!("<p><!--c--></p>", round_trip("<p><!--c--></p>"));
    assert_eq!("<p><!--c-->a</p>", round_trip("<p><!--c-->a</p>"));
    assert_eq!(
        r#"<p><iframe src="u">a</iframe></p>"#,
        round_trip(r#"<p><iframe src="u">a</iframe></p>"#)
    );
    // A list item's marker and a blockquote's `>` are stripped before the line
    // is scanned, so they protect the paragraph no better than the root does.
    assert_eq!(
        "<blockquote>\n<p><br></p></blockquote>\n",
        round_trip("<blockquote><p><br></p></blockquote>")
    );

    // A second tag keeps the line from being type 7, and a tag which does not
    // open the content cannot start a block at all: both stay paragraphs.
    assert_round_trips("<p><br><br></p>");
    assert_round_trips("<p><br>a</p>");
    assert_round_trips("<p><del><br></del></p>");

    // A setext heading falls back to ATX, whose `#` the block scan matches
    // before any HTML block start condition.
    assert_eq!("<h1><br></h1>\n", round_trip_setext("<h1><br></h1>"));
    assert_eq!(
        "<h1><div>a</div></h1>\n",
        round_trip_setext("<h1><div>a</div></h1>")
    );
    // Content which opens no HTML block keeps the setext form.
    assert_eq!(
        "<h1><br><br></h1>\n",
        round_trip_setext("<h1><br><br></h1>")
    );
}

/// [`round_trip`] with setext headings, the style whose h1 and h2 leave their
/// content at the start of a line.
fn round_trip_setext(html: &str) -> String {
    render_markdown(
        &convert_faithful_setext(html).unwrap(),
        CommonMarkOptions::empty(),
    )
}

/// A setext heading's content opens a line, so content which starts any leaf
/// block dissolves the heading exactly as it dissolves a paragraph. The escape
/// a paragraph puts on such content keeps the setext spelling whole, so the
/// headings section's fallback to ATX is not needed here.
#[test]
fn round_trip_of_a_setext_heading_of_leaf_block_syntax() {
    assert_eq!("<h1>-</h1>\n", round_trip_setext("<h1>-</h1>"));
    assert_eq!("<h2>-</h2>\n", round_trip_setext("<h2>-</h2>"));
    assert_eq!("<h1>---</h1>\n", round_trip_setext("<h1>---</h1>"));
    assert_eq!("<h1>1.</h1>\n", round_trip_setext("<h1>1.</h1>"));
}

/// Step 4 of the headings approach: CommonMark reads a `#` run ending an ATX
/// heading as its closing sequence when it follows a space or tab, so each of
/// these trips would otherwise lose the run.
#[test]
fn round_trip_of_an_atx_heading_ending_in_a_hash_run() {
    assert_round_trips("<h1>a #</h1>");
    assert_round_trips("<h1>a ##</h1>");
    assert_round_trips("<h1><em>a</em> #</h1>");
    assert_round_trips("<h3>#</h3>");
    // The walk collapses the tab to a space.
    assert_eq!("<h1>a #</h1>\n", round_trip("<h1>a\t#</h1>"));
    assert_round_trips(r"<h1>a \#</h1>");

    // These runs follow no space or tab, so they need no escape.
    assert_eq!("# a#", convert_faithful("<h1>a#</h1>").unwrap());
    assert_eq!("# a\u{a0}#", convert_faithful("<h1>a&nbsp;#</h1>").unwrap());
    assert_eq!(r"# a \\#", convert_faithful(r"<h1>a \#</h1>").unwrap());
}

/// Step 4 of the headings approach, for content which is nothing but a `#` run.
/// The generic text escape has already escaped a text node which opens with
/// one, so only a custom handler reaches this case.
#[test]
fn atx_heading_of_a_hash_run_from_a_handler() {
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            translation_mode: TranslationMode::Faithful,
            ..Default::default()
        })
        .add_handler(
            vec!["x-hash"],
            |_: &dyn Handlers, _: &Element| -> Option<HandlerResult> { Some("#".into()) },
        )
        .build();
    let markdown = converter.convert("<h1><x-hash></x-hash></h1>").unwrap();
    assert_eq!(r"# \#", markdown);
    assert_eq!(
        "<h1>#</h1>\n",
        render_markdown(&markdown, CommonMarkOptions::empty())
    );
    assert_eq!(
        r"# \##",
        converter
            .convert("<h1><x-hash></x-hash><x-hash></x-hash></h1>")
            .unwrap()
    );
}

/// A heading is a leaf block, so each element below is written as a raw HTML
/// inline. Only its tags are HTML; what sits between them is CommonMark text,
/// which is what makes the escape of `a\*b` work here — the CommonMark parser
/// consumes the backslash before any HTML parser sees a `<script>` element.
/// `round_trip_losses_of_a_walked_raw_inline` holds the cases where reading
/// that content as text costs something.
#[test]
fn round_trip_in_an_inline_context() {
    // The Markdown special survives; the line ending is collapsed to a space.
    assert_eq!(
        "<h1>x<script>a*b c</script>y</h1>\n",
        round_trip("<h1>x<script>a*b\nc</script>y</h1>")
    );
    assert_eq!(
        "<h1>x<style>a*b c</style>y</h1>\n",
        round_trip("<h1>x<style>a*b\nc</style>y</h1>")
    );
    assert_round_trips("<h1>x<script>a*b*c</script>y</h1>");
    assert_round_trips("<h1>x<style>a*b*c</style>y</h1>");
    assert_round_trips("<h1>x<script>[a](b)</script>y</h1>");

    // `textarea` and `title` are RCDATA rather than raw text: an HTML parser
    // does decode a character reference inside one, so nothing forces their
    // structure to be serialized.
    assert_eq!(
        "<h1>x<textarea>a*b c</textarea>y</h1>\n",
        round_trip("<h1>x<textarea>a*b\nc</textarea>y</h1>")
    );
    assert_eq!(
        "<h1>x<pre>a*b c</pre>y</h1>\n",
        round_trip("<h1>x<pre>a*b\nc</pre>y</h1>")
    );

    assert_round_trips("<h1>x<div>a*b*c</div>y</h1>");
    assert_eq!(
        "<h1>x<div>a*b c</div>y</h1>\n",
        round_trip("<h1>x<div>a*b\nc</div>y</h1>")
    );
    assert_eq!(
        "<h1>x<del>a*b c</del>y</h1>\n",
        round_trip("<h1>x<del>a*b\nc</del>y</h1>")
    );

    // A comment goes the same way.
    assert_eq!(
        "<h1>x<!--a b-->y</h1>\n",
        round_trip("<h1>x<!--a\nb-->y</h1>")
    );
}

/// Every raw HTML inline has its content walked, `<pre>`, `<script>` and
/// `<style>` included, so each of the trips below returns the HTML it started
/// from. `round_trip_losses_of_a_walked_raw_inline` holds the cases which do
/// not.
#[test]
fn round_trip_of_a_walked_raw_inline() {
    // A nested element.
    assert_round_trips("<h1>x<div>a<em>b</em>c</div>y</h1>");
    assert_round_trips("<h1>x<del>a<em>b</em>c</del>y</h1>");
    assert_round_trips("<h1>x<pre>a<em>b</em>c</pre>y</h1>");

    // A literal `<`.
    assert_round_trips("<h1>x<div>a&lt;b&gt;c</div>y</h1>");
    assert_round_trips("<h1>x<del>a&lt;b&gt;c</del>y</h1>");
    assert_round_trips("<h1>x<pre>a&lt;b&gt;c</pre>y</h1>");
    assert_round_trips("<h1>x<textarea>a&lt;b&gt;c</textarea>y</h1>");

    // A literal `&`.
    assert_round_trips("<h1>x<div>a&amp;b</div>y</h1>");
    assert_round_trips("<h1>x<pre>a&amp;b</pre>y</h1>");

    // Markdown specials.
    assert_round_trips("<h1>x<div>a*b_c[d]</div>y</h1>");
    assert_round_trips("<h1>x<pre>a*b_c[d]</pre>y</h1>");
    assert_round_trips("<h1>x<script>a*b_c[d]</script>y</h1>");
    assert_round_trips("<h1>x<style>a*b_c[d]</style>y</h1>");

    // A `<code>` child: a code span where CommonMark allows one, and a raw
    // HTML inline of its own inside a `<pre>`.
    assert_round_trips("<h1>x<div>a<code>c*d</code>b</div>y</h1>");
    assert_round_trips("<h1>x<pre>a<code>c*d</code>b</pre>y</h1>");
    assert_round_trips("<h1><pre><code>a</code></pre></h1>");
    assert_round_trips(r#"<h1>x<div>a<code class="q">c*d</code>b</div>y</h1>"#);

    // A block context is untouched by all of this: this is an HTML block,
    // written as it stands. `round_trip_in_a_block_context` has the rest.
    assert_eq!("<pre>a*b\n\nc</pre>", round_trip("<pre>a*b\n\nc</pre>"));
}

/// The trips of `round_trip_of_a_walked_raw_inline` which lose something. Each
/// assertion holds what actually comes back, and says what went missing.
#[test]
fn round_trip_losses_of_a_walked_raw_inline() {
    // The `<` of a raw text element: CommonMark writes `&lt;`, and an HTML
    // parser leaves that reference alone inside a `<script>`. This is the
    // tradeoff the "Translating HTML nodes" section of
    // `translation_approach.md` takes; the alternative it names is serializing
    // the containing block.
    assert_eq!(
        "<h1>x<script>if(a&lt;b){}</script>y</h1>\n",
        round_trip("<h1>x<script>if(a<b){}</script>y</h1>")
    );
    assert_eq!(
        "<h1>x<script>a&lt;em&gt;b&lt;/em&gt;c</script>y</h1>\n",
        round_trip("<h1>x<script>a<em>b</em>c</script>y</h1>")
    );
    // Its `&` goes the same way. A raw text element decodes no reference, so
    // this `<script>` holds the characters `a&amp;b`; the text node's `&` is
    // escaped, and CommonMark writes it back as `&amp;`.
    assert_eq!(
        "<h1>x<script>a&amp;amp;b</script>y</h1>\n",
        round_trip("<h1>x<script>a&amp;b</script>y</h1>")
    );
    assert_eq!(
        "<h1>x<script>a&amp;lt;b&amp;gt;c</script>y</h1>\n",
        round_trip("<h1>x<script>a&lt;b&gt;c</script>y</h1>")
    );

    // `<b>` and `<i>` have no Markdown of their own, so the walk writes the
    // `<strong>` and `<em>` their Markdown means.
    assert_eq!(
        "<h1><pre><strong>a</strong></pre></h1>\n",
        round_trip("<h1><pre><b>a</b></pre></h1>")
    );

    // A `<textarea>` is RCDATA, so this one holds the characters `a<em>b`.
    assert_eq!(
        "<h1>x<textarea>a&lt;em&gt;b&lt;/em&gt;c</textarea>y</h1>\n",
        round_trip("<h1>x<textarea>a<em>b</em>c</textarea>y</h1>")
    );

    // A code block ends with a line ending its content did not have.
    assert_eq!(
        "<pre><code>a*b\nc\n</code></pre>\n",
        round_trip("<pre><code>a*b\nc</code></pre>")
    );

    // A line ending inside a raw HTML inline is collapsed to a space. No one
    // encoding is decoded in every shape a raw HTML inline takes — a comment
    // and a raw text element decode none at all — which is why the
    // "Translating HTML nodes" section of `translation_approach.md` collapses.
    for (html, expected) in [
        ("<h1>x<div>a\nb</div>y</h1>", "<h1>x<div>a b</div>y</h1>\n"),
        ("<h1>x<pre>a\nb</pre>y</h1>", "<h1>x<pre>a b</pre>y</h1>\n"),
        (
            "<h1>x<script>a\nb</script>y</h1>",
            "<h1>x<script>a b</script>y</h1>\n",
        ),
        (
            "<h1>x<textarea>a\nb</textarea>y</h1>",
            "<h1>x<textarea>a b</textarea>y</h1>\n",
        ),
        (
            "<h1><pre><code>a*b\nc</code></pre></h1>",
            "<h1><pre><code>a*b c</code></pre></h1>\n",
        ),
    ] {
        assert_eq!(expected, round_trip(html), "round trip of {html}");
    }
}
