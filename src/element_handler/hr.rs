use crate::{Element, options::HrStyle, serialize_if_faithful};

pub(super) fn hr_handler(element: Element) -> (Option<String>, bool) {
    serialize_if_faithful!(element, 0);
    (
        match element.options.hr_style {
            HrStyle::Dashes => Some("\n\n- - -\n\n".to_string()),
            HrStyle::Asterisks => Some("\n\n* * *\n\n".to_string()),
            HrStyle::Underscores => Some("\n\n_ _ _\n\n".to_string()),
        },
        true,
    )
}
