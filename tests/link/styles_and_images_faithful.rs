use htmd::{
    HtmlToMarkdown,
    options::{LinkStyle, Options, TranslationMode},
};
use pulldown_cmark::Options as CommonMarkOptions;

use super::convert_referenced;
use crate::common::{convert_faithful, convert_faithful_options, render_markdown, round_trip};

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
/// `translation_approach.md`; failing any one gives the inline form.
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

/// An `<img>` without a `src` has no Markdown spelling -- `![q](<>)` reads
/// back with an empty `src` -- so it is written as raw HTML rather than dropped
/// along with its alt text.
#[test]
fn an_image_without_a_source_is_written_as_html() {
    assert_eq!(
        r#"a<img alt="q">b"#,
        convert_faithful(r#"<p>a<img alt="q">b</p>"#).unwrap()
    );
    assert_eq!(
        "<p>a<img alt=\"q\">b</p>\n",
        round_trip(r#"<p>a<img alt="q">b</p>"#)
    );
}

/// An `<img>` without an `alt` has no Markdown spelling -- `![](i)` reads back
/// with `alt=""`, which [means](https://html.spec.whatwg.org/multipage/images.html#alt)
/// a decorative image rather than one lacking a text alternative -- so it is
/// written as raw HTML.
#[test]
fn an_image_without_alt_text_is_written_as_html() {
    assert_eq!(
        r#"a<img src="i">b"#,
        convert_faithful(r#"<p>a<img src="i">b</p>"#).unwrap()
    );
    assert_eq!(
        "<p>a<img src=\"i\">b</p>\n",
        round_trip(r#"<p>a<img src="i">b</p>"#)
    );
    // An empty `alt` is the one `![](i)` spells.
    assert_eq!(
        "![](i)",
        convert_faithful(r#"<img src="i" alt="">"#).unwrap()
    );
    assert_eq!(
        "<p><img src=\"i\" alt=\"\" /></p>\n",
        round_trip(r#"<img src="i" alt="">"#)
    );
}

/// An `<img>` with an empty `title` has no Markdown spelling -- `![q](i "")`
/// reads back without a `title`, which
/// [lets](https://html.spec.whatwg.org/multipage/dom.html#the-title-attribute)
/// an ancestor's advisory information apply where the empty one hid it -- so
/// it is written as raw HTML.
#[test]
fn an_image_with_an_empty_title_is_written_as_html() {
    assert_eq!(
        r#"a<img src="i" alt="q" title="">b"#,
        convert_faithful(r#"<p>a<img src="i" alt="q" title="">b</p>"#).unwrap()
    );
    assert_eq!(
        "<p>a<img src=\"i\" alt=\"q\" title=\"\">b</p>\n",
        round_trip(r#"<p>a<img src="i" alt="q" title="">b</p>"#)
    );
}
