use crate::{
    Element,
    element_handler::{HandlerResult, Handlers},
    serialize_if_faithful,
    text_util::{JoinOnStringIterator, TrimDocumentWhitespace, concat_strings},
};

pub(super) fn blockquote_handler(
    handlers: &dyn Handlers,
    element: Element,
) -> Option<HandlerResult> {
    serialize_if_faithful!(handlers, element, 0);
    let content = handlers.walk_children(element.node).content;
    let content = content.trim_start_matches('\n');
    let content = content
        .trim_end_document_whitespace()
        .lines()
        .map(|line| concat_strings!("> ", line))
        .join("\n");
    Some(concat_strings!("\n\n", content, "\n\n").into())
}
