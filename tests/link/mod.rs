use htmd::options::{LinkStyle, Options};

use crate::common::convert_faithful_options;

mod escaping_faithful;
mod styles_and_images;
mod styles_and_images_faithful;

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
