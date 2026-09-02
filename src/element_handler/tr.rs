use crate::{
    Element,
    element_handler::element_util::handle_or_serialize_by_parent,
    element_handler::{HandlerResult, Handlers},
};

pub(super) fn tr_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    // This tag's ability to translate to markdown requires its children to be
    // markdown translatable as well.
    handle_or_serialize_by_parent(handlers, &element, &["tbody", "thead"], 0, true)
}
