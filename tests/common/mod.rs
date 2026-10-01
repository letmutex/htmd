// Shared code for all integration tests.
use htmd::{
    HtmlToMarkdown,
    options::{HeadingStyle, Options, TranslationMode},
};
use pulldown_cmark::{Options as CommonMarkOptions, Parser};

// By default, use the faithful translation mode, which is more stringent.
pub fn convert_faithful(html: &str) -> std::io::Result<String> {
    convert_faithful_options(html, Options::default())
}

// `convert_faithful` with the given options. The translation mode is
// overridden, so callers vary only the options they care about.
pub fn convert_faithful_options(html: &str, options: Options) -> std::io::Result<String> {
    HtmlToMarkdown::builder()
        .options(Options {
            translation_mode: TranslationMode::Faithful,
            ..options
        })
        .build()
        .convert(html)
}

// `convert_faithful` with setext headings, the style whose h1 and h2 leave
// their content at the start of a line.
#[allow(dead_code)]
pub fn convert_faithful_setext(html: &str) -> std::io::Result<String> {
    convert_faithful_options(
        html,
        Options {
            heading_style: HeadingStyle::Setex,
            ..Default::default()
        },
    )
}

// The default translation mode, which drops what CommonMark cannot spell.
#[allow(dead_code)]
pub fn convert_pure(html: &str) -> std::io::Result<String> {
    HtmlToMarkdown::builder().build().convert(html)
}

// Takes `html` back to HTML the long way round: `convert_faithful` writes the
// Markdown, and pulldown-cmark reads that Markdown back.
//
// What comes back is HTML *source*, not a DOM, so a character reference in it
// is decoded only later, by whatever HTML parser reads the result. That is why
// several callers still assert a `&#10;`: it decodes to the line ending it
// replaced, so the trip is faithful even though the strings differ. Where a
// trip loses something instead, the assertion says what.
#[allow(dead_code)]
pub fn round_trip(html: &str) -> String {
    render_markdown(&convert_faithful(html).unwrap(), CommonMarkOptions::empty())
}

// The second half of a round trip: the CommonMark of the first half read back
// as HTML.
#[allow(dead_code)]
pub fn render_markdown(markdown: &str, options: CommonMarkOptions) -> String {
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, Parser::new_ext(markdown, options));
    html_output
}
