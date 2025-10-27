// Shared code for all integration tests.
use htmd::{
    HtmlToMarkdown,
    options::{Options, TranslationMode},
};

// By default, use the faithful translation mode, which is more stringent.
pub fn convert(html: &str) -> std::io::Result<String> {
    HtmlToMarkdown::builder()
        .options(Options {
            translation_mode: TranslationMode::Faithful,
            ..Default::default()
        })
        .build()
        .convert(html)
}
