use crate::{
    element_handler::{
        Context, Element, HandlerResult, Handlers, element_util::serialize_if_extra_attrs,
    },
    util::text::frame_as_block,
};

pub(super) fn caption_handler(handlers: &dyn Handlers, element: &Element) -> Option<HandlerResult> {
    // CommonMark has no caption, so faithful mode always writes the element as
    // HTML. A `<caption>` is only valid inside a `<table>`, so `table_handler`
    // serializes the whole table around it.
    serialize_if_extra_attrs!(handlers, element, -1);
    // Only pure mode reaches here. A caption is written as a paragraph above
    // the table, and a paragraph is a leaf block: its children begin an inline
    // context.
    let content = handlers.walk_children_content(element.node, Context::INLINE);
    Some(frame_as_block(&content).into())
}
