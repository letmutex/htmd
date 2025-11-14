use crate::{Element, element_handler::Chain, node_util::is_parent_handler};

pub(super) fn head_body_handler(_chain: &dyn Chain, element: Element) -> (Option<String>, bool) {
    is_parent_handler(&element, &vec!["html"], true)
}
