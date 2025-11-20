use crate::{
    Element,
    element_handler::{Chain, HandlerResult},
    serialize_if_faithful,
    text_util::{JoinOnStringIterator, TrimAsciiWhitespace, concat_strings},
};

pub(super) fn blockquote_handler(chain: &dyn Chain, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(chain, element, 0);
    let content = chain.walk_children(element.node).content;
    let content = content.trim_start_matches('\n');
    let content = content
        .trim_end_ascii_whitespace()
        .lines()
        .map(|line| concat_strings!("> ", line))
        .join("\n");
    Some(concat_strings!("\n\n", content, "\n\n").into())
}
