use crate::{
    Element,
    element_handler::{HandlerResult, Handlers},
    options::BrStyle,
    serialize_if_faithful,
};

pub(super) fn br_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(handlers, element, 0);

    match handlers.options().br_style {
        BrStyle::TwoSpaces => Some("  \n".into()),
        BrStyle::Backslash => Some("\\\n".into()),
    }
}
