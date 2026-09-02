use crate::{
    Element,
    element_handler::element_util::serialize_if_extra_attrs,
    element_handler::{HandlerResult, Handlers},
    text_util::frame_as_block,
};

pub(super) fn p_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    serialize_if_extra_attrs!(handlers, element, 0);
    let content = handlers.walk_children(element.node).content;
    Some(frame_as_block(&content).into())
}
