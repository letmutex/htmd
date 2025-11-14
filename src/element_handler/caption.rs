use crate::{Element, element_handler::Chain, node_util::is_parent_handler, serialize_if_faithful};

pub(super) fn caption_handler(_chain: &dyn Chain, element: Element) -> (Option<String>, bool) {
    serialize_if_faithful!(element, 0);
    is_parent_handler(&element, &vec!["table"], element.markdown_translated)
}
