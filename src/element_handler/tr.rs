use crate::{
    Element,
    element_handler::element_util::is_parent_handler,
    element_handler::{Chain, HandlerResult},
    serialize_if_faithful,
};

pub(super) fn tr_handler(_chain: &dyn Chain, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(element, 0);
    // This tag's ability to translate to markdown requires its children to be
    // markdown translatable as well.
    is_parent_handler(
        &element,
        &vec!["tbody", "thead"],
        element.markdown_translated,
    )
}
