use crate::{Element, node_util::is_parent_handler, serialize_if_faithful};

pub(super) fn td_th_handler(element: Element) -> (Option<String>, bool) {
    serialize_if_faithful!(element, 0);
    is_parent_handler(&element, &vec!["tr"], true)
}
