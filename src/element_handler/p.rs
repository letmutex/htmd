use crate::{
    Element,
    element_handler::{Chain, HandlerResult},
    serialize_if_faithful,
    text_util::concat_strings,
};

pub(super) fn p_handler(chain: &dyn Chain, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(chain, element, 0);
    let content = chain.walk_children(element.node).content;
    let content = content.trim_matches('\n');
    Some(concat_strings!("\n\n", content, "\n\n").into())
}
