use crate::{
    Element,
    element_handler::element_util::handle_or_serialize_by_parent,
    element_handler::{HandlerResult, Handlers},
};

/// Handler for the `<thead>` and `<tbody>` sections of a table. This tag's
/// ability to translate to markdown requires its children to be markdown
/// translatable as well.
pub(super) fn table_section_handler(
    handlers: &dyn Handlers,
    element: Element,
) -> Option<HandlerResult> {
    handle_or_serialize_by_parent(handlers, &element, &["table"], 0, true)
}
