use crate::{
    Element, NodeType,
    element_handler::{HandlerResult, Handlers, element_util::serialize_element_result},
    options::TranslationMode,
    util::text::frame_as_block,
};

pub(super) fn html_handler(handlers: &dyn Handlers, element: &Element) -> Option<HandlerResult> {
    // It's always markdown translatable in pure mode; in faithful mode, only
    // when it's the root of the document.
    let markdown_translatable = handlers.options().translation_mode == TranslationMode::Pure
        || element
            .node
            .parent()
            .is_some_and(|parent| parent.node_type() == NodeType::Document);

    if markdown_translatable {
        let content = handlers.walk_children_content(element.node, element.context);
        Some(frame_as_block(&content).into())
    } else {
        Some(serialize_element_result(handlers, element))
    }
}
