use crate::{
    Context, Element,
    element_handler::element_util::handle_or_serialize_by_parent,
    element_handler::{HandlerResult, Handlers},
};

pub(super) fn td_th_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    handle_or_serialize_by_parent(
        handlers,
        &element,
        &["tr"],
        0,
        false,
        // A cell's contents are parsed as inline content, so no HTML block can
        // open inside one: an element which only HTML can express becomes a raw
        // HTML inline here, whatever its tag. See the "Translating HTML nodes"
        // section of `unsupported_html.md`.
        Context::Inline,
    )
}
