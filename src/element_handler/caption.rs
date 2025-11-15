use crate::{
    Element,
    element_handler::element_util::is_parent_handler,
    element_handler::{Chain, HandlerResult},
    serialize_if_faithful,
};

pub(super) fn caption_handler(_chain: &dyn Chain, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(element, 0);
    is_parent_handler(&element, &vec!["table"], element.markdown_translated)
}
