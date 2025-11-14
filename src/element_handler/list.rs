use crate::{
    Element,
    element_handler::{Chain, serialize_element},
    node_util::{get_node_tag_name, get_parent_node},
    options::TranslationMode,
    serialize_if_faithful,
    text_util::concat_strings,
};

pub(super) fn list_handler(_chain: &dyn Chain, element: Element) -> (Option<String>, bool) {
    // In faithful mode, ...
    if element.options.translation_mode == TranslationMode::Faithful {
        // ...make sure this element's attributes can be translated as markdown.
        let has_start = element
            .attrs
            .first()
            .is_some_and(|attr| &attr.name.local == "start");
        serialize_if_faithful!(element, if has_start { 1 } else { 0 });

        // ...all children must be translated as Markdown, and all children must
        // be li elements.
        if !element.markdown_translated
            || !element.node.children.borrow().iter().all(|node| {
                let tag_name = get_node_tag_name(node);
                // In addition to elements, there will be text nodes, generally
                // with whitespace; these should be ignored.
                tag_name == Some("li") || tag_name.is_none()
            })
        {
            return (Some(serialize_element(&element)), false);
        }
    }
    let parent = get_parent_node(element.node);
    let is_parent_li = parent
        .map(|p| get_node_tag_name(&p).is_some_and(|tag| tag == "li"))
        .unwrap_or(false);
    if is_parent_li {
        (
            Some(concat_strings!(
                "\n",
                element.content.trim_matches(|ch| ch == '\n'),
                "\n"
            )),
            true,
        )
    } else {
        (
            Some(concat_strings!(
                "\n\n",
                element.content.trim_matches(|ch| ch == '\n'),
                "\n\n"
            )),
            true,
        )
    }
}
