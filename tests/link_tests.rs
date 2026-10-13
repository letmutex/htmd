use htmd::{
    HtmlToMarkdown,
    options::{LinkStyle, Options, TranslationMode},
};
mod common;
use common::{
    convert_faithful, convert_faithful_options, convert_pure, render_markdown, round_trip,
};
use pulldown_cmark::Options as CommonMarkOptions;

#[test]
fn links() {
    let html = r#"
        <a href="https://example.com">Link 1</a>
        <a href="https://example.com" title="Hello">Link 2</a>
        "#;
    assert_eq!(
        "[Link 1](https://example.com) [Link 2](https://example.com \"Hello\")",
        convert_faithful(html).unwrap(),
    );
}

#[test]
fn links_with_spaces_in_destination_and_title() {
    assert_eq!(
        r#"[Link](<https://example.com/hello world> "Hello")"#,
        convert_faithful(r#"<a href="https://example.com/hello world" title="Hello">Link</a>"#)
            .unwrap(),
    );
}

#[test]
fn links_with_spaces_around_text() {
    assert_eq!(
        "[ bla ](/)",
        convert_faithful(r#"<a href="/"> bla </a>"#).unwrap()
    );
    assert_eq!(
        "Some [ random ](/) text",
        convert_faithful(r#"Some <a href="/"> random </a> text"#).unwrap()
    )
}

#[test]
fn links_inlined_prefer_autolinks() {
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            translation_mode: TranslationMode::Faithful,
            link_style: LinkStyle::InlinedPreferAutolinks,
            ..Default::default()
        })
        .build();

    let html = r#"<a href="https://example.com">https://example.com</a>"#;
    assert_eq!("<https://example.com>", converter.convert(html).unwrap());

    let html = r#"<a href="https://example.com">Link</a>"#;
    assert_eq!(
        "[Link](https://example.com)",
        converter.convert(html).unwrap()
    );

    let html = r#"<a href="https://example.com" title="https://example.com">Link</a>"#;
    assert_eq!(
        r#"[Link](https://example.com "https://example.com")"#,
        converter.convert(html).unwrap()
    );
}

/// A line ending in a link destination ends the leaf block holding it: written
/// literally, `a[t](u⏎⏎v)b` is two paragraphs and no link at all. CommonMark
/// decodes a character reference in a destination, so it is encoded instead.
#[test]
fn a_link_destination_escapes_its_line_endings() {
    assert_eq!(
        "a[t](u&#10;v)b",
        convert_faithful("<p>a<a href=\"u\nv\">t</a>b</p>").unwrap()
    );
    assert_eq!(
        "a[t](u&#10;&#10;v)b",
        convert_faithful("<p>a<a href=\"u\n\nv\">t</a>b</p>").unwrap()
    );
    assert_eq!(
        "# a[t](u&#10;v)b",
        convert_faithful("<h1>a<a href=\"u\nv\">t</a>b</h1>").unwrap()
    );
    // A carriage return reaches the destination only as a character reference,
    // the parser having folded any literal CRLF into a line feed.
    assert_eq!(
        "a[t](u&#13;v)b",
        convert_faithful("<p>a<a href=\"u&#13;v\">t</a>b</p>").unwrap()
    );
    // An image's destination takes the same encoding.
    assert_eq!(
        "a![](i&#10;j)b",
        convert_faithful("<p>a<img src=\"i\nj\">b</p>").unwrap()
    );
    assert_eq!(
        "[t](u)",
        convert_faithful(r#"<p><a href="u">t</a></p>"#).unwrap()
    );
    assert_eq!(
        r"[t](u\(v\))",
        convert_faithful(r#"<p><a href="u(v)">t</a></p>"#).unwrap()
    );
}

#[test]
fn nested_convert_with_referenced_links() {
    use htmd::Element;
    use htmd::element_handler::Handlers;
    use indoc::indoc;

    let converter = HtmlToMarkdown::builder()
        .options(Options {
            link_style: LinkStyle::Referenced,
            ..Default::default()
        })
        .add_handler(
            vec!["widget"],
            |_handlers: &dyn Handlers, _element: &Element| {
                let inner_converter = HtmlToMarkdown::builder()
                    .options(Options {
                        link_style: LinkStyle::Referenced,
                        ..Default::default()
                    })
                    .build();
                let inner_md = inner_converter
                    .convert(r#"<a href="https://inner.com">Inner</a>"#)
                    .unwrap();
                Some(inner_md.into())
            },
        )
        .build();

    let html = indoc!(
        r#"
        <p><a href="https://outer1.com">Outer 1</a></p>
        <widget></widget>
        <p><a href="https://outer2.com">Outer 2</a></p>
        "#
    );
    let md = converter.convert(html).unwrap();

    assert_eq!(
        indoc!(
            r#"
            [Outer 1][1]

            [Inner][1]

            [1]: https://inner.com

            [Outer 2][2]

            [1]: https://outer1.com
            [2]: https://outer2.com"#
        ),
        md
    );
}

#[test]
fn discard_links_in_table_falling_back_to_raw_html() {
    use indoc::indoc;

    let converter = HtmlToMarkdown::builder()
        .options(Options {
            link_style: LinkStyle::Referenced,
            translation_mode: TranslationMode::Faithful,
            ..Default::default()
        })
        .build();

    let html = indoc!(
        r#"
        <table>
            <tr>
                <td><a href="https://discarded.com">Discarded</a></td>
                <td><custom-unsupported>test</custom-unsupported></td>
            </tr>
        </table>
        <p><a href="https://kept.com">Kept</a></p>
        "#
    );
    let md = converter.convert(html).unwrap();

    assert_eq!(
        indoc!(
            r#"
            <table>
                <tbody><tr>
                    <td><a href="https://discarded.com">Discarded</a></td>
                    <td><custom-unsupported>test</custom-unsupported></td>
                </tr>
            </tbody></table>

            [Kept][1]

            [1]: https://kept.com"#
        ),
        md
    );
}

/// A backslash in a destination escapes the character which follows it, the
/// `)` which closes the destination included: written literally, `[x](a\)`
/// reads back as the text `[x](a)` and holds no link at all. The destination's
/// own backslashes are escaped so that the delimiters htmd writes are the only
/// ones the parser sees.
#[test]
fn a_link_destination_escapes_its_backslashes() {
    assert_eq!(
        r"[x](a\\)",
        convert_faithful(r#"<a href="a\">x</a>"#).unwrap()
    );
    assert_eq!(
        r"![q](i\\)",
        convert_faithful(r#"<img src="i\" alt="q">"#).unwrap()
    );
    // A destination holding a space closes with a `>` instead, which a
    // trailing backslash escapes the same way.
    assert_eq!(
        r"[x](<a b\\>)",
        convert_faithful(r#"<a href="a b\">x</a>"#).unwrap()
    );
    // A backslash before a paren is text the destination holds, not the escape
    // htmd writes for the paren, so it needs an escape of its own.
    assert_eq!(
        r"[x](a\\\(b)",
        convert_faithful(r#"<a href="a\(b">x</a>"#).unwrap()
    );
    assert_eq!(
        "<p><a href=\"a%5C\">x</a></p>\n",
        round_trip(r#"<a href="a\">x</a>"#)
    );
}

/// A backslash in a title escapes the character which follows it, the `"`
/// which closes the title included: written literally, `[t](x "a\")` reads
/// back as text and holds no link at all.
#[test]
fn a_link_title_escapes_its_backslashes() {
    assert_eq!(
        r#"[t](x "a\\")"#,
        convert_faithful(r#"<a href="x" title="a\">t</a>"#).unwrap()
    );
    // A title is a plain string rather than inline content, so the `*` needs
    // no escape -- but the backslash before it would be read as one, and the
    // title would come back a character short.
    assert_eq!(
        r#"[t](x "a\\*b")"#,
        convert_faithful(r#"<a href="x" title="a\*b">t</a>"#).unwrap()
    );
    assert_eq!(
        r#"![q](i "a\\")"#,
        convert_faithful(r#"<img src="i" alt="q" title="a\">"#).unwrap()
    );
}

/// An image's alt text lands in a
/// [link label](https://spec.commonmark.org/0.31.2/#link-label), which is
/// inline content: an unescaped `]` closes the label early, so written
/// literally `![a](y)b](x)` is an image of `y` with the alt text `a`, followed
/// by the text `b](x)`. The label reads its other specials too, which an alt
/// of `a*b*c` would come back emphasized by.
#[test]
fn an_image_alt_escapes_its_markdown_specials() {
    assert_eq!(
        r"![a\](y)b](x)",
        convert_faithful(r#"<img src="x" alt="a](y)b">"#).unwrap()
    );
    assert_eq!(
        "<p><img src=\"x\" alt=\"a](y)b\" /></p>\n",
        round_trip(r#"<img src="x" alt="a](y)b">"#)
    );
    // An unmatched `[` opens a label the parser never closes.
    assert_eq!(
        r"![a\[b](x)",
        convert_faithful(r#"<img src="x" alt="a[b">"#).unwrap()
    );
    assert_eq!(
        r"![a\\b\*c\_d\`e](x)",
        convert_faithful(r#"<img src="x" alt="a\b*c_d`e">"#).unwrap()
    );
}

/// An `alt` attribute holds text, but the label it lands in is inline content,
/// where a tag is a [raw HTML](https://spec.commonmark.org/0.31.2/#raw-html)
/// inline. Raw HTML is literal, so the backslashes escaping the Markdown
/// specials inside a tag are never removed: without the `<` escape,
/// `![a<b c="\*">d](x)` comes back with the alt text `a<b c="\*">d`. A text
/// node takes the same escape, from `escape_html`, once its Markdown specials
/// are escaped.
#[test]
fn an_image_alt_escapes_its_raw_html() {
    assert_eq!(
        r#"![a\<b c="\*">d](x)"#,
        convert_faithful(r#"<img src="x" alt='a<b c="*">d'>"#).unwrap()
    );
    assert_eq!(
        "<p><img src=\"x\" alt=\"a&lt;b c=&quot;*&quot;&gt;d\" /></p>\n",
        round_trip(r#"<img src="x" alt='a<b c="*">d'>"#)
    );
    // A `<` which opens no tag is text in either reading, so it needs no
    // backslash -- `leaves_non_html_sequences` in `src/util/escape.rs` holds
    // the rest of that line.
    assert_eq!(
        "![a < b](x)",
        convert_faithful(r#"<img src="x" alt="a < b">"#).unwrap()
    );
}

/// `img_handler` writes a destination holding a space in the
/// [pointy bracket form](https://spec.commonmark.org/0.31.2/#link-destination),
/// where a `<` or `>` in the destination itself has to be escaped: written
/// literally, `![q](<a b>c>)` ends the destination at the first `>`, which
/// leaves `c>)` behind as text.
#[test]
fn a_bracketed_link_destination_escapes_its_angle_brackets() {
    assert_eq!(
        r"![q](<a b\>c>)",
        convert_faithful(r#"<img src="a b>c" alt="q">"#).unwrap()
    );
    assert_eq!(
        r"![q](<a b\<c>)",
        convert_faithful(r#"<img src="a b<c" alt="q">"#).unwrap()
    );
    // pulldown-cmark percent-encodes a destination holding a space, so the
    // trip shows `a b>c` as `a%20b%3Ec` rather than as itself.
    assert_eq!(
        "<p><img src=\"a%20b%3Ec\" alt=\"q\" /></p>\n",
        round_trip(r#"<img src="a b>c" alt="q">"#)
    );
}

/// An [autolink](https://spec.commonmark.org/0.31.2/#autolinks) takes what
/// sits between its angle brackets as the destination itself, so it is spelled
/// from the unescaped href: a backslash there is a character of the URL rather
/// than an escape.
#[test]
fn an_autolink_spells_its_destination_without_escapes() {
    assert_eq!(
        r"<https://example.com/a\b>",
        convert_autolinks(r#"<a href="https://example.com/a\b">https://example.com/a\b</a>"#)
    );
    // Text-node escapes don't prevent an autolink.
    assert_eq!(
        "<https://example.com/a_b*c>",
        convert_autolinks(r#"<a href="https://example.com/a_b*c">https://example.com/a_b*c</a>"#)
    );
    assert_eq!(
        "<p><a href=\"https://example.com/a_b*c\">https://example.com/a_b*c</a></p>\n",
        render_markdown(
            &convert_autolinks(
                r#"<a href="https://example.com/a_b*c">https://example.com/a_b*c</a>"#
            ),
            CommonMarkOptions::empty()
        )
    );
}

/// The conditions for an autolink are listed in the links section of
/// `overall_translation_approach.md`; failing any one gives the inline form.
#[test]
fn an_autolink_needs_every_condition() {
    // No autolink can hold a `<`.
    assert_eq!(
        r"[https://example.com/a\<b](https://example.com/a<b)",
        convert_autolinks(r#"<a href="https://example.com/a<b">https://example.com/a&lt;b</a>"#)
    );
    // An autolink would write a `&` as `&amp;`.
    assert_eq!(
        r"[https://example.com/?a\&b](https://example.com/?a\&b)",
        convert_autolinks(
            r#"<a href="https://example.com/?a&amp;b">https://example.com/?a&amp;b</a>"#
        )
    );
    // An autolink can't hold a title.
    assert_eq!(
        r#"[https://example.com](https://example.com "t")"#,
        convert_autolinks(r#"<a href="https://example.com" title="t">https://example.com</a>"#)
    );
    // The content must be a single text node.
    assert_eq!(
        "[*https://example.com*](https://example.com)",
        convert_autolinks(r#"<a href="https://example.com"><em>https://example.com</em></a>"#)
    );
    // A relative href is no absolute URI: written as an autolink it would go
    // out as raw HTML.
    assert_eq!("[a/b](a/b)", convert_autolinks(r#"<a href="a/b">a/b</a>"#));
    // A one-letter scheme is too short.
    assert_eq!("[a:b](a:b)", convert_autolinks(r#"<a href="a:b">a:b</a>"#));
}

/// A
/// [link reference definition](https://spec.commonmark.org/0.31.2/#link-reference-definition)
/// takes the destination an inline link takes, so
/// `AnchorElementHandler::build_referenced_anchor` writes one holding a space
/// in the same pointy bracket form `build_inlined_anchor` writes. Written bare
/// the line opens no definition, so it would stay in the output as a paragraph
/// of text and the reference pointing at it would find nothing.
#[test]
fn a_referenced_link_destination_brackets_its_spaces() {
    assert_eq!(
        "[t][1]\n\n[1]: <a b>",
        convert_referenced(r#"<a href="a b">t</a>"#)
    );
}

/// A `!` immediately before a link would make it an image.
#[test]
fn a_bang_before_a_link_is_escaped() {
    assert_eq!(
        r"Hi\![t](x)",
        convert_faithful(r#"<p>Hi!<a href="x">t</a></p>"#).unwrap()
    );
    assert_eq!(
        "<p>Hi!<a href=\"x\">t</a></p>\n",
        round_trip(r#"<p>Hi!<a href="x">t</a></p>"#)
    );
    assert_eq!(
        r"Hi\![t][1]",
        convert_referenced(r#"<p>Hi!<a href="x">t</a></p>"#)
            .lines()
            .next()
            .unwrap()
    );
    // The link needn't be the `!`'s sibling.
    assert_eq!(
        r"Hi\![t](x)",
        convert_pure(r#"<p>Hi!<span><a href="x">t</a></span></p>"#).unwrap()
    );
    // A `\` before the `!` is text, escaped by its own backslash.
    assert_eq!(
        r"\\\![t](x)",
        convert_faithful(r#"<p>\!<a href="x">t</a></p>"#).unwrap()
    );
    // Anything between the two leaves the `!` alone.
    assert_eq!(
        "Hi! [t](x)",
        convert_faithful(r#"<p>Hi! <a href="x">t</a></p>"#).unwrap()
    );
    assert_eq!(
        "![a](i)![b](j)",
        convert_faithful(r#"<p><img src="i" alt="a"><img src="j" alt="b"></p>"#).unwrap()
    );
}

/// Every `&` in link text, a destination, a title or an image description is
/// escaped, so that `&amp;copy;` comes back as the text `&copy;` rather than
/// the character it names.
#[test]
fn a_link_escapes_its_ampersands() {
    assert_eq!(
        r#"[x\&copy;](a\&copy;b "t\&copy;")"#,
        convert_faithful(r#"<a href="a&amp;copy;b" title="t&amp;copy;">x&amp;copy;</a>"#).unwrap()
    );
    assert_eq!(
        "<p><a href=\"a&amp;copy;b\" title=\"t&amp;copy;\">x&amp;copy;</a></p>\n",
        round_trip(r#"<a href="a&amp;copy;b" title="t&amp;copy;">x&amp;copy;</a>"#)
    );
    assert_eq!(
        r#"![a\&copy;](i\&copy; "t\&copy;")"#,
        convert_faithful(r#"<img src="i&amp;copy;" alt="a&amp;copy;" title="t&amp;copy;">"#)
            .unwrap()
    );
    assert_eq!(
        "<p><img src=\"i&amp;copy;\" alt=\"a&amp;copy;\" title=\"t&amp;copy;\" /></p>\n",
        round_trip(r#"<img src="i&amp;copy;" alt="a&amp;copy;" title="t&amp;copy;">"#)
    );
    assert_eq!(
        r"[t][1]

[1]: a\&b",
        convert_referenced(r#"<a href="a&amp;b">t</a>"#)
    );
}

/// A destination takes the pointy bracket form when it is empty, starts with a
/// `<`, or holds a space or ASCII control character. Only that form escapes a
/// `<` or `>`.
#[test]
fn a_link_destination_takes_pointy_brackets_where_needed() {
    assert_eq!("[t](<>)", convert_faithful(r#"<a href="">t</a>"#).unwrap());
    assert_eq!(
        "<p><a href=\"\">t</a></p>\n",
        round_trip(r#"<a href="">t</a>"#)
    );
    // A bare empty destination opens no link reference definition.
    assert_eq!(
        "[t][1]\n\n[1]: <>",
        convert_referenced(r#"<a href="">t</a>"#)
    );
    assert_eq!(
        r"[t](<\<a>)",
        convert_faithful(r#"<a href="&lt;a">t</a>"#).unwrap()
    );
    assert_eq!(
        "<p><a href=\"%3Ca\">t</a></p>\n",
        round_trip(r#"<a href="&lt;a">t</a>"#)
    );
    assert_eq!(
        "[t](<a\tb>)",
        convert_faithful("<a href=\"a\tb\">t</a>").unwrap()
    );
    assert_eq!(
        "<p><a href=\"a%09b\">t</a></p>\n",
        round_trip("<a href=\"a\tb\">t</a>")
    );
    assert_eq!(
        "[t](a<b>c)",
        convert_faithful(r#"<a href="a&lt;b&gt;c">t</a>"#).unwrap()
    );
    assert_eq!(
        "<p><a href=\"a%3Cb%3Ec\">t</a></p>\n",
        round_trip(r#"<a href="a&lt;b&gt;c">t</a>"#)
    );
    assert_eq!(
        "![q](<>)",
        convert_faithful(r#"<img src="" alt="q">"#).unwrap()
    );
}

/// A line ending in link text would end a heading or table cell holding the
/// link, and a blank line would end any paragraph, so each is encoded.
#[test]
fn link_text_encodes_its_line_endings() {
    // Block content gives the link text a blank line in pure mode.
    assert_eq!(
        "[&#10;&#10;a&#10;&#10;b&#10;&#10;](x)",
        convert_pure(r#"<a href="x"><p>a</p><p>b</p></a>"#).unwrap()
    );
    // A link begins an inline context, so faithful mode writes that content as
    // raw HTML inlines instead, which survive the trip back to HTML.
    assert_eq!(
        "[<p>a</p><p>b</p>](x)",
        convert_faithful(r#"<a href="x"><p>a</p><p>b</p></a>"#).unwrap()
    );
    assert_eq!(
        "<p><a href=\"x\"><p>a</p><p>b</p></a></p>\n",
        round_trip(r#"<a href="x"><p>a</p><p>b</p></a>"#)
    );
    assert_eq!(
        "[<p>a</p><p>b</p>][1]\n\n[1]: x",
        convert_referenced(r#"<a href="x"><p>a</p><p>b</p></a>"#)
    );
    // A pure-mode `<br>` is a line ending too.
    assert_eq!(
        "# [a  &#10;b](x)",
        convert_pure(r#"<h1><a href="x">a<br>b</a></h1>"#).unwrap()
    );
}

/// An image is written inline even when links are referenced.
#[test]
fn an_image_is_inline_in_the_referenced_style() {
    assert_eq!(
        r#"![a](i "t")"#,
        convert_referenced(r#"<img src="i" alt="a" title="t">"#)
    );
}

/// Whitespace at either end of the link text stays inside the brackets, so the
/// text reads back unchanged.
#[test]
fn a_link_keeps_the_whitespace_around_its_text() {
    assert_eq!(
        "Hi\\![ t](x)",
        convert_faithful(r#"<p>Hi!<a href="x"> t</a></p>"#).unwrap()
    );
    assert_eq!(
        "<p>Hi!<a href=\"x\"> t</a></p>\n",
        round_trip(r#"<p>Hi!<a href="x"> t</a></p>"#)
    );
    assert_eq!(
        "Hi\\![ t][1]",
        convert_referenced(r#"<p>Hi!<a href="x"> t</a></p>"#)
            .lines()
            .next()
            .unwrap()
    );
    // A space on each side of the `<a>` tag collapses to one.
    assert_eq!(
        "Hi! [ t](x)",
        convert_faithful(r#"<p>Hi! <a href="x"> t</a></p>"#).unwrap()
    );
    assert_eq!(
        "[t ](x)now",
        convert_faithful(r#"<p><a href="x">t </a>now</p>"#).unwrap()
    );
}

/// An [email autolink](https://spec.commonmark.org/0.31.2/#email-autolink) may
/// open with a digit, a `/` or a symbol after its `<`, where a tag needs a
/// letter. Unescaped, one in link text nests a `mailto:` link inside the link,
/// and one in an alt text comes back without its angle brackets.
#[test]
fn a_link_escapes_an_email_autolink() {
    assert_eq!(
        r"[\<1foo@bar.com>](x)",
        convert_faithful(r#"<a href="x">&lt;1foo@bar.com&gt;</a>"#).unwrap()
    );
    assert_eq!(
        "<p><a href=\"x\">&lt;1foo@bar.com&gt;</a></p>\n",
        round_trip(r#"<a href="x">&lt;1foo@bar.com&gt;</a>"#)
    );
    assert_eq!(
        r"![\<+a@b.co>](i)",
        convert_faithful(r#"<img src="i" alt="<+a@b.co>">"#).unwrap()
    );
    assert_eq!(
        "<p><img src=\"i\" alt=\"&lt;+a@b.co&gt;\" /></p>\n",
        round_trip(r#"<img src="i" alt="<+a@b.co>">"#)
    );
}

/// `convert_faithful` with [`LinkStyle::InlinedPreferAutolinks`], the style
/// whose link text and destination decide between the two spellings.
fn convert_autolinks(html: &str) -> String {
    convert_faithful_options(
        html,
        Options {
            link_style: LinkStyle::InlinedPreferAutolinks,
            ..Default::default()
        },
    )
    .unwrap()
}

/// `convert_faithful` with [`LinkStyle::Referenced`], the style which writes
/// the destination in a link reference definition rather than inline.
fn convert_referenced(html: &str) -> String {
    convert_faithful_options(
        html,
        Options {
            link_style: LinkStyle::Referenced,
            ..Default::default()
        },
    )
    .unwrap()
}
