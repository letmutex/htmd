use crate::{
    Element,
    dom_walker::is_block_element,
    element_handler::{HandlerResult, Handlers},
    node_util::parent_tag_name_equals,
    options::TranslationMode,
    text_util::concat_strings,
};
use html5ever::serialize::{HtmlSerializer, SerializeOpts, Serializer, TraversalScope, serialize};
use markup5ever_rcdom::{NodeData, SerializableHandle};
use std::{
    cell::Cell,
    io::{self, Write},
};

thread_local! {
    /// How many elements are being serialized as HTML around the element
    /// currently being converted. See [`in_raw_html`].
    static RAW_HTML_DEPTH: Cell<usize> = const { Cell::new(0) };
}

/// Whether the element being converted sits inside an element that is being
/// written as raw HTML.
///
/// Only a serialized *inline* element gets here, since only there does
/// [`serialize_element`] walk children at all: a block goes to html5ever whole,
/// and nothing inside one is ever converted.
///
/// Markdown inside such an element is still Markdown — a [raw HTML
/// inline](https://spec.commonmark.org/0.31.2/#raw-html) holds Markdown, unlike
/// an HTML block — but a hard line break is a poor way to spell a `<br>` there.
/// Two trailing spaces are invisible, and any tool that trims line ends deletes
/// the break without touching the tags around it; both spellings also split the
/// element across lines, where a line holding only its closing tag would open an
/// HTML block. Writing the `<br>` itself keeps the element on one line and
/// independent of [`BrStyle`](crate::options::BrStyle), which spells *Markdown*
/// breaks.
pub(crate) fn in_raw_html() -> bool {
    RAW_HTML_DEPTH.with(|depth| depth.get() > 0)
}

/// Marks the content walked during its lifetime as being inside an element
/// serialized as HTML, for [`in_raw_html`] to report.
struct RawHtmlGuard;

impl RawHtmlGuard {
    fn enter() -> Self {
        RAW_HTML_DEPTH.with(|depth| depth.set(depth.get() + 1));
        Self
    }
}

impl Drop for RawHtmlGuard {
    fn drop(&mut self) {
        RAW_HTML_DEPTH.with(|depth| depth.set(depth.get() - 1));
    }
}

/// Handles a structural element when it has an allowed parent, or preserves it
/// as HTML in faithful mode when it appears elsewhere.
///
/// Handled child content is framed as a block. When
/// `propagate_children_translation` is true, the returned translation status
/// is false if any child required HTML; callers can use that status to fall
/// back to serializing a larger containing structure.
pub(super) fn handle_or_serialize_by_parent(
    handlers: &dyn Handlers,
    element: &Element,
    tag_names: &[&str],
    propagate_children_translation: bool,
) -> Option<HandlerResult> {
    if handlers.options().translation_mode == TranslationMode::Faithful
        && !parent_tag_name_equals(element.node, tag_names)
    {
        Some(HandlerResult {
            content: serialize_element(handlers, element),
            markdown_translated: false,
        })
    } else {
        let result = handlers.walk_children(element.node);
        let content = result.content.trim_matches('\n');
        Some(HandlerResult {
            content: concat_strings!("\n\n", content, "\n\n"),
            markdown_translated: !propagate_children_translation || result.markdown_translated,
        })
    }
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
    let content = {
        let _guard = RawHtmlGuard::enter();
        handlers.walk_children(element.node).content
    };
    serializer.writer.write_all(content.as_bytes())?;
    serializer.end_elem(name.clone())?;
    String::from_utf8(bytes).map_err(io::Error::other)
}

fn serialize_block_element(element: &Element, options: SerializeOpts) -> io::Result<String> {
    let mut bytes = Vec::new();
    let handle = SerializableHandle::from(element.node.clone());
    serialize(&mut bytes, &handle, options)?;
    let html = String::from_utf8(bytes).map_err(io::Error::other)?;
    Ok(concat_strings!(
        "\n\n",
        escape_html_block_blank_lines(&html),
        "\n\n"
    ))
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

#[macro_export]
macro_rules! serialize_if_faithful {
    ($handlers:expr, $element:expr, $num_attrs_allowed:expr) => {
        if $handlers.options().translation_mode == $crate::options::TranslationMode::Faithful
            && $element.attrs.len() as i64 > $num_attrs_allowed
        {
            return Some($crate::element_handler::HandlerResult {
                content: $crate::element_handler::element_util::serialize_element(
                    $handlers, &$element,
                ),
                markdown_translated: false,
            });
        }
    };
}

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
