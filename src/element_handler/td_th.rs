use crate::{
    Context, Element,
    dom_walker::is_block_element,
    element_handler::element_util::handle_or_serialize_by_parent,
    element_handler::{HandlerResult, Handlers},
    node_util::get_node_tag_name,
    options::TranslationMode,
};

pub(super) fn td_th_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    // Are any of this node's children block elements?
    let has_block_elements = handlers.options().translation_mode == TranslationMode::Faithful
        && element
            .node
            .children
            .borrow()
            .iter()
            .any(|child| get_node_tag_name(child).is_some_and(is_block_element));
    handle_or_serialize_by_parent(
        handlers,
        &element,
        &["tr"],
        // Force HTML serialization in faithful mode if the table cell contains
        // block elements.
        if has_block_elements { -1 } else { 0 },
        false,
        // A cell's contents are parsed as inline content, so no HTML block can
        // open inside one.
        Context::Inline,
    )
}
