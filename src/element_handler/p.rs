use crate::{
    Element,
    element_handler::{HandlerResult, Handlers},
    serialize_if_faithful,
    text_util::concat_strings,
};

pub(super) fn p_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(handlers, element, 0);
    let content = handlers.walk_children(element.node).content;
    let content = content.trim_matches('\n');
    Some(concat_strings!("\n\n", content, "\n\n").into())
}
