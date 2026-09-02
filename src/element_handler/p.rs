use crate::{
    Context, Element,
    element_handler::element_util::serialize_if_extra_attrs,
    element_handler::{HandlerResult, Handlers},
    text_util::frame_as_block,
};

pub(super) fn p_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    serialize_if_extra_attrs!(handlers, element, 0);
    // A paragraph is a leaf block: its children begin an inline context.
    let content = handlers.walk_children_content(element.node, Context::Inline);
    Some(frame_as_block(&content).into())
}
