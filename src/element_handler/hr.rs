use crate::{
    Element,
    element_handler::{HandlerResult, Handlers},
    options::HrStyle,
    serialize_if_faithful,
};

pub(super) fn hr_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(handlers, element, 0);
    match handlers.options().hr_style {
        HrStyle::Dashes => Some("\n\n- - -\n\n".into()),
        HrStyle::Asterisks => Some("\n\n* * *\n\n".into()),
        HrStyle::Underscores => Some("\n\n_ _ _\n\n".into()),
    }
}
