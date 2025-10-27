use crate::{Element, serialize_if_faithful, text_util::concat_strings};

pub(super) fn p_handler(element: Element) -> (Option<String>, bool) {
    serialize_if_faithful!(element, 0);
    (Some(concat_strings!("\n\n", element.content, "\n\n")), true)
}
