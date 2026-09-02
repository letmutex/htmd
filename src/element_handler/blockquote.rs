use crate::{
    Context, Element,
    element_handler::element_util::serialize_if_extra_attrs,
    element_handler::{HandlerResult, Handlers},
    text_util::{JoinOnStringIterator, TrimDocumentWhitespace, concat_strings, frame_as_block},
};

pub(super) fn blockquote_handler(
    handlers: &dyn Handlers,
    element: Element,
) -> Option<HandlerResult> {
    serialize_if_extra_attrs!(handlers, element, 0);
    // A blockquote is a container block: its children begin a block context.
    let content = handlers.walk_children_content(element.node, Context::Block);
    let content = content.trim_start_matches('\n');
    let content = content
        .trim_end_document_whitespace()
        .lines()
        .map(|line| concat_strings!("> ", line))
        .join("\n");
    Some(frame_as_block(&content).into())
}
