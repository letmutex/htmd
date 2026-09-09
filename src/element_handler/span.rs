use markup5ever_rcdom::NodeData;

use crate::{
    Element,
    element_handler::element_util::serialize_if_extra_attrs,
    element_handler::{HandlerResult, Handlers},
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
        let delimiter = match &*attr.value {
            "math math-inline" => Some("$"),
            "math math-display" => Some("$$"),
            _ => None,
        };
        if let Some(delimiter) = delimiter {
            // Per the [spec](../unsupported_html.md), replace newlines with
            // spaces: LaTeX ignores whitespace, while a line ending here would
            // end the math span and begin another block. A CRLF pair is a
            // single line ending, so it becomes a single space.
            let contents = contents.borrow();
            let mut math = String::with_capacity(contents.len());
            let mut chars = contents.chars().peekable();
            while let Some(c) = chars.next() {
                match c {
                    '\r' => {
                        chars.next_if_eq(&'\n');
                        math.push(' ');
                    }
                    '\n' => math.push(' '),
                    _ => math.push(c),
                }
            }
            return Some(concat_strings!(delimiter, math, delimiter).into());
        }
    }

    // Always serialize as HTML if we're in faithful mode.
    serialize_if_extra_attrs!(handlers, element, -1);

    // Otherwise, just return the contents.
    let content = handlers.walk_children_content(element.node, element.context);
    let content = content.trim_matches('\n');

    Some(content.into())
}
