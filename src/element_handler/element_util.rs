use crate::{
    Context, Element,
    dom_walker::is_block_element,
    element_handler::{HandlerResult, Handlers},
    node_util::parent_tag_name_equals,
    text_util::frame_as_block,
};
use html5ever::serialize::{HtmlSerializer, SerializeOpts, Serializer, TraversalScope, serialize};
use markup5ever_rcdom::{NodeData, SerializableHandle};
use std::io::{self, Write};

/// Returns from the enclosing handler with `content` as the element's HTML,
/// but only in faithful mode and only when `condition` holds. Pure mode has no
/// HTML to fall back on, so it always translates.
macro_rules! serialize_when_faithful {
    ($handlers:expr, $condition:expr, $content:expr) => {
        if $handlers.options().translation_mode == $crate::options::TranslationMode::Faithful
            && $condition
        {
            return Some($crate::element_handler::HandlerResult::html($content));
        }
    };
}

pub(crate) use serialize_when_faithful;

/// Handles a structural element when it has an allowed parent, or preserves it
/// as HTML in faithful mode when it appears elsewhere.
///
/// Faithful mode also writes the element as HTML when it carries more
/// attributes than its Markdown translation can express, on the same terms as
/// [`serialize_if_extra_attrs!`]: `num_attrs_allowed` of -1 rejects every
/// attribute set, and [`i64::MAX`] accepts any. Either test serializes the same
/// element, so the order they are tried in does not matter.
///
/// Handled child content is framed as a block, and translated in
/// `children_context`. When `propagate_children_translation` is true, the
/// returned translation status is false if any child required HTML; callers can
/// use that status to fall back to serializing a larger containing structure.
pub(super) fn handle_or_serialize_by_parent(
    handlers: &dyn Handlers,
    element: &Element,
    tag_names: &[&str],
    num_attrs_allowed: i64,
    propagate_children_translation: bool,
    children_context: Context,
) -> Option<HandlerResult> {
    serialize_when_faithful!(
        handlers,
        element.attrs.len() as i64 > num_attrs_allowed
            || !parent_tag_name_equals(element.node, tag_names),
        serialize_element(handlers, element)
    );
    let result = handlers.walk_children(element.node, children_context);
    Some(HandlerResult {
        content: frame_as_block(&result.content),
        markdown_translated: !propagate_children_translation || result.markdown_translated,
    })
}

/// The [`HandlerResult`] for an element which can only be written as HTML: the
/// element serialized, reported as not translated so that a container needing
/// all-CommonMark children can serialize itself instead.
pub(crate) fn serialize_element_result(
    handlers: &dyn Handlers,
    element: &Element,
) -> HandlerResult {
    HandlerResult::html(serialize_element(handlers, element))
}

pub(crate) fn serialize_element(handlers: &dyn Handlers, element: &Element) -> String {
    try_serialize_element(handlers, element).unwrap_or_else(|error| error.to_string())
}

fn try_serialize_element(handlers: &dyn Handlers, element: &Element) -> io::Result<String> {
    let options = SerializeOpts {
        traversal_scope: TraversalScope::IncludeNode,
        ..Default::default()
    };

    if is_block_element(element.tag) {
        serialize_block_element(element, options)
    } else {
        serialize_inline_element(handlers, element, options)
    }
}

fn serialize_inline_element(
    handlers: &dyn Handlers,
    element: &Element,
    options: SerializeOpts,
) -> io::Result<String> {
    let NodeData::Element { name, attrs, .. } = &element.node.data else {
        return Err(io::Error::other("expected an element node"));
    };

    let mut bytes = Vec::new();
    let mut serializer = HtmlSerializer::new(&mut bytes, options);
    serializer.start_elem(
        name.clone(),
        attrs
            .borrow()
            .iter()
            .map(|attr| (&attr.name, &attr.value[..])),
    )?;
    serializer
        .writer
        // What a raw HTML inline holds is inline content, whatever context the
        // element itself appears in.
        .write_all(
            handlers
                .walk_children_content(element.node, Context::Inline)
                .as_bytes(),
        )?;
    serializer.end_elem(name.clone())?;
    String::from_utf8(bytes).map_err(io::Error::other)
}

fn serialize_block_element(element: &Element, options: SerializeOpts) -> io::Result<String> {
    let mut bytes = Vec::new();
    let handle = SerializableHandle::from(element.node.clone());
    serialize(&mut bytes, &handle, options)?;
    let html = String::from_utf8(bytes).map_err(io::Error::other)?;
    Ok(frame_as_block(&escape_html_block_blank_lines(&html)))
}

// A blank line terminates a CommonMark HTML block. Encode every line ending
// after the first so serialized block content remains in one HTML block.
fn escape_html_block_blank_lines(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut chars = html.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\r' && ch != '\n' {
            result.push(ch);
            continue;
        }

        result.push(ch);
        if ch == '\r' && chars.peek() == Some(&'\n') {
            result.push(chars.next().unwrap());
        }

        copy_horizontal_whitespace(&mut chars, &mut result);
        let Some(next) = chars.next() else {
            break;
        };
        match next {
            '\n' => result.push_str("&#10;"),
            '\r' if chars.peek() == Some(&'\n') => {
                chars.next();
                result.push_str("&#13;&#10;");
            }
            '\r' => result.push_str("&#13;"),
            _ => {
                result.push(next);
                continue;
            }
        }
        copy_horizontal_whitespace(&mut chars, &mut result);
    }

    result
}

fn copy_horizontal_whitespace<I>(chars: &mut std::iter::Peekable<I>, output: &mut String)
where
    I: Iterator<Item = char>,
{
    while let Some(&next) = chars.peek() {
        if !next.is_whitespace() || next == '\r' || next == '\n' {
            break;
        }
        output.push(next);
        chars.next();
    }
}

/// Returns from the enclosing handler with `element` written as HTML when it
/// carries more attributes than its Markdown translation can express.
/// `num_attrs_allowed` of -1 rejects every attribute set, serializing the
/// element whenever the mode is faithful; [`i64::MAX`] accepts any.
macro_rules! serialize_if_extra_attrs {
    ($handlers:expr, $element:expr, $num_attrs_allowed:expr) => {
        $crate::element_handler::element_util::serialize_when_faithful!(
            $handlers,
            $element.attrs.len() as i64 > $num_attrs_allowed,
            $crate::element_handler::element_util::serialize_element($handlers, &$element)
        )
    };
}

pub(crate) use serialize_if_extra_attrs;

#[cfg(test)]
mod tests {
    use super::escape_html_block_blank_lines;

    #[test]
    fn escapes_blank_lines_for_each_line_ending_style() {
        assert_eq!("a\n&#10;b", escape_html_block_blank_lines("a\n\nb"));
        assert_eq!(
            "a\r\n&#13;&#10;b",
            escape_html_block_blank_lines("a\r\n\r\nb")
        );
        assert_eq!("a\r&#13;b", escape_html_block_blank_lines("a\r\rb"));
    }
}
