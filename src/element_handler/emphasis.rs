use crate::{
    Element,
    element_handler::{Chain, HandlerResult},
    serialize_if_faithful,
    text_util::{StripWhitespace, concat_strings},
};

pub(super) fn emphasis_handler(
    chain: &dyn Chain,
    element: Element,
    marker: &str,
) -> Option<HandlerResult> {
    serialize_if_faithful!(chain, element, 0);
    let content = chain.walk_children(element.node);
    if content.is_empty() {
        return None;
    }
    let (content, leading_whitespace) = content.strip_leading_whitespace();
    let (content, trailing_whitespace) = content.strip_trailing_whitespace();
    if content.is_empty() {
        return None;
    }
    let content = concat_strings!(
        leading_whitespace.unwrap_or(""),
        marker,
        content,
        marker,
        trailing_whitespace.unwrap_or("")
    );
    Some(content.into())
}
