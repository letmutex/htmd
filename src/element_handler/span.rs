use markup5ever_rcdom::NodeData;

use crate::{
    Element,
    element_handler::{HandlerResult, Handlers},
    serialize_if_faithful,
    text_util::concat_strings,
};

pub(super) fn span_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    // See if this contains math: `<span class="math math-inline/display>text-only content</span>`.
    if element.attrs.len() == 1
        && let attr = &element.attrs[0]
        && *attr.name.local == *"class"
        && let children = element.node.children.borrow()
        && children.len() == 1
        && let NodeData::Text { contents } = &children[0].data
    {
        if *attr.value == *"math math-inline" {
            return Some(concat_strings!("$", contents.borrow().to_string(), "$").into());
        }

        if *attr.value == *"math math-display" {
            return Some(concat_strings!("$$", contents.borrow().to_string(), "$$").into());
        }
    }

    // Always serialize as HTML if we're in faithful mode.
    serialize_if_faithful!(handlers, element, -1);

    // Otherwise, just return the contents.
    let content = handlers.walk_children(element.node).content;
    let content = content.trim_matches('\n');

    Some(content.into())
}
