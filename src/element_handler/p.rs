use crate::{
    Element,
    element_handler::{HandlerResult, Handlers},
    serialize_if_faithful,
    text_util::frame_as_block,
};

pub(super) fn p_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(handlers, element, 0);
    let content = handlers.walk_children(element.node).content;
    Some(frame_as_block(&content).into())
}
