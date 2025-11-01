use crate::{Element, node_util::is_parent_handler, serialize_if_faithful};

pub(super) fn tbody_handler(element: Element) -> (Option<String>, bool) {
    serialize_if_faithful!(element, 0);
    // This tag's ability to translate to markdown requires its children to be
    // markdown translatable as well.
    is_parent_handler(&element, &vec!["table"], element.markdown_translated)
}
