use pretty_assertions::assert_eq;
use pulldown_cmark::Options as CommonMarkOptions;

use crate::common::{convert_faithful, convert_pure, render_markdown};

fn assert_round_trips_with_strikethrough(html: &str) {
    let markdown = convert_faithful(html).unwrap();
    assert_eq!(
        format!("{html}\n"),
        render_markdown(&markdown, CommonMarkOptions::ENABLE_STRIKETHROUGH),
        "round trip of {html} as {markdown:?}"
    );
}

/// pulldown-cmark reads a run of one or two `~` as a
/// [strikethrough](https://pulldown-cmark.github.io/pulldown-cmark/specs/strikethrough.html)
/// delimiter, which a run of the same length closes. An escape shortens a run,
/// so escaping only the first `~` of `~~` leaves a delimiter of one.
#[test]
fn tildes_in_text_strike_nothing() {
    for html in [
        "<p>~a~</p>",
        "<p>~~a~~</p>",
        "<p>a~~b~~c</p>",
        "<p>a ~b~~ c ~~d~ e</p>",
        "<p>~~~a~~</p>",
        "<p><em>x</em>~~~a<em>y</em>~~~ b</p>",
        "<p><strong>~~</strong>~</p>",
        r#"<p><img src="u" alt="~a~" /></p>"#,
    ] {
        assert_round_trips_with_strikethrough(html);
    }
    assert_eq!(r"\~a\~", convert_faithful("<p>~a~</p>").unwrap());
    assert_eq!(r"\~a\~", convert_pure("<p>~a~</p>").unwrap());
}

/// A run of three or more delimits nothing, so only one beginning a text node,
/// which may begin a line and open a code fence, is escaped: all of it, since a
/// single backslash would leave a run of two.
#[test]
fn a_longer_run_of_tildes_is_escaped_only_where_it_could_open_a_code_fence() {
    assert_eq!("a ~~~ b", convert_faithful("<p>a ~~~ b</p>").unwrap());
    assert_eq!(r"\~\~\~ foo", convert_faithful("<p>~~~ foo</p>").unwrap());
    assert_eq!(r"\~\~\~\~", convert_faithful("<p>~~~~</p>").unwrap());
    assert_round_trips_with_strikethrough("<p>~~~ foo</p>");
}
