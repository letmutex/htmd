use crate::{
    Element,
    element_handler::element_util::handle_or_serialize_by_parent,
    element_handler::{Chain, HandlerResult},
    serialize_if_faithful,
};

pub(super) fn thead_handler(chain: &dyn Chain, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(chain, element, 0);
    // This tag's ability to translate to markdown requires its children to be
    // markdown translatable as well.
    handle_or_serialize_by_parent(chain, &element, &vec!["table"], element.markdown_translated)
}
