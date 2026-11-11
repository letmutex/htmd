use indoc::indoc;
use pretty_assertions::assert_eq;
use pulldown_cmark::Options as CommonMarkOptions;

use super::assert_round_trips;
use crate::common::{convert_faithful, render_markdown, round_trip};

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

/// An [email autolink](https://spec.commonmark.org/0.31.2/#email-autolink)
/// needs the escape a tag gets, though its `<` may be followed by a digit, a
/// `/` or a symbol rather than a letter.
#[test]
fn inline_email_autolink_escaping() {
    let html = "<p>Mail &lt;1foo@bar.com&gt;, &lt;/1@b.co&gt; or &lt;+a@b.co&gt;.</p>";
    assert_eq!(
        r"Mail \<1foo@bar.com>, \</1@b.co> or \<+a@b.co>.",
        convert_faithful(html).unwrap()
    );
    assert_eq!(
        "<p>Mail &lt;1foo@bar.com&gt;, &lt;/1@b.co&gt; or &lt;+a@b.co&gt;.</p>\n",
        round_trip(html)
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

/// Every `&` is escaped, since one which begins an
/// [entity or numeric character reference](https://spec.commonmark.org/0.31.2/#entity-and-numeric-character-references)
/// would otherwise be decoded.
#[test]
fn html_entities() {
    let html = r#"<p><a href="/my%20&amp;uri" title="my%20&amp;title">my%20&amp;link</a></p>"#;
    assert_eq!(
        r#"[my%20\&link](/my%20\&uri "my%20\&title")"#,
        convert_faithful(html).unwrap()
    );

    let html_plain = r#"<p>This &amp; that, then &lt; &gt; now.</p>"#;
    assert_eq!(
        r#"This \& that, then < > now."#,
        convert_faithful(html_plain).unwrap()
    );

    // `&amp;copy;` is the text `&copy;`, not the character it names.
    assert_round_trips("<p>&amp;copy; &amp;#169; &amp;#xA9;</p>");
}

/// What `escape_html` owes a [CDATA section](https://spec.commonmark.org/0.31.2/#cdata-section),
/// which no conversion can reach: `escape_if_needed` escapes the brackets
/// before `escape_html` runs, so the spelling htmd emits is the escaped one,
/// and the first trip below is the one that spelling takes -- proof that
/// `escape_html` must go on leaving it alone.
///
/// The last two read Markdown directly, which is the only way to reach the
/// unescaped spelling, and show what emitting it would cost: it is raw HTML,
/// so it passes through to the output for an HTML parser to read as a bogus
/// comment and show as nothing.
#[test]
fn a_literal_cdata_section_is_lost_unescaped() {
    assert_eq!(
        "<p>&lt;![CDATA[x]]&gt;</p>\n",
        round_trip("<p>&lt;![CDATA[x]]&gt;</p>")
    );
    assert_eq!(
        "<p>a <![CDATA[x]]> b</p>\n",
        render_markdown("a <![CDATA[x]]> b", CommonMarkOptions::empty())
    );
    assert_eq!(
        "<p>a &lt;![CDATA[x]]&gt; b</p>\n",
        render_markdown(r"a \<![CDATA[x]]> b", CommonMarkOptions::empty())
    );
}
