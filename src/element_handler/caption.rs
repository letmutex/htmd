use crate::{
    Element,
    element_handler::element_util::handle_or_serialize_by_parent,
    element_handler::{HandlerResult, Handlers},
};

pub(super) fn caption_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    handle_or_serialize_by_parent(handlers, &element, &["table"], 0, true)
}
