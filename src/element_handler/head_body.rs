use crate::{
    Element,
    element_handler::element_util::is_parent_handler,
    element_handler::{Chain, HandlerResult},
};

pub(super) fn head_body_handler(_chain: &dyn Chain, element: Element) -> Option<HandlerResult> {
    is_parent_handler(&element, &vec!["html"], true)
}
