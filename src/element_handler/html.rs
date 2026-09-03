use markup5ever_rcdom::NodeData;

use crate::{
    Element,
    element_handler::{HandlerResult, Handlers, element_util::serialize_element_result},
    node_util::get_parent_node,
    options::TranslationMode,
    text_util::frame_as_block,
};

pub(super) fn html_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    // It's always markdown translatable in pure mode; in faithful mode, only
    // when it's the root of the document.
    let markdown_translatable = handlers.options().translation_mode == TranslationMode::Pure
        || get_parent_node(element.node)
            .is_some_and(|parent| matches!(parent.data, NodeData::Document));

    if markdown_translatable {
        let content = handlers.walk_children(element.node).content;
        Some(frame_as_block(&content).into())
    } else {
        Some(serialize_element_result(handlers, &element))
    }
}
