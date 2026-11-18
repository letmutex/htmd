use crate::element_handler::{
    Element, HandlerResult, Handlers, element_util::handle_or_serialize_by_parent,
};

pub(super) fn head_body_handler(
    handlers: &dyn Handlers,
    element: &Element,
) -> Option<HandlerResult> {
    handle_or_serialize_by_parent(
        handlers,
        element,
        &["html"],
        i64::MAX,
        false,
        element.context,
    )
}
