use crate::{
    Context, Element,
    element_handler::element_util::{
        serialize_element_when_faithful, serialize_if_extra_attrs_or_inline,
    },
    element_handler::{HandlerResult, Handlers},
    html_block::starts_html_block,
    util::text::frame_as_block,
};

pub(super) fn p_handler(handlers: &dyn Handlers, element: &Element) -> Option<HandlerResult> {
    serialize_if_extra_attrs_or_inline!(handlers, element, 0);
    // A paragraph is a leaf block: its children begin an inline context.
    let content = handlers.walk_children_content(element.node, Context::INLINE);
    // An HTML block opening at the start of the content dissolves the paragraph
    // holding it, so writing the content alone would lose the `<p>`; the whole
    // element goes out as HTML instead. See the "Special case for paragraphs"
    // section of `unsupported_html.md`.
    serialize_element_when_faithful!(handlers, element, starts_html_block(&content));
    Some(frame_as_block(&content).into())
}
