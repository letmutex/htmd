use std::{sync::Arc, thread::JoinHandle};

use indoc::indoc;
use pretty_assertions::assert_eq;
use pulldown_cmark::{Options as CommonMarkOptions, Parser};

use htmd::{
    Element, HtmlToMarkdown,
    element_handler::Handlers,
    options::{LinkStyle, Options, TranslationMode},
};
mod common;
use common::convert_faithful;

#[test]
fn links_with_spaces() {
    let html = r#"
        <a href="https://example.com/Some Page.html">Example</a>
        "#;
    assert_eq!(
        "[Example](<https://example.com/Some Page.html>)",
        convert_faithful(html).unwrap(),
    )
}

#[test]
fn referenced_links_with_title() {
    let html = r#"
        <a href="https://example.com" title="Some title">Example</a>
        "#;
    let md = HtmlToMarkdown::builder()
        .options(Options {
            link_style: LinkStyle::Referenced,
            translation_mode: TranslationMode::Faithful,
            ..Default::default()
        })
        .build()
        .convert(html)
        .unwrap();
    assert_eq!(
        "[Example][1]\n\n[1]: https://example.com \"Some title\"",
        &md
    )
}

#[test]
fn consecutive_referenced_links_with_title() {
    let html = r#"
        <a href="https://example.com" title="Some title">Example</a><a href="https://example.com" title="Some title">Another example</a>
        "#;
    let md = HtmlToMarkdown::builder()
        .options(Options {
            link_style: LinkStyle::Referenced,
            translation_mode: TranslationMode::Faithful,
            ..Default::default()
        })
        .build()
        .convert(html)
        .unwrap();
    assert_eq!(
        indoc!(
            r#"
        [Example][1][Another example][2]

        [1]: https://example.com "Some title"
        [2]: https://example.com "Some title""#
        ),
        &md
    )
}

#[test]
fn images() {
    let html = r#"
        <img src="https://example.com" />
        <img src="https://example.com" alt="Image 1" />
        <img src="https://example.com" alt="Image 2" title="Hello" />
        "#;
    assert_eq!(
        "![](https://example.com) ![Image 1](https://example.com) \
            ![Image 2](https://example.com \"Hello\")",
        convert_faithful(html).unwrap(),
    )
}

#[test]
fn images_with_spaces_in_url() {
    let html = r#"
        <img src="https://example.com/Some Image.jpg" />
        "#;
    assert_eq!(
        "![](<https://example.com/Some Image.jpg>)",
        convert_faithful(html).unwrap(),
    )
}

#[test]
fn image_title_stays_outside_an_angle_bracket_destination() {
    let markdown = htmd::convert(
        r#"<img src="https://example.com/image name.png" alt="diagram" title="A title">"#,
    )
    .unwrap();

    assert_eq!(
        r#"![diagram](<https://example.com/image name.png> "A title")"#,
        markdown
    );
}

#[test]
fn headings() {
    let html = r#"
        <h1>Heading 1</h1>
        <h2>Heading 2</h2>
        <h3>Heading 3</h3>
        <h4>Heading 4</h4>
        <h5>Heading 5</h5>
        <h6>Heading 6</h6>
        "#;
    assert_eq!(
        "# Heading 1\n\n## Heading 2\n\n### Heading 3\n\n\
             #### Heading 4\n\n##### Heading 5\n\n###### Heading 6",
        convert_faithful(html).unwrap(),
    )
}

#[test]
fn paragraphs() {
    let html = r#"
        <p>The first.</p>
        <p>The <span>second.</span></p>
        "#;
    assert_eq!(
        "The first.\n\nThe <span>second.</span>",
        convert_faithful(html).unwrap()
    );
}

#[test]
fn quotes() {
    let html = r#"
        <blockquote>Once upon a time</blockquote>
        "#;
    assert_eq!("> Once upon a time", convert_faithful(html).unwrap());
}

#[test]
fn br() {
    let html = r#"
        <p>Hi<br>there<br><br>!</p>"#;
    assert_eq!("Hi<br>there<br><br>!", convert_faithful(html).unwrap());
}

#[test]
fn hr() {
    let html = r#"Hi <hr/> there"#;
    assert_eq!("Hi\n\n* * *\n\nthere", convert_faithful(html).unwrap());
}

#[test]
fn strong_italic() {
    let html = r#"<i>Italic</i><em>Also italic</em><strong>Strong</strong><b>Stronger</b>"#;
    assert_eq!(
        "*ItalicAlso italic***StrongStronger**",
        convert_faithful(html).unwrap()
    );
}

#[test]
fn italic_inside_word() {
    let html = r#"It<i>al</i>ic St<b>ro</b>ng"#;
    assert_eq!("It*al*ic St**ro**ng", convert_faithful(html).unwrap());
}

#[test]
fn inline_raw_html_escaping() {
    let html = r#"Test &lt;code&gt;tags&lt;/code&gt;, &lt;!-- comments --&gt;, &lt;?processing instructions?&gt;, &lt;!A declaration&gt;, and &lt;![CDATA[character data]]&gt;."#;
    assert_eq!(
        r#"Test \<code>tags\</code>, \<!-- comments -->, \<?processing instructions?>, \<!A declaration>, and <!\[CDATA\[character data\]\]>."#,
        convert_faithful(html).unwrap()
    );
}

#[test]
fn multiline_raw_html_escaping() {
    let html = indoc!(
        r#"
    Test &lt;code&gt;multi-line
    tags&lt;/code&gt;, &lt;!-- multi-line
    comments --&gt;, &lt;?multi-line
    processing instructions?&gt;, &lt;!A multi-line
    declaration&gt;, and &lt;![CDATA[multi-line
    character data]]&gt;.
    "#
    );
    assert_eq!(
        indoc!(
            r#"Test \<code>multi-line tags\</code>, \<!-- multi-line comments -->, \<?multi-line processing instructions?>, \<!A multi-line declaration>, and <!\[CDATA\[multi-line character data\]\]>."#
        ),
        convert_faithful(html).unwrap()
    );
}

#[test]
fn html_escaping() {
    let html = indoc!(
        r#"
        <p>&lt;pre</p>
        <p>&lt;script</p>
        <p>&lt;style</p>
        <p>&lt;textarea</p>
        <p>&lt;address</p>
        <p>&lt;ul</p>
        "#
    );
    assert_eq!(
        indoc!(
            r#"\<pre

            \<script

            \<style

            \<textarea

            \<address

            \<ul"#
        ),
        convert_faithful(html).unwrap()
    );
}

#[test]
fn faithful_mode_inline() {
    assert_eq!(
        convert_faithful(indoc!(
            r#"<p>
                <img src="one.png" alt="yyy" title="zzz" scale="50%">
                <em bar>Testing</em>
                <strong foo>Testing</strong>
                <a href="http://foo.com" bar>link</a>
                <code class="not-a-language">code</code>
                <br foo>
            </p>"#
        ))
        .unwrap(),
        indoc!(
            r#"<img src="one.png" alt="yyy" title="zzz" scale="50%"> <em bar="">Testing</em> <strong foo="">Testing</strong> <a href="http://foo.com" bar="">link</a> <code class="not-a-language">code</code> <br foo="">"#
        )
    );
}

#[test]
fn faithful_mode_hr() {
    assert_eq!(
        convert_faithful(indoc!(r#"<hr bar>"#)).unwrap(),
        indoc!(r#"<hr bar="">"#)
    );
}

#[test]
fn faithful_mode_blockquote() {
    assert_eq!(
        convert_faithful(indoc!(
            r#"<blockquote style="foo">
            <em>Testing</em>

            <blockquote>Nested</blockquote>
        </blockquote>"#
        ))
        .unwrap(),
        indoc!(
            r#"<blockquote style="foo">
                <em>Testing</em>
            &#10;    <blockquote>Nested</blockquote>
            </blockquote>"#
        )
    );
}

#[test]
fn faithful_mode_h1() {
    assert_eq!(
        convert_faithful(indoc!(r#"<h1 class="foo">Heading</h1>"#)).unwrap(),
        indoc!(r#"<h1 class="foo">Heading</h1>"#)
    );
}

#[test]
fn faithful_mode_p() {
    assert_eq!(
        convert_faithful(indoc!(r#"<p dir="ltr">Test 1</p>"#)).unwrap(),
        indoc!(r#"<p dir="ltr">Test 1</p>"#)
    );
}

#[test]
fn faithful_mode_ol1() {
    assert_eq!(
        convert_faithful(indoc!(
            r#"<ol>
            <li>Test 1</li>
            <li foo>Test 2</li>
            <li>Test 3</li>
        </ol>"#
        ))
        .unwrap(),
        indoc!(
            r#"<ol>
                <li>Test 1</li>
                <li foo="">Test 2</li>
                <li>Test 3</li>
            </ol>"#
        )
    );
}

#[test]
fn faithful_mode_ol2() {
    assert_eq!(
        convert_faithful(indoc!(
            r#"<ol foo>
            <li>Test</li>
        </ol>"#
        ))
        .unwrap(),
        indoc!(
            r#"<ol foo="">
                <li>Test</li>
            </ol>"#
        )
    );
}

#[test]
fn faithful_mode_comment() {
    assert_eq!(
        convert_faithful(indoc!(r#"<!-- Test -->"#)).unwrap(),
        indoc!(r#"<!-- Test -->"#)
    );
}

#[test]
fn faithful_mode_html() {
    let html = indoc!(
        r#"<details>
            <summary>Test

                1</summary>
            Test 2
        </details>"#
    );
    let md = convert_faithful(html).unwrap();
    assert_eq!(
        indoc!(
            r#"<details>
                <summary>Test
            &#10;        1</summary>
                Test 2
            </details>"#
        ),
        md
    );
}

#[test]
fn faithful_mode_table() {
    assert_eq!(
        convert_faithful(indoc!(
            r#"<table>
            <tr>
                <th>Header 1</th>
                <th>Header 2</th>
            </tr>
            <tr>
                <td foo>Cell 1</td>
                <td>Cell 2</td>
            </tr>
            <tr>
                <td>Cell 3</td>
                <td>Cell 4</td>
            </tr>
        </table>
"#
        ))
        .unwrap(),
        indoc!(
            r#"<table>
            <tbody><tr>
                <th>Header 1</th>
                <th>Header 2</th>
            </tr>
            <tr>
                <td foo="">Cell 1</td>
                <td>Cell 2</td>
            </tr>
            <tr>
                <td>Cell 3</td>
                <td>Cell 4</td>
            </tr>
        </tbody></table>"#
        )
    );
}

/// CommonMark has no caption, so faithful mode always writes the element as
/// HTML — and a `<caption>` is only valid inside a `<table>`, so the whole table
/// is serialized around it.
#[test]
fn faithful_mode_serializes_a_table_with_a_caption() {
    let html = concat!(
        r#"<table><caption><span class="label">Caption</span></caption>"#,
        "<tr><th>Header</th></tr><tr><td>Cell</td></tr></table>"
    );
    // html5ever inserts the `<tbody>` around the rows.
    let expected = concat!(
        r#"<table><caption><span class="label">Caption</span></caption>"#,
        "<tbody><tr><th>Header</th></tr><tr><td>Cell</td></tr></tbody></table>"
    );

    assert_eq!(expected, convert_faithful(html).unwrap());
    // The point of serializing the whole table: the `<caption>` comes back as
    // an element rather than as the bare text an HTML block outside the table
    // would give. The block is the whole document here, so pulldown-cmark
    // writes it without the trailing line ending `assert_round_trips` expects.
    assert_eq!(expected, round_trip(expected));
}

/// Pure mode has no HTML to fall back on, so it keeps writing the caption as a
/// paragraph above the table.
#[test]
fn pure_mode_writes_a_caption_as_a_paragraph() {
    let html = concat!(
        "<table><caption>Caption</caption>",
        "<tr><th>Header</th></tr><tr><td>Cell</td></tr></table>"
    );

    assert_eq!(
        "Caption\n| Header |\n| ------ |\n| Cell   |",
        htmd::convert(html).unwrap()
    );
}

#[test]
fn faithful_mode_nested_inline_html() {
    assert_eq!(
        convert_faithful("<p>Nested <foo><bar><em>content</em></bar></foo></p>").unwrap(),
        "Nested <foo><bar>*content*</bar></foo>"
    );
}

#[test]
fn spaces_check() {
    let html = r#"<i>Italic</i> <em>Also italic</em>  <strong>Strong</strong> <b>Stronger </b>"#;
    assert_eq!(
        "*Italic* *Also italic* **Strong** **Stronger**",
        convert_faithful(html).unwrap()
    );
}

#[test]
fn consecutive_blocks() {
    let html = r#"<p>One</p><p>Two</p>"#;
    assert_eq!(
        indoc!(
            "
        One

        Two"
        ),
        convert_faithful(html).unwrap()
    );
}

#[test]
fn raw_text() {
    let html = r#"Hello world!"#;
    assert_eq!("Hello world!", convert_faithful(html).unwrap());
}

#[test]
fn nested_divs() {
    let html = r#"
    <div>
        <div>
            <div>Hi</div>
        </div>
        <div></div>
        <div>there</div>
    </div>
    "#;
    assert_eq!("Hi\n\nthere", htmd::convert(html).unwrap());
}

#[test]
fn with_head() {
    let html = r#"
    <html>
        <head>
            <title>Demo</title>
            <script>console.log('Hello');</script>
            <style>body {}</style>
        </head>
        <body>
            Content
        </body>
    </html>
    "#;
    assert_eq!(
        "Demo\n\nconsole.log('Hello');\n\nbody {}\n\nContent",
        htmd::convert(html).unwrap()
    );
}

#[test]
fn with_custom_rules() {
    // Remove element
    let html = r#"<img src="https://example.com"/>"#;
    let md = HtmlToMarkdown::builder()
        .add_handler(vec!["img"], |_: &dyn Handlers, _element: Element| None)
        .build()
        .convert(html)
        .unwrap();
    assert_eq!("", &md);
}

#[test]
fn with_custom_rules_and_fallback() {
    let html = r#"<img src="https://example.com"/>"#;
    let converter = HtmlToMarkdown::builder()
        .add_handler(vec!["img"], |handlers: &dyn Handlers, element: Element| {
            if element
                .attrs
                .iter()
                .any(|attr| &attr.name.local == "id" && attr.value.as_ref() == "do_not_skip_me")
            {
                handlers.fallback(element)
            } else {
                None
            }
        })
        .options(Options {
            ..Default::default()
        })
        .build();
    assert_eq!("", &converter.convert(html).unwrap());

    let html = r#"<img src="https://example.com" id="do_not_skip_me"/>"#;
    assert_eq!(
        "![](https://example.com)",
        &converter.convert(html).unwrap()
    );
}

#[test]
fn upper_case_tags() {
    let html = r#"<H1>Hello</H1> <P>World</P>"#;
    assert_eq!("# Hello\n\nWorld", convert_faithful(html).unwrap());
}

#[test]
fn html_entities() {
    let html = r#"<p><a href="/my%20&amp;uri" title="my%20&amp;title">my%20&amp;link</a></p>"#;
    assert_eq!(
        r#"[my%20&link](/my%20&uri "my%20&title")"#,
        convert_faithful(html).unwrap()
    );

    let html_plain = r#"<p>This &amp; that, then &lt; &gt; now.</p>"#;
    assert_eq!(
        r#"This & that, then < > now."#,
        convert_faithful(html_plain).unwrap()
    );
}

#[test]
fn scripting_option() {
    let html = r#"<noscript><p>Hello</p></noscript>"#;
    let md = HtmlToMarkdown::builder()
        .scripting_enabled(true)
        .build()
        .convert(html)
        .unwrap();
    assert_eq!(r#"\<p>Hello\</p>"#, md);

    let md = HtmlToMarkdown::builder()
        .scripting_enabled(false)
        .build()
        .convert(html)
        .unwrap();
    assert_eq!("Hello", md);
}

#[test]
fn multithreading() {
    let html = r#"<a href="https://example.com">Example</a>
    <a href="https://example.com">Example</a>
    <a href="https://example.com">Example</a>
    <a href="https://example.com">Example</a>
    <a href="https://example.com">Example</a>
    "#;
    let expected = "[Example][1] [Example][2] [Example][3] [Example][4] [Example][5]\n\n\
    [1]: https://example.com\n[2]: https://example.com\n[3]: https://example.com\n\
    [4]: https://example.com\n[5]: https://example.com";
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            // We use a global vec to store all referenced links of the doc in
            // the anchor element handler, this is unsafe for multithreading
            // usage if we do nothing
            link_style: LinkStyle::Referenced,
            translation_mode: TranslationMode::Faithful,
            ..Default::default()
        })
        .build();
    let converter = Arc::new(converter);
    let mut handlers: Vec<JoinHandle<()>> = vec![];
    for _ in 0..20 {
        let converter_clone = converter.clone();
        let handle = std::thread::spawn(move || {
            let md = converter_clone.convert(html).unwrap();
            assert_eq!(expected, md);
        });
        handlers.push(handle);
    }
    for handle in handlers {
        handle.join().unwrap();
    }
}

#[test]
fn unterminated_html() {
    // The `<i>` tag isn't terminated. Make sure the conversion still works.
    assert_eq!("# *A*", convert_faithful("<h1><i>A</h1>").unwrap());
}

#[test]
fn misnested_formatting_does_not_duplicate_or_lose_text() {
    let markdown = htmd::convert("<p><b>one<i>two</b>three</i>four").unwrap();

    assert_eq!("**one*two****three*four", markdown);
}

#[test]
fn math() {
    assert_eq!(
        "$x^2$",
        convert_faithful(r#"<p><span class="math math-inline">x^2</span></p>"#).unwrap()
    );

    assert_eq!(
        "$$x^2$$",
        convert_faithful(r#"<p><span class="math math-display">x^2</span></p>"#).unwrap()
    );

    // Test escaping -- values inside math should not be escaped.
    assert_eq!(
        "$${a}_1, b_{2}, a*1, b*2, [a](b), 3 <a> b, a \\; b$$",
        convert_faithful(r#"<p><span class="math math-display">{a}_1, b_{2}, a*1, b*2, [a](b), 3 &lt;a&gt; b, a \; b</span></p>"#).unwrap()
    );
}

// Document white space characters don't include non-breaking spaces; these should be preserved.
#[test]
fn document_whitespace() {
    assert_eq!(
        "bar\u{a0}\n\n*   foo\u{a0}",
        convert_faithful(indoc!(
            "
            <p>bar&nbsp;</p>
            <ul>
              <li>foo&nbsp;</li>
            </ul>
            "
        ))
        .unwrap()
    );
}

// Multi-byte UTF-8 characters before a markdown ordered list dot must not
// cause a panic due to byte/char index confusion in escape_text.
#[test]
fn multibyte_ordered_list_escape_half() {
    // U+00BD (½) is 2 bytes in UTF-8
    let md = convert_faithful("<p>2½. Long shot</p>").unwrap();
    assert_eq!(r"2½\. Long shot", md);
}

#[test]
fn multibyte_ordered_list_escape_accented() {
    // e-acute before dot -- not numeric, so the dot is not an ordered list marker
    let md = convert_faithful("<p>1é. text</p>").unwrap();
    assert_eq!(r"1é. text", md);
}

#[test]
fn multibyte_ordered_list_escape_trademark() {
    // trademark symbol is not numeric
    let md = convert_faithful("<p>3™. text</p>").unwrap();
    assert_eq!(r"3™. text", md);
}

#[test]
fn ascii_ordered_list_escape() {
    let md = convert_faithful("<p>10. normal</p>").unwrap();
    assert_eq!(r"10\. normal", md);
}

#[test]
fn multibyte_no_dot() {
    // No dot, should not be affected
    let md = convert_faithful("<p>2½</p>").unwrap();
    assert_eq!("2½", md);
}

#[test]
fn cjk_before_ordered_list() {
    // CJK chars are not numeric in Rust's is_numeric(), so this is not an ordered list pattern
    let md = convert_faithful("<p>日本語1. test</p>").unwrap();
    assert_eq!(r"日本語1. test", md);
}

#[test]
fn multibyte_atx_heading_escape() {
    let md = convert_faithful("<p># héading</p>").unwrap();
    assert_eq!(r"\# héading", md);
}

#[test]
fn multibyte_atx_heading_escape_umlaut() {
    let md = convert_faithful("<p>## über</p>").unwrap();
    assert_eq!(r"\## über", md);
}

/// Takes `html` back to HTML the long way round: `convert_faithful` writes the
/// Markdown, and pulldown-cmark reads that Markdown back.
///
/// What comes back is HTML *source*, not a DOM, so a character reference in it
/// is decoded only later, by whatever HTML parser reads the result. That is why
/// several assertions below still hold a `&#10;`: it decodes to the line ending
/// it replaced, so the trip is faithful even though the strings differ. Where a
/// trip loses something instead, the assertion says what.
fn round_trip(html: &str) -> String {
    let markdown = convert_faithful(html).unwrap();
    let parser = Parser::new_ext(&markdown, CommonMarkOptions::empty());
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);
    html_output
}

/// `script` and `style` open a
/// [type 1 HTML block](https://spec.commonmark.org/0.31.2/#html-blocks), which
/// runs to the line holding its closing tag: the element passes through whole,
/// blank lines and Markdown specials alike. `div` is type 6, which ends at a
/// blank line instead, so a blank line inside one has to be escaped. A tag
/// outside both lists is type 7, which is written as a raw HTML inline whatever
/// the context.
///
/// The tags the third "Special case" of `unsupported_html.md` sets aside —
/// `iframe`, `xmp`, `noscript`, `noembed`, `noframes` and `plaintext` — are
/// deliberately left uncovered here and in `round_trip_in_an_inline_context`.
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
    // the tradeoff the "Translating HTML nodes" section of `unsupported_html.md`
    // takes deliberately. The `*` survives, escaped as `a\*b`; the blank line
    // does not, a raw HTML inline having its whitespace collapsed.
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

/// Asserts that `html` comes back as itself. pulldown-cmark ends a block with
/// a line ending, which is the only difference these trips are allowed.
fn assert_round_trips(html: &str) {
    assert_eq!(
        format!("{html}\n"),
        round_trip(html),
        "round trip of {html}"
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
    // A raw text element decodes no reference, so this `<script>` holds the
    // eleven characters `a&lt;b&gt;c` — and those are what comes back.
    assert_round_trips("<h1>x<script>a&lt;b&gt;c</script>y</h1>");

    // A literal `&`.
    assert_round_trips("<h1>x<div>a&amp;b</div>y</h1>");
    assert_round_trips("<h1>x<pre>a&amp;b</pre>y</h1>");
    assert_round_trips("<h1>x<script>a&amp;b</script>y</h1>");

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
    // tradeoff the "Translating HTML nodes" section of `unsupported_html.md`
    // takes; the alternative it names is serializing the containing block.
    assert_eq!(
        "<h1>x<script>if(a&lt;b){}</script>y</h1>\n",
        round_trip("<h1>x<script>if(a<b){}</script>y</h1>")
    );
    assert_eq!(
        "<h1>x<script>a&lt;em&gt;b&lt;/em&gt;c</script>y</h1>\n",
        round_trip("<h1>x<script>a<em>b</em>c</script>y</h1>")
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
    // "Translating HTML nodes" section of `unsupported_html.md` collapses.
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
