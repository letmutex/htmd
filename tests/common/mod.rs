// Shared code for all integration tests.
//
// Each integration test file compiles as a crate of its own, so a helper here is
// dead code to every crate that does not happen to use it.
#![allow(dead_code)]

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

/// `md` read as CommonMark and written back out as HTML, for the tests that care
/// whether their output *means* what it looks like.
pub fn render(md: &str) -> String {
    let mut options = pulldown_cmark::Options::empty();
    options.insert(pulldown_cmark::Options::ENABLE_TABLES);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, pulldown_cmark::Parser::new_ext(md, options));
    html
}
