use crate::{
    Element,
    element_handler::element_util::handle_or_serialize_by_parent,
    element_handler::{Chain, HandlerResult},
    serialize_if_faithful,
};

pub(super) fn td_th_handler(_chain: &dyn Chain, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(element, 0);
    handle_or_serialize_by_parent(&element, &vec!["tr"], true)
}
