use std::{sync::Arc, thread::JoinHandle};

use indoc::indoc;
use pretty_assertions::assert_eq;

use htmd::{
    Element, HtmlToMarkdown,
    element_handler::Handlers,
    options::{BrStyle, HeadingStyle, LinkStyle, Options, TranslationMode},
};
mod common;
use common::{convert, render};

#[test]
fn links_with_spaces() {
    let html = r#"
        <a href="https://example.com/Some Page.html">Example</a>
        "#;
    assert_eq!(
        "[Example](<https://example.com/Some Page.html>)",
        convert(html).unwrap(),
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
        convert(html).unwrap(),
    )
}

#[test]
fn images_with_spaces_in_url() {
    let html = r#"
        <img src="https://example.com/Some Image.jpg" />
        "#;
    assert_eq!(
        "![](<https://example.com/Some Image.jpg>)",
        convert(html).unwrap(),
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
        convert(html).unwrap(),
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
        convert(html).unwrap()
    );
}

#[test]
fn quotes() {
    let html = r#"
        <blockquote>Once upon a time</blockquote>
        "#;
    assert_eq!("> Once upon a time", convert(html).unwrap());
}

#[test]
fn br() {
    let html = r#"
        Hi<br>there<br><br>!"#;
    // The second `<br>` of the pair opens an empty line, where two spaces would
    // be invisible and leave a blank line that ends the paragraph, so it falls
    // back to a backslash break.
    assert_eq!("Hi  \nthere  \n\\\n!", convert(html).unwrap());

    let md = HtmlToMarkdown::builder()
        .options(Options {
            br_style: BrStyle::Backslash,
            translation_mode: TranslationMode::Faithful,
            ..Default::default()
        })
        .build()
        .convert(html)
        .unwrap();
    assert_eq!("Hi\\\nthere\\\n\\\n!", &md);
}

#[test]
fn hr() {
    let html = r#"Hi <hr/> there"#;
    assert_eq!("Hi\n\n* * *\n\nthere", convert(html).unwrap());
}

#[test]
fn strong_italic() {
    let html = r#"<i>Italic</i><em>Also italic</em><strong>Strong</strong><b>Stronger</b>"#;
    assert_eq!(
        "*ItalicAlso italic***StrongStronger**",
        convert(html).unwrap()
    );
}

#[test]
fn italic_inside_word() {
    let html = r#"It<i>al</i>ic St<b>ro</b>ng"#;
    assert_eq!("It*al*ic St**ro**ng", convert(html).unwrap());
}

/// A literal backslash at the end of an emphasis element is written `\\`, whose
/// second backslash sits against the newline that follows — exactly where a
/// backslash hard break's own marker sits. Pure mode moves such a break outside
/// the closing marker, and must not mistake this for one: hoisting one backslash
/// out of the pair leaves the other escaping the closing marker, which loses the
/// emphasis altogether.
#[test]
fn trailing_backslash_is_not_a_hard_break() {
    for html in [
        // A block child is what puts a newline after the backslash pair.
        r"<em>path C:\<div></div></em>",
        r"<strong>path C:\<div></div></strong>",
        // Here a real break follows the literal, so the run is odd and the break
        // does move out — but the pair it sits behind must stay whole.
        r"<em>path C:\<br>x</em>",
    ] {
        for mode in [TranslationMode::Pure, TranslationMode::Faithful] {
            let md = HtmlToMarkdown::builder()
                .options(Options {
                    translation_mode: mode,
                    ..Default::default()
                })
                .build()
                .convert(html)
                .unwrap();
            assert!(
                md.contains(r"C:\\"),
                "{html:?} ({mode:?}) became {md:?}, which split the escaped backslash pair"
            );
        }
    }
}

fn convert_in(html: &str, translation_mode: TranslationMode) -> String {
    HtmlToMarkdown::builder()
        .options(Options {
            translation_mode,
            ..Default::default()
        })
        .build()
        .convert(html)
        .unwrap()
}

// ---------------------------------------------------------------------------
// Emphasis wrapping a *block*.
//
// `emphasis_handler` checks that its markers would sit against characters that
// let them open and close emphasis. CommonMark asks something else as well:
// both markers must land in the same paragraph, since a blank line ends one. A
// block child writes a blank line, so an `<em>` holding one can satisfy the
// flanking rule and still fail to be emphasis — nothing looks. The handler's
// hoists move a blank line at either *edge* outside the markers, which is why a
// block that is the whole content is fine; only an interior block leaves a
// blank line with content on both sides, where nothing can hoist it away.
//
// Whether that reaches the output turns entirely on what encloses the emphasis,
// so both tests run the same `<em>a<div>b</div>c</em>` in every context and
// differ only in what they expect.
// ---------------------------------------------------------------------------

/// The emphasis carries a block child intact here — each for its own reason,
/// none of them the flanking check:
///
/// * In a `<p>`, the HTML parser gets there first. A `<div>` closes an open
///   paragraph, so html5ever reconstructs the `<em>` around each piece and htmd
///   is handed three well-formed elements instead of one.
/// * In a `<div>` under faithful mode, the `<div>` is serialized whole, so the
///   `<em>` inside it never has to write markers at all.
/// * In a table cell, the cell flattens its content onto one line, which puts
///   both markers back in the same paragraph after the fact.
#[test]
fn emphasis_around_a_block_keeps_its_markers() {
    let cell = "<table><thead><tr><th>h</th></tr></thead><tbody><tr>\
                <td>x<em>a<div>b</div>c</em></td></tr></tbody></table>";
    for (html, faithful, pure) in [
        (
            "<p><em>a<div>b</div>c</em></p>",
            "*a*\n\n<div><em>b</em></div>\n\n*c*",
            "*a*\n\n*b*\n\n*c*",
        ),
        (
            "<div><em>a<div>b</div>c</em></div>",
            "<div><em>a<div>b</div>c</em></div>",
            // Pure mode has no HTML to hide behind, so it is the broken case
            // below; only the faithful half belongs here.
            "",
        ),
        (
            cell,
            "| h                     |\n| --------------------- |\n| x*a  <div>b</div>  c* |",
            "| h          |\n| ---------- |\n| x*a  b  c* |",
        ),
    ] {
        let md = convert_in(html, TranslationMode::Faithful);
        assert_eq!(faithful, md);
        // The markers really do pair up: this reads back as emphasis.
        assert!(
            render(&md).contains("<em>"),
            "{html:?} became {md:?}, which reads back with no emphasis"
        );
        if !pure.is_empty() {
            assert_eq!(pure, convert_in(html, TranslationMode::Pure));
        }
    }
}

/// The same emphasis, in the contexts that neither split it nor flatten it. The
/// markers straddle a blank line, so CommonMark reads them as literal asterisks
/// and the `<em>` is lost — `*a\n\nb\n\nc*` reads back as three paragraphs, the
/// first opening with a stray `*` and the last closing with one.
///
/// **This pins a known defect, not intended behavior.** The flanking check
/// should reject these and let faithful mode serialize the element, the way it
/// already does for the `<br>` shapes in `br_tests`; pure mode, which never
/// consults that check, needs its own answer. Fixing either will fail these
/// assertions — that is what they are for. See `emphasis_handler`.
#[test]
fn emphasis_around_a_block_loses_its_markers() {
    for (html, faithful, pure) in [
        // A `<div>` is serialized whole in faithful mode, so only pure mode
        // reaches the emphasis here; the faithful half is asserted above.
        ("<div><em>a<div>b</div>c</em></div>", "", "*a\n\nb\n\nc*"),
        (
            "<ul><li><em>a<div>b</div>c</em></li></ul>",
            "*   *a\n\n    <div>b</div>\n\n    c*",
            "*   *a\n\n    b\n\n    c*",
        ),
        (
            "<blockquote><em>a<div>b</div>c</em></blockquote>",
            "> *a\n> \n> <div>b</div>\n> \n> c*",
            "> *a\n> \n> b\n> \n> c*",
        ),
        (
            "<em>a<div>b</div>c</em>",
            "*a\n\n<div>b</div>\n\nc*",
            "*a\n\nb\n\nc*",
        ),
    ] {
        if !faithful.is_empty() {
            assert_eq!(faithful, convert_in(html, TranslationMode::Faithful));
        }
        assert_eq!(pure, convert_in(html, TranslationMode::Pure));

        // The defect itself: no emphasis survives the round trip.
        for md in [faithful, pure].into_iter().filter(|md| !md.is_empty()) {
            assert!(
                !render(md).contains("<em>"),
                "{html:?} became {md:?}, which now reads back as emphasis — if this was \
                 fixed on purpose, move the shape into the test above"
            );
        }
    }
}

/// A carriage return is a character of the document, not a line ending.
///
/// html5ever normalizes the source's own CR and CRLF to `\n` before htmd sees
/// them, and ordinary text is whitespace-collapsed on top of that, so `&#13;`
/// disappears from a paragraph entirely. Preformatted content is the exception:
/// a `<pre>` carries one through verbatim, which is the one way a `\r` reaches
/// the emphasis handler's leading and trailing hoists.
///
/// Those hoists look for `\n` alone. Every newline htmd writes is one, so a
/// `\r` arriving here is always literal text — counting it as a line ending
/// would cut the hoist short around a run of whitespace that merely contains
/// one.
///
/// What these pin is the output: a carriage return rides through into it and
/// changes nothing else. They do *not* pin the `\n`-only decision itself, which
/// no input reaches through this API — adding `\r` back to the searches leaves
/// every one of these passing. `emphasis::tests::carriage_return_is_not_a_line_ending`
/// pins that, against the hoists directly.
#[test]
fn carriage_return_is_text_not_a_line_ending() {
    // Collapsed out of ordinary text before any of this can matter.
    assert_eq!(
        "x *a*y",
        convert_in("<p>x<em>&#13;a</em>y</p>", TranslationMode::Pure)
    );

    for (html, faithful, pure) in [
        // A `<pre>` keeps the CR, and the emphasis inside it still resolves.
        (
            "<pre><em>&#13;a</em>b</pre>",
            "<pre><em>\ra</em>b</pre>",
            "\r*a*b",
        ),
        (
            "<pre>x<em>a&#13;</em>y</pre>",
            "<pre>x<em>a\r</em>y</pre>",
            "x*a*\ry",
        ),
        // A CR the hoist has to step over on its way out of the element.
        (
            "<em><pre>&#13;</pre>x</em>",
            "*<pre>\r</pre>\n\nx*",
            "\r\n\n*x*",
        ),
        // Two CRLFs — the shape a blank-line test spelled `\n\n` would miss if
        // `\r` counted as a line ending here.
        (
            "<em><pre>&#13;&#10;&#13;&#10;</pre>x</em>",
            "*<pre>\r\n&#13;&#10;</pre>\n\nx*",
            "\r\n\r\n\n*x*",
        ),
    ] {
        assert_eq!(
            faithful,
            convert_in(html, TranslationMode::Faithful),
            "{html:?}"
        );
        assert_eq!(pure, convert_in(html, TranslationMode::Pure), "{html:?}");
    }
}

/// A Setext underline attaches to the whole paragraph above it, not just the
/// last line, so multi-line heading content is fine. Only a blank line — which
/// ends that paragraph — forces ATX.
///
/// ATX is not a safe default to reach for: it is single-line, so content holding
/// a newline loses everything past the first line to a paragraph of its own.
/// Falling back where Setext would have worked breaks the heading rather than
/// protecting it.
#[test]
fn setext_falls_back_to_atx_only_for_a_blank_line() {
    fn setext(html: &str) -> String {
        HtmlToMarkdown::builder()
            .options(Options {
                translation_mode: TranslationMode::Faithful,
                heading_style: HeadingStyle::Setex,
                ..Default::default()
            })
            .build()
            .convert(html)
            .unwrap()
    }

    fn setext_pure(html: &str) -> String {
        HtmlToMarkdown::builder()
            .options(Options {
                translation_mode: TranslationMode::Pure,
                heading_style: HeadingStyle::Setex,
                ..Default::default()
            })
            .build()
            .convert(html)
            .unwrap()
    }

    // A block child writes a blank line, so the underline could not reach the
    // heading's own text. ATX at least keeps that text in a heading.
    assert_eq!(
        "# a\n\n<div>x</div>\n\nb",
        setext("<h1>a<div>x</div>b</h1>")
    );
    // Non-ASCII whitespace is not document whitespace, so it survives the
    // heading's trim and opens the content — which changes nothing here.
    assert_eq!(
        "# \u{a0}\n\n<div>x</div>",
        setext("<h1>&nbsp;<div>x</div></h1>")
    );
    // A line of nothing but spaces is blank too, and a `<pre>` is where one
    // reaches a heading: its whitespace is kept rather than compressed. Setext
    // here would underline `y` alone and leave `x` outside the heading.
    assert_eq!("# x\n \ny", setext_pure("<h1><pre>x\n \ny</pre></h1>"));

    // No blank line: Setext holds, however little the first line carries. Each
    // of these reads back as a single heading.
    for (html, expected) in [
        ("<h1>a<br>b</h1>", "a  \nb\n====="),
        ("<h1>&nbsp;a</h1>", "\u{a0}a\n=="),
        // A raw `<br>` that *opens* the line is the one break shape that must
        // use ATX: it starts an HTML block which eats the underline.
        ("<h1><br>a</h1>", "# <br>a"),
        // ...but one with anything ahead of it is an inline tag, so Setext is
        // still fine.
        ("<h1>a<br></h1>", "a<br>\n====="),
    ] {
        assert_eq!(expected, setext(html), "{html:?}");
    }
    for html in ["<h1>a<br>b</h1>", "<h1>&nbsp;a</h1>", "<h1>a<br></h1>"] {
        let rendered = render(&setext(html));
        assert!(
            rendered.starts_with("<h1>") && rendered.matches("<h1>").count() == 1,
            "{html:?} became {:?}, which reads back as {rendered:?}",
            setext(html)
        );
    }
}

/// `br_handler` decides how to write a `<br>` from the heading's level and the
/// requested style alone, since the content `can_use_setext` judges does not
/// exist until the walk it is part of returns. So it can write a hard break into
/// a heading that then falls back to ATX, where the break ends the heading and
/// leaves the rest to a paragraph — and under `BrStyle::Backslash` leaves a
/// stray `\` in the heading's text as well. `fold_hard_breaks` writes those
/// breaks back the way ATX needs them.
///
/// The invariant is that asking for Setext is never worse than not asking: where
/// Setext cannot be used, the output is the one ATX gives for the same input.
#[test]
fn setext_falling_back_to_atx_rewrites_hard_breaks() {
    fn convert_with(html: &str, heading_style: HeadingStyle, mode: TranslationMode) -> String {
        HtmlToMarkdown::builder()
            .options(Options {
                translation_mode: mode,
                heading_style,
                br_style: BrStyle::Backslash,
                ..Default::default()
            })
            .build()
            .convert(html)
            .unwrap()
    }

    // A block child puts a blank line in the content, which rules Setext out in
    // either mode — after the `<br>` ahead of it has already been written.
    for html in [
        "<h1>a<br>b<div>c</div></h1>",
        "<h2>a<br>b<div>c</div></h2>",
        "<h1>a<br>b<hr></h1>",
        // A break *after* the blank line, which a repair that stopped at the
        // first one would leave behind.
        "<h1>a<div>c</div>d<br>e</h1>",
    ] {
        for mode in [TranslationMode::Pure, TranslationMode::Faithful] {
            assert_eq!(
                convert_with(html, HeadingStyle::Atx, mode),
                convert_with(html, HeadingStyle::Setex, mode),
                "{html:?} ({mode:?}) came out worse under Setex than under Atx"
            );
        }
    }

    // A raw `<br>` opening the first line rules Setext out as well, but only
    // faithful mode writes one: pure mode drops a `<br>` it cannot spell, which
    // leaves the content opening with the `a` after it and Setext usable — so
    // these are only the faithful half of the pair.
    for html in ["<h1><br>a<br>b</h1>", "<h1><a><br></a>a<br>b</h1>"] {
        assert_eq!(
            convert_with(html, HeadingStyle::Atx, TranslationMode::Faithful),
            convert_with(html, HeadingStyle::Setex, TranslationMode::Faithful),
            "{html:?} came out worse under Setex than under Atx"
        );
        // Pure mode keeps the break as a break, which is the better answer of
        // the two and must not be folded away with it.
        assert_eq!(
            "a\\\nb\n====",
            convert_with(html, HeadingStyle::Setex, TranslationMode::Pure),
            "{html:?}"
        );
    }

    // The one shape ATX can hold outright also has to read back as the heading
    // it came from, which is what the stray `\` used to cost.
    for mode in [TranslationMode::Pure, TranslationMode::Faithful] {
        let md = convert_with("<h1><br>a<br>b</h1>", HeadingStyle::Setex, mode);
        let rendered = render(&md);
        assert!(
            rendered.starts_with("<h1>") && rendered.matches("<h1>").count() == 1,
            "({mode:?}) became {md:?}, which reads back as {rendered:?}"
        );
    }

    // An escaped literal backslash is not a break marker, so the newline after
    // it is the block's own and the pair must survive whole.
    let md = convert_with(
        r"<h1>path C:\<div>c</div></h1>",
        HeadingStyle::Setex,
        TranslationMode::Faithful,
    );
    assert!(
        md.contains(r"C:\\"),
        "{md:?} split the escaped backslash pair"
    );
}

#[test]
fn inline_raw_html_escaping() {
    let html = r#"Test &lt;code&gt;tags&lt;/code&gt;, &lt;!-- comments --&gt;, &lt;?processing instructions?&gt;, &lt;!A declaration&gt;, and &lt;![CDATA[character data]]&gt;."#;
    assert_eq!(
        r#"Test \<code>tags\</code>, \<!-- comments -->, \<?processing instructions?>, \<!A declaration>, and <!\[CDATA\[character data\]\]>."#,
        convert(html).unwrap()
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
        convert(html).unwrap()
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
        convert(html).unwrap()
    );
}

#[test]
fn faithful_mode_inline() {
    assert_eq!(
        convert(indoc!(
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
        convert(indoc!(r#"<hr bar>"#)).unwrap(),
        indoc!(r#"<hr bar="">"#)
    );
}

#[test]
fn faithful_mode_blockquote() {
    assert_eq!(
        convert(indoc!(
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
        convert(indoc!(r#"<h1 class="foo">Heading</h1>"#)).unwrap(),
        indoc!(r#"<h1 class="foo">Heading</h1>"#)
    );
}

#[test]
fn faithful_mode_p() {
    assert_eq!(
        convert(indoc!(r#"<p dir="ltr">Test 1</p>"#)).unwrap(),
        indoc!(r#"<p dir="ltr">Test 1</p>"#)
    );
}

#[test]
fn faithful_mode_ol1() {
    assert_eq!(
        convert(indoc!(
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
        convert(indoc!(
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
        convert(indoc!(r#"<!-- Test -->"#)).unwrap(),
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
    let md = convert(html).unwrap();
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
        convert(indoc!(
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

#[test]
fn faithful_mode_nested_inline_html() {
    assert_eq!(
        convert("<p>Nested <foo><bar><em>content</em></bar></foo></p>").unwrap(),
        "Nested <foo><bar>*content*</bar></foo>"
    );
}

#[test]
fn spaces_check() {
    let html = r#"<i>Italic</i> <em>Also italic</em>  <strong>Strong</strong> <b>Stronger </b>"#;
    assert_eq!(
        "*Italic* *Also italic* **Strong** **Stronger**",
        convert(html).unwrap()
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
        convert(html).unwrap()
    );
}

#[test]
fn raw_text() {
    let html = r#"Hello world!"#;
    assert_eq!("Hello world!", convert(html).unwrap());
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
    assert_eq!("# Hello\n\nWorld", convert(html).unwrap());
}

#[test]
fn html_entities() {
    let html = r#"<p><a href="/my%20&amp;uri" title="my%20&amp;title">my%20&amp;link</a></p>"#;
    assert_eq!(
        r#"[my%20&link](/my%20&uri "my%20&title")"#,
        convert(html).unwrap()
    );

    let html_plain = r#"<p>This &amp; that, then &lt; &gt; now.</p>"#;
    assert_eq!(
        r#"This & that, then < > now."#,
        convert(html_plain).unwrap()
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
    assert_eq!("# *A*", convert("<h1><i>A</h1>").unwrap());
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
        convert(r#"<p><span class="math math-inline">x^2</span></p>"#).unwrap()
    );

    assert_eq!(
        "$$x^2$$",
        convert(r#"<p><span class="math math-display">x^2</span></p>"#).unwrap()
    );

    // Test escaping -- values inside math should not be escaped.
    assert_eq!(
        "$${a}_1, b_{2}, a*1, b*2, [a](b), 3 <a> b, a \\; b$$",
        convert(r#"<p><span class="math math-display">{a}_1, b_{2}, a*1, b*2, [a](b), 3 &lt;a&gt; b, a \; b</span></p>"#).unwrap()
    );
}

// Document white space characters don't include non-breaking spaces; these should be preserved.
#[test]
fn document_whitespace() {
    assert_eq!(
        "bar\u{a0}\n\n*   foo\u{a0}",
        convert(indoc!(
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
    let md = convert("<p>2½. Long shot</p>").unwrap();
    assert_eq!(r"2½\. Long shot", md);
}

#[test]
fn multibyte_ordered_list_escape_accented() {
    // e-acute before dot -- not numeric, so the dot is not an ordered list marker
    let md = convert("<p>1é. text</p>").unwrap();
    assert_eq!(r"1é. text", md);
}

#[test]
fn multibyte_ordered_list_escape_trademark() {
    // trademark symbol is not numeric
    let md = convert("<p>3™. text</p>").unwrap();
    assert_eq!(r"3™. text", md);
}

#[test]
fn ascii_ordered_list_escape() {
    let md = convert("<p>10. normal</p>").unwrap();
    assert_eq!(r"10\. normal", md);
}

#[test]
fn multibyte_no_dot() {
    // No dot, should not be affected
    let md = convert("<p>2½</p>").unwrap();
    assert_eq!("2½", md);
}

#[test]
fn cjk_before_ordered_list() {
    // CJK chars are not numeric in Rust's is_numeric(), so this is not an ordered list pattern
    let md = convert("<p>日本語1. test</p>").unwrap();
    assert_eq!(r"日本語1. test", md);
}

#[test]
fn multibyte_atx_heading_escape() {
    let md = convert("<p># héading</p>").unwrap();
    assert_eq!(r"\# héading", md);
}

#[test]
fn multibyte_atx_heading_escape_umlaut() {
    let md = convert("<p>## über</p>").unwrap();
    assert_eq!(r"\## über", md);
}
