use htmd::{
    HtmlToMarkdown,
    options::{BulletListMarker, Options, TranslationMode},
};

fn main() {
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            translation_mode: TranslationMode::Faithful,
            bullet_list_marker: BulletListMarker::Dash,
            ul_bullet_spacing: 1,
            ol_number_spacing: 1,
            ..Default::default()
        })
        .build();

    let html = include_str!("tasks.html");

    println!("{}", converter.convert(html).unwrap());
}
