use crate::{
    Context, Element,
    element_handler::element_util::handle_or_serialize_by_parent,
    element_handler::{HandlerResult, Handlers},
};

pub(super) fn caption_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    // A caption is written as a paragraph above the table, and a paragraph is a
    // leaf block: its children begin an inline context.
    handle_or_serialize_by_parent(handlers, &element, &["table"], 0, true, Context::Inline)
}
