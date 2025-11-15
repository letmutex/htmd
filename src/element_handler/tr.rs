use crate::{
    Element,
    element_handler::element_util::handle_or_serialize_by_parent,
    element_handler::{Chain, HandlerResult},
    serialize_if_faithful,
};

pub(super) fn tr_handler(_chain: &dyn Chain, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(element, 0);
    // This tag's ability to translate to markdown requires its children to be
    // markdown translatable as well.
    handle_or_serialize_by_parent(
        &element,
        &vec!["tbody", "thead"],
        element.markdown_translated,
    )
}
