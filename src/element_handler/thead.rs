use crate::{
    Element,
    element_handler::element_util::handle_or_serialize_by_parent,
    element_handler::element_util::serialize_if_extra_attrs,
    element_handler::{HandlerResult, Handlers},
};

pub(super) fn thead_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    serialize_if_extra_attrs!(handlers, element, 0);
    // This tag's ability to translate to markdown requires its children to be
    // markdown translatable as well.
    handle_or_serialize_by_parent(handlers, &element, &["table"], true)
}
