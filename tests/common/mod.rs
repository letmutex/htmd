// Shared code for all integration tests.
use htmd::{
    HtmlToMarkdown,
    options::{HeadingStyle, Options, TranslationMode},
};

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
