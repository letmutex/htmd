use markup5ever_rcdom::NodeData;

use crate::{
    Element, element_handler::serialize_element, node_util::get_parent_node,
    options::TranslationMode, text_util::concat_strings,
};

pub(super) fn html_handler(element: Element) -> (Option<String>, bool) {
    // In faithful mode, this is markdown translatable only when it's the root
    // of the document.
    let markdown_translatable = if element.html_to_markdown.options.translation_mode
        == TranslationMode::Faithful
        && let Some(parent) = get_parent_node(element.node)
        && let NodeData::Document = parent.data
    {
        true
    } else {
        // It's always markdown translatable in pure mode.
        element.html_to_markdown.options.translation_mode == TranslationMode::Pure
    };

    if markdown_translatable {
        (Some(concat_strings!("\n\n", element.content, "\n\n")), true)
    } else {
        (Some(serialize_element(&element)), false)
    }
}
