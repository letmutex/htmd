use crate::{
    Element,
    element_handler::{Chain, HandlerResult},
    options::HrStyle,
    serialize_if_faithful,
};

pub(super) fn hr_handler(_chain: &dyn Chain, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(element, 0);
    match element.options.hr_style {
        HrStyle::Dashes => Some("\n\n- - -\n\n".into()),
        HrStyle::Asterisks => Some("\n\n* * *\n\n".into()),
        HrStyle::Underscores => Some("\n\n_ _ _\n\n".into()),
    }
}
