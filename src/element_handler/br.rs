use crate::{Element, options::BrStyle, serialize_if_faithful};

pub(super) fn br_handler(element: Element) -> (Option<String>, bool) {
    serialize_if_faithful!(element, 0);
    (
        match element.options.br_style {
            BrStyle::TwoSpaces => Some("  \n".to_string()),
            BrStyle::Backslash => Some("\\\n".to_string()),
        },
        true,
    )
}
