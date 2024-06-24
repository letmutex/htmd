use crate::{node_util::get_parent_node_tag_name, text_util::concat_strings, Element};

pub(super) fn list_handler(element: Element) -> Option<String> {
    let parent_tag = get_parent_node_tag_name(&element.node);
    if parent_tag.is_some_and(|tag| tag == "li") {
        Some(concat_strings!(
            "\n",
            element.content.trim_matches(|ch| ch == '\n'),
            "\n"
        ))
    } else {
        Some(concat_strings!(
            "\n\n",
            element.content.trim_matches(|ch| ch == '\n'),
            "\n\n"
        ))
    }
}
