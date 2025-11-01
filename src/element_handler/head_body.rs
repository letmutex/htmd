use crate::{Element, node_util::is_parent_handler};

pub(super) fn head_body_handler(element: Element) -> (Option<String>, bool) {
    is_parent_handler(&element, &vec!["html"], true)
}
