use crate::{
    Element,
    element_handler::{Chain, HandlerResult},
    options::BrStyle,
    serialize_if_faithful,
};

pub(super) fn br_handler(chain: &dyn Chain, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(chain, element, 0);

    match element.options.br_style {
        BrStyle::TwoSpaces => Some("  \n".into()),
        BrStyle::Backslash => Some("\\\n".into()),
    }
}
