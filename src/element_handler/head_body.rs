use crate::{
    Element,
    element_handler::element_util::handle_or_serialize_by_parent,
    element_handler::{Chain, HandlerResult},
};

pub(super) fn head_body_handler(_chain: &dyn Chain, element: Element) -> Option<HandlerResult> {
    handle_or_serialize_by_parent(&element, &vec!["html"], true)
}
