use markup5ever_rcdom::NodeData;

use crate::{
    Element,
    element_handler::{Chain, HandlerResult, serialize_element},
    node_util::get_parent_node,
    options::TranslationMode,
    text_util::concat_strings,
};

pub(super) fn html_handler(_chain: &dyn Chain, element: Element) -> Option<HandlerResult> {
    // In faithful mode, this is markdown translatable only when it's the root
    // of the document.
    let markdown_translatable = if element.options.translation_mode == TranslationMode::Faithful
        && let Some(parent) = get_parent_node(element.node)
        && let NodeData::Document = parent.data
    {
        true
    } else {
        // It's always markdown translatable in pure mode.
        element.options.translation_mode == TranslationMode::Pure
    };

    if markdown_translatable {
        Some(concat_strings!("\n\n", element.content, "\n\n").into())
    } else {
        Some(HandlerResult {
            content: serialize_element(&element),
            markdown_translated: false,
        })
    }
}
